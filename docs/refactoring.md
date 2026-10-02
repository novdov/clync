# 리팩터링 계획: 출력과 로직 분리 (학습 목적)

## 목표와 작업 방식

- 대상: `~/.claude/` 설정을 GitHub 저장소와 동기화하는 Rust CLI (`clync`)
- 초기 코드는 Claude Code로 작성되었고 출력(`println!`)과 비즈니스 로직이 섞여 있음
- 학습을 겸해 사용자가 **직접** 리팩터링함
- Claude의 역할: 코드 대신 작성보다 리뷰, 방향 제시, 질문 응답. 사용자가 요청하지 않는 한 대규모 코드 변경 금지

## 현재 코드 구조

코드를 확인하여 정리한 사실 (v0.8.1 기준)

- 명령: `push`, `pull`, `diff`, `status`, `config {show, repo, whitelist {list, add, remove, exclude {...}}}`, `self-update`
- 모듈: `cli/`(clap 정의), `config/`(모델, 로더, config 서브커맨드 핸들러), `github/`(`gh` CLI 호출), `sync/`(diff, status, push, pull), `whitelist/`(경로 매칭, 로컬 파일 IO), `backup/`, `update.rs`, `error.rs`
- `lib.rs` + `main.rs` 구조이며 `main.rs`는 이미 인자 파싱과 디스패치만 담당
- 원격 접근은 HTTP 클라이언트가 아니라 `gh api` 프로세스 호출 (`src/github/api.rs`)
- 출력 지점: `println!`/`print!` 89곳. `sync/`, `config/mod.rs`, `update.rs`에 분포하고 `eprintln!`은 없음 (모든 출력이 stdout)
- 경로 의존: `claude_dir()`, `config_path()`가 내부에서 `dirs::home_dir()`를 호출하고 `WhitelistMatcher`, `BackupManager`, `load_config`가 이를 직접 호출함 (숨겨진 전역 의존)

### 이미 일부 분리되어 있는 부분

- `sync/diff.rs`에 도메인 타입이 존재
  - `enum FileStatus { Same, LocalOnly, RemoteOnly, Modified }`
  - `struct FileDiff { path, status, local_content, remote_content, remote_sha }`
  - `compute_diff(client, matcher, sync_mode) -> Result<Vec<FileDiff>>`는 출력 없이 데이터만 반환
- `sync/status.rs`에 `get_status() -> Result<StatusSummary>`가 이미 존재하고 `show_status()`가 이를 호출하여 출력
- 남은 혼재 지점
  - `FileDiff::format_diff()`: 도메인 타입 메서드 안에서 `console::style`로 색상 처리
  - `show_status()`: 설정 검증(repo 미설정, 빈 whitelist)과 출력이 섞여 있고 설정을 두 번 로드 (`show_status`와 `get_status` 각각)
  - `push::execute`, `pull::execute`: 대상 선정, 확인 프롬프트, 백업, 네트워크 호출, 진행 출력이 한 함수에 있음
  - `config/mod.rs`: 설정 변경과 결과 메시지 출력이 함께 있음

## 논의 내용 대비 수정 사항

코드를 보지 않은 상태에서 가정한 내용 중 실제와 다른 부분

- **파일 트리 출력 기능은 없음.** 트리 문자(`├──`)를 쓰는 곳이 없음. 실제 출력은 status의 그룹별 목록(`+`/`-`/`M`), diff의 라인 단위 전체 diff(hunk 없이 변경되지 않은 줄까지 dim 처리하여 모두 출력), config 목록
- **`Conflict` 상태는 현재 구조로 판정 불가.** 마지막 동기화 시점의 기준(base) 정보를 저장하지 않으므로 "양쪽이 모두 바뀌었다"를 알 수 없음. `Modified`는 "내용이 다르다"만 의미함. `Conflict`를 도입하려면 base SHA 저장이 먼저 필요하며 이번 리팩터링 범위 밖으로 둠
- **도메인 타입은 이미 있지만 형태를 바꿔야 함.** `FileStatus`/`FileDiff`가 존재하지만 상태별로 유효한 필드가 다른 구조를 `Option` 필드로 표현하고 있음. variant별 데이터를 담는 enum으로 재설계함 (설계 1 참조)
- **모듈 구성안(`main.rs`/`sync.rs`/`render.rs`)은 현재 구조보다 단순함.** 이미 기능별 모듈이 나뉘어 있으므로 합치지 않고 `render` 모듈만 추가하는 방향이 변경량이 적음
- **에러 처리는 `anyhow`가 아니라 `thiserror` 기반 `ClyncError`를 사용 중.** 전환 여부는 "미결정 사항" 참조
- **push는 원격 삭제도 수행함.** `RemoteOnly` 파일을 원격에서 삭제(로컬을 원격에 미러링)하지만 pull은 `LocalOnly` 파일을 로컬에서 삭제하지 않음. plan 설계 시 이 비대칭을 의도로 유지할지 확인 필요
- **pull은 파일 단위 대화형 선택이 있음.** `Modified` 파일마다 `Select`로 묻기 때문에 "plan 출력 후 일괄 확인"만으로는 표현되지 않음 (아래 설계 2 참조)
  - 선택지는 "Overwrite with remote", "Keep local", "Skip" 세 개지만 뒤의 둘은 모두 `continue`로 동작이 같음 (`src/sync/pull.rs:86-95`). 실질적인 선택은 "덮어쓰기"와 "건너뛰기" 두 가지
- **백업은 충돌 선택 전에 수행됨.** 이후 건너뛴 파일도 백업에 포함됨 (`src/sync/pull.rs:55-69`)

## 설계 방향

### 1. 로직은 데이터를 반환하고 출력은 render 계층에서 처리

#### `FileDiff`를 불가능한 상태가 없는 타입으로 재설계

- 현재 `FileDiff`는 `status`와 `Option` 필드 3개(`local_content`, `remote_content`, `remote_sha`)를 가짐. 각 필드가 채워져 있는지는 `status`에 따라 달라지지만 타입은 이를 보장하지 않음
- 그 결과 일어날 수 없는 상태를 런타임에 검사하는 코드가 존재함
  - `Cannot read local file` (`src/sync/push.rs:96`): 업로드 대상인데 로컬 내용이 없는 경우
  - `Missing remote SHA` (`src/sync/push.rs:116`): 원격 삭제 대상인데 SHA가 없는 경우
- variant마다 필요한 데이터만 담는 enum으로 바꾸면 위 검사가 사라지고 `match`가 모든 경우의 처리를 강제함
  ```rust
  struct FileDiff {
      path: String,
      kind: DiffKind,
  }

  enum DiffKind {
      Same,
      LocalOnly { content: String },
      RemoteOnly { sha: String },
      Modified { local_content: String, remote_sha: String },
  }
  ```
  - 모든 variant에 공통인 `path`는 바깥 struct에 둠. `path`를 각 variant 안에 넣으면 경로를 읽을 때마다 `match`가 필요함
  - 기존 `FileStatus`는 `DiffKind`로 대체됨
- 원격 내용(`remote_content`)은 `FileDiff`에 담지 않음
  - 현재 `compute_diff`는 `Modified`/`RemoteOnly` 파일마다 원격 내용을 받아오므로 status에서도 파일 수만큼 불필요한 `gh api` 호출이 발생함
  - 원격 내용이 필요한 곳은 diff 명령의 본문 출력, pull의 충돌 프롬프트에 보여주는 diff(`src/sync/pull.rs:75`), pull의 다운로드뿐임. 각 명령이 필요한 시점에 `client.get_file_content`로 가져옴
  - `get_file_content`는 `Option`을 반환하며 비교 시점과 조회 시점 사이에 원격 파일이 삭제되면 `None`이 될 수 있음. 따라서 pull의 `Cannot read remote file` (`src/sync/pull.rs:99`)은 불가능한 상태가 아니라 정당한 런타임 에러로 남음
- push와 pull이 같은 `FileDiff`를 쓰므로 variant 이름은 방향 중립적인 현재 이름(`LocalOnly`, `RemoteOnly`)을 유지함

#### render 분리

- `FileDiff`, `StatusSummary`의 출력 코드를 `src/render.rs`(또는 `src/render/`)로 이동
  - 예: `fn render_status(summary: &StatusSummary, out: &mut impl Write) -> io::Result<()>`
  - `FileDiff::format_diff()`는 `render_diff(&FileDiff, remote_content: Option<&str>, &mut impl Write)`처럼 원격 내용을 별도로 받는 함수로 이동하여 `sync/diff.rs`에서 `console` 의존 제거
- 색상(`console::style`)과 기호(`+`, `-`, `M`, `✓`)는 render 계층에서만 사용
- 도메인 타입에 `Display`를 구현하지 않음. 표시 문자열이 필요하면 render 함수로 만듦 (예: `render::action_label(&action)`)
- `show_status()`의 설정 검증 분기(repo 미설정, 빈 whitelist)는 상태를 표현하는 타입으로 반환하거나 에러로 올리는 것 중 선택 필요. 현재는 둘 다 정상 종료(exit 0)하며 안내 메시지만 출력함
- status/diff/push/pull 4곳에 반복되는 준비 코드(설정 로드, repo 확인, 빈 whitelist 검사, `GitHubClient`/`WhitelistMatcher` 생성)는 하나의 함수로 추출 가능

### 2. 동기화는 plan / execute 분리

- push와 pull은 같은 `FileDiff`를 반대 방향으로 해석하므로 plan 함수를 나눔
  - `plan_push(diffs: &[FileDiff]) -> Vec<Action>`: `LocalOnly`/`Modified`는 업로드, `RemoteOnly`는 원격 삭제
  - `plan_pull(diffs: &[FileDiff]) -> Vec<Action>`: `RemoteOnly`/`Modified`는 다운로드
  - 두 함수 모두 순수 함수
- Action이 데이터를 담는 방식은 둘 중 선택 ("미결정 사항" 참조)
  - `Action<'a>`가 `&'a FileDiff`를 빌림: 복제가 없고 lifetime을 다루는 연습이 됨
  - Action이 path, content, sha를 소유: 단순하지만 내용 복제가 발생함
- dry-run은 plan을 render한 뒤 종료하는 것으로 표현
- 확인 프롬프트 처리
  - push: plan 출력 후 일괄 `Confirm` 한 번이므로 그대로 적용 가능
  - pull: `Modified`에서 나온 Action마다 사용자 선택을 받아 plan을 걸러내는 단계가 필요. 선택 로직을 인자로 받으면 프롬프트 없이 테스트 가능
    ```rust
    enum Choice { Overwrite, Skip }

    fn resolve<'a>(
        actions: Vec<Action<'a>>,
        mut choose: impl FnMut(&Action<'a>) -> Choice,
    ) -> Vec<Action<'a>>
    ```
- 실행 순서: plan → resolve → 백업 → execute. 백업 대상은 resolve 이후 남은 Action에서 정함
- 진행 출력: 현재는 파일마다 `→ path ... done`을 출력함. Reporter trait 없이 이를 유지하려면 `execute`가 단일 Action을 처리하고 호출부가 반복하면서 앞뒤로 `eprint!` 하는 형태가 단순함
  ```rust
  for action in &actions {
      eprint!("  {} ... ", render::action_label(action));
      execute(&client, &matcher, action)?;
      eprintln!("done");
  }
  ```
  - stderr는 버퍼링하지 않으므로 `eprint!`는 네트워크 호출 전에 바로 표시됨 (현재 stdout `print!`는 줄바꿈 전까지 표시되지 않음)

### 3. 진행 상황 이벤트(콜백/Reporter trait)는 도입하지 않음

- 파일당 `gh api` 프로세스를 1회 이상 실행하므로 파일 수에 비례하여 수 초가 걸릴 수 있음. 위의 호출부 반복 방식으로 파일 단위 진행 표시는 유지됨

### 4. 경로는 주입하고 입력은 경계에서 파싱

- `Paths { claude_dir, config_dir, backup_dir }` 같은 구조체를 `main`에서 한 번 만들어 `WhitelistMatcher`, `BackupManager`, 설정 로더에 넘김. 내부에서 `claude_dir()`을 직접 호출하지 않음
  - 단위 테스트에서 tempdir을 직접 주입할 수 있어 `HOME` 교체가 필요 없어짐
  - 현재 cwd 기준인 백업 위치도 `backup_dir` 한 곳에서 결정하므로 위치 변경이 쉬워짐
- "검증하지 말고 파싱하라": 문자열을 로직 깊은 곳까지 들고 가지 않고 진입 시점에 도메인 타입으로 변환
  - whitelist/exclude 패턴: 잘못된 glob을 조용히 버리지 않고 입력 시점(clap `value_parser`)이나 설정 로드 시점에 에러로 처리
  - repo: `owner/repo` 형식을 newtype으로 파싱. `config.toml`을 역직렬화할 때도 검증되도록 `#[serde(try_from = "String")]` 사용

### 5. CLI 출력 관례

- 결과(status 목록, diff 본문, config 목록)는 stdout, 진행/안내/경고는 stderr
- 대량 출력은 `BufWriter::new(io::stdout().lock())` + `writeln!` (Rust 1.61부터 `lock()`이 `StdoutLock<'static>`을 반환하므로 임시 변수 불필요)
- `writeln!`으로 바꾸면 다음을 함께 처리
  - `BufWriter`는 drop 시 flush 에러를 버리므로 마지막에 `out.flush()?`를 명시적으로 호출
  - `clync status | head`처럼 파이프가 닫히면 `ErrorKind::BrokenPipe`가 반환됨. `?`로 `main`까지 전파만 하면 panic은 사라지지만 에러 메시지가 출력되고 exit 1로 끝남. 조용히 끝내려면 `BrokenPipe`를 정상 종료로 처리
- `console::style()`은 stdout의 TTY 여부로 색상을 판정함. stderr에 출력하는 문자열은 `style(..).for_stderr()`를 사용해야 `2>log` 같은 리다이렉트에서 ANSI 코드가 섞이지 않음
- 현재 stdout만 사용하므로 어떤 메시지를 stderr로 옮길지 각 render 분리 단위(5-2, 5-4, 5-5, 6-2)에서 결정

### 6. 에러와 종료 코드

- 현재: `ClyncError`(thiserror) + `crate::Result`. 대부분의 variant가 `String`을 담아 원인 에러(source)가 사라짐 (예: `GitHubApi(String)`, `FileRead(String)`)
- **현재 동작상 문제**: `main`이 `Result<(), ClyncError>`를 반환하면 Rust는 에러를 `Debug` 형식으로 출력함. 따라서 `#[error("...")]`에 작성한 안내 문구 대신 `Error: NotAuthenticated` 같은 variant 이름이 출력됨
- `UserCancelled`가 에러로 처리되어 취소 시에도 `Error: UserCancelled`가 출력되고 exit 1로 종료됨
- `main`은 `ExitCode`를 반환하고 로직은 `run() -> Result`로 분리. 에러 출력 형식, 취소 시 종료 코드, `BrokenPipe` 처리를 한 곳에서 결정
  ```rust
  fn main() -> ExitCode {
      let cli = Cli::parse();
      match run(cli) {
          Ok(()) => ExitCode::SUCCESS,
          Err(e) => {
              eprintln!("error: {e:#}");
              ExitCode::FAILURE
          }
      }
  }
  ```
  - `{e:#}`가 원인 체인(`a: b: c`)을 출력하는 것은 `anyhow` 에러의 동작. `ClyncError`에서는 일반 `Display`와 같음
- 취소는 에러가 아니라 결과 값으로 반환하는 편이 자연스러움 (예: `run`이 완료/취소를 나타내는 값을 반환)

## 작업 단위

원칙

- 각 단위는 하나의 커밋(또는 PR)으로 완료하며 완료 시점에 `cargo build`, `cargo test`, `cargo clippy`가 통과해야 함
- 동작을 바꾸지 않는 단위와 동작을 바꾸는 단위를 섞지 않음. 동작이 바뀌는 단위는 "동작 변화"에 명시함
- 앞 단위에 의존하는 경우 "선행"에 명시함. 선행 표시가 없는 단위는 순서를 바꿔도 됨

### 0단계: 안전망

**0-1. 현재 출력 기록**

- `gh` 의존 명령은 자동 테스트 비용이 크므로 실제 저장소 기준 출력을 파일로 기록해 두고 단위 완료 시마다 비교
- 대상: `clync status`, `clync diff`, `clync push --dry-run`, `clync pull --dry-run`, `clync config show`
- 에러 출력도 기록하되 실제 `config.toml`을 바꾸지 않도록 임시 `HOME`에서 실행 (예: `HOME=$(mktemp -d) clync diff`로 repo 미설정 에러 확인)
- 비교 시 색상 코드를 피하려면 파이프로 받아서 기록 (`clync status | cat > before-status.txt`)
- 완료 기준: 기록 파일 생성. 저장소에는 커밋하지 않음

**0-2. config 서브커맨드 통합 테스트**

- `assert_cmd`를 dev-dependency로 추가 (`tempfile`은 이미 있음)
- `assert_cmd`, `tempfile`로 `HOME`을 tempdir로 바꾸고 `config repo`, `config whitelist add/remove/list`, `config show` 출력과 `config.toml` 내용을 검증
- `gh` 의존이 없어 통합 테스트 비용이 낮은 유일한 명령군
- 완료 기준: 현재 동작 그대로 테스트 통과 (`Claudy Configuration` 표기 오류도 현재 동작으로 고정)

### 1단계: 독립적인 소규모 정리

구조 변경 없이 할 수 있는 정리. Rust 관용구 연습 단위

**1-1. `main`을 `run()` + `ExitCode`로 변경**

- `main`은 `run()`의 결과를 match하여 에러를 `Display`(`{e}`)로 stderr에 출력하고 `ExitCode::FAILURE` 반환
- 동작 변화: 에러 메시지가 `Error: NotAuthenticated` 대신 `#[error]` 문구로 출력됨

**1-2. `gh` 호출 헬퍼 정리**

- `check_gh_error`를 `Option<ClyncError>` 대신 `Result<()>` 반환으로 바꾸고 `unwrap()` 제거
- `run_gh`와 `run_gh_with_extra_args`를 하나로 합침
- `auth.rs`의 에러 매핑을 `map_gh_error` 재사용으로 변경
- 동작 변화 없음

**1-3. 할당과 표기 정리**

- `contains(&path.to_string())`를 `iter().any(|p| p == path)`로 변경
- 동작 변화: `config show` 제목을 `Clync Configuration`으로 수정 (0-2 테스트 기대값도 함께 수정)

**1-4. 미사용 코드 제거**

- "점검 대상 패턴"의 미사용 코드 목록을 사용 여부 확인 후 제거
- lib crate의 `pub` 항목이라 컴파일러 경고가 없으므로 grep으로 호출부를 확인
- 동작 변화 없음

### 2단계: 경로 주입

**2-1. `Paths` 구조체 도입**

- `Paths { claude_dir, config_dir, backup_dir }`를 `main`(또는 `run`)에서 한 번 생성하여 `load_config`, `save_config`, `WhitelistMatcher`, `BackupManager`에 넘김
- 각 모듈 내부의 `claude_dir()`, `config_path()` 직접 호출 제거
- 백업 위치(cwd 기준)는 이 단위에서 바꾸지 않음. `backup_dir`에 현재 위치(`env::current_dir()` 기준)를 그대로 담아 동작을 유지
- 동작 변화 없음

**2-2. 로컬 파일 IO 단위 테스트**

- 선행: 2-1
- tempdir을 주입하여 `WhitelistMatcher::list_local_files`, `read_local_file_with_sha`, `write_local_file` 테스트

### 3단계: 입력 파싱

**3-1. glob 패턴 검증**

- `WhitelistMatcher::new`가 잘못된 패턴을 버리지 않고 `Result`로 에러를 반환
- `is_excluded`가 호출마다 `Pattern::new(e)`로 패턴을 다시 컴파일하므로 (`src/whitelist/matcher.rs:44`) `new`에서 한 번만 컴파일하도록 변경
- exclude 항목은 glob이면서 디렉토리 경로로도 쓰이므로 glob으로 파싱되지 않는 경로(예: `[`를 포함한 경로)를 거부할지 이 단위에서 결정
- `config whitelist add`, `config whitelist exclude add`에서 clap `value_parser`로 입력 시점에 검증
- 동작 변화: 잘못된 패턴이 에러로 보고됨

**3-2. repo newtype**

- `owner/repo` 형식을 파싱하는 newtype을 만들고 `config repo` 인자와 `Config.repo`에 적용
- `config.toml` 역직렬화 시에도 검증되도록 `#[serde(try_from = "String")]` 사용
- 동작 변화: 잘못된 형식의 repo 입력이 에러로 보고됨

### 4단계: diff 도메인 재설계

**4-1. `compute_diff`에서 상태 분류를 순수 함수로 분리**

- `compute_diff`를 "로컬/원격 목록 수집"과 "path별 SHA 비교로 상태 분류"로 나눔. 분류 함수는 `gh` 없이 단위 테스트
- 이 단위에서 `get().cloned()`를 `remove()`로, `match &local_result`를 값 match로 바꿔 clone 제거
- `FileDiff` 형태는 아직 유지
- 동작 변화 없음

**4-2. `FileDiff`에서 원격 내용 제거**

- 선행: 4-1
- `remote_content` 필드를 제거하고 diff 명령(`Modified` 본문 출력), pull의 충돌 프롬프트(diff 표시), pull의 다운로드가 필요한 시점에 `client.get_file_content`를 호출
- 동작 변화: 출력은 같고 status/push에서 불필요한 `gh api` 호출이 사라짐

**4-3. `FileStatus` + `Option` 필드를 `DiffKind` enum으로 변경**

- 선행: 4-2
- `struct FileDiff { path, kind: DiffKind }`로 변경 (설계 1 참조)
- push의 런타임 검사(`Cannot read local file`, `Missing remote SHA`) 제거. pull의 `Cannot read remote file`은 원격 조회 결과가 `None`인 정당한 에러로 남김
- 동작 변화 없음

### 5단계: 읽기 전용 명령의 render 분리

**5-1. 명령 공통 준비 코드 추출**

- 선행: 2-1
- status/diff/push/pull에 반복되는 설정 로드, repo 확인, 빈 whitelist 검사, `GitHubClient`/`WhitelistMatcher` 생성을 하나의 함수로 추출
- status는 빈 whitelist에서 에러 대신 안내 후 exit 0으로 끝나므로 이 차이를 유지할 수 있는 형태로 추출 (예: 검사 없이 준비만 하는 함수 + 검사 함수)
- 동작 변화 없음

**5-2. render 모듈 생성과 diff 명령 분리**

- 선행: 4-3
- `src/render.rs` 생성. `FileDiff::format_diff()`를 `render_diff`로 옮기고 `push_str(&format!(..))`를 `write!`로 변경
- diff 명령은 `BufWriter::new(io::stdout().lock())`로 출력하고 마지막에 `flush()?`
- `Vec<u8>`을 넘기는 render 단위 테스트 추가. 색상은 render 함수 인자로 받아 `console`의 `Style::force_styling(bool)`로 호출 단위에서 제어
- 동작 변화 없음

**5-3. `BrokenPipe` 처리**

- 선행: 1-1, 5-2
- `run()`에서 반환된 에러가 `BrokenPipe`이면 메시지 없이 정상 종료
- 동작 변화: `clync diff | head`에서 에러가 출력되지 않음

**5-4. status 명령 분리**

- 선행: 5-1
- `render_status` 추가. `show_status`가 설정을 두 번 로드하지 않도록 정리
- 동작 변화 없음 (0-1 기록과 비교)

**5-5. config 서브커맨드 분리**

- `config show`, `whitelist list`, `exclude list`를 render 함수로 이동
- add/remove의 결과 메시지를 stdout에 둘지 stderr로 옮길지 이 단위에서 결정
- 0-2 통합 테스트로 회귀 확인

### 6단계: push의 plan / execute 분리

**6-1. `plan_push` 추출**

- 선행: 4-3, "미결정 사항"의 Action 소유 방식 결정
- `FileDiff` 목록에서 업로드/원격 삭제 Action 목록을 만드는 순수 함수와 단위 테스트
- 출력, 확인 프롬프트, 실행 순서는 그대로 둠
- 동작 변화 없음

**6-2. 계획 출력을 render로 이동**

- 선행: 5-2, 6-1
- "Files to push" 목록과 dry-run 안내를 render 함수로 이동
- 동작 변화 없음

**6-3. 단일 Action 실행 함수와 진행 출력**

- 선행: 6-1
- `execute(&client, &matcher, &action)`이 Action 하나를 처리하고 호출부가 반복하며 `eprint!`/`eprintln!`으로 진행 표시. `style(..).for_stderr()` 사용
- 동작 변화: 진행 표시가 stderr로 이동하고 네트워크 호출 전에 바로 표시됨

### 7단계: pull의 plan / resolve / execute 분리

**7-1. `plan_pull` 추출**

- 선행: 4-3
- 6-1과 같은 방식. 동작 변화 없음

**7-2. `resolve` 추출**

- 선행: 7-1
- `resolve(actions, choose)` 순수 함수와 단위 테스트. `Choice { Overwrite, Skip }`
- 실제 실행에서 `choose` 클로저는 원격 내용을 가져와 diff를 보여준 뒤 프롬프트를 띄움. 네트워크 호출은 클로저 안에 있으므로 `resolve` 자체는 순수 함수로 유지됨
- `--force`일 때는 항상 `Choice::Overwrite`를 반환하는 클로저를 넘김
- 프롬프트 UI의 선택지 3개는 유지하고 "Keep local"과 "Skip"을 모두 `Choice::Skip`으로 대응시켜 동작 유지. 선택지를 줄이는 것은 별도 결정
- 동작 변화 없음

**7-3. 백업 시점 변경**

- 선행: 7-2
- plan → resolve → 백업 → execute 순서로 변경
- 동작 변화: 건너뛴 파일은 백업에 포함되지 않음

**7-4. 단일 Action 실행 함수와 진행 출력**

- 선행: 6-3, 7-2
- 6-3과 같은 방식으로 적용

### 8단계: 에러 정리

"미결정 사항"의 에러 타입 결정 후 진행

**8-1. 에러 타입 결정 반영**

- `anyhow` 전환 시: `ClyncError`의 `String` variant를 `with_context`로 대체하고 `main`의 출력을 `{e:#}`로 변경
- `thiserror` 유지 시: `String` variant를 `#[source]`를 가진 variant로 변경

**8-2. 취소를 결과 값으로 변경**

- `UserCancelled`를 에러가 아닌 반환 값으로 바꾸고 종료 코드를 결정
- 프롬프트의 `interact().unwrap_or(..)`는 에러로 전파
- 동작 변화: 취소 시 출력과 종료 코드, 비 TTY 환경에서의 프롬프트 동작

### 9단계: 결정이 필요한 후속 정리

각 항목은 "미결정 사항" 결정 후 개별 단위로 진행

- 백업 위치를 `~/.clync/backup`으로 이동할지
- `Sync mode: {:?}` 출력을 사용자용 문자열로 변경
- push의 원격 삭제와 pull의 로컬 미삭제 비대칭

## 학습 개념

작업 단위별로 필요한 Rust 개념. 단계 순서대로 진행하면 기초 개념에서 lifetime, 클로저 순으로 익히게 됨

### 단계별 개념

- **0단계 (테스트)**
  - 단위 테스트(`#[cfg(test)] mod tests`)와 통합 테스트(`tests/` 디렉토리)의 차이. 통합 테스트는 crate를 외부 사용자 입장에서 호출함
- **1단계 (main, gh 헬퍼, 표기 정리)**
  - `Display`와 `Debug`의 차이, 포맷 지정자 `{}` / `{:?}` / `{:#}`
  - `std::process::ExitCode`, `main`의 반환 타입을 결정하는 `Termination` trait
  - `Option`과 `Result` 사이의 변환(`ok_or`, `ok_or_else`, `map_err`), `?` 연산자와 `From` 변환(`#[from]`)
  - `String`과 `&str`의 관계: 두 타입 사이의 `PartialEq` 구현, deref coercion
- **2단계 (Paths 주입)**
  - 구조체 필드를 소유할지(`PathBuf`) 빌릴지(`&Path`) 결정, 함수 인자로 `&Paths` 전달
- **3단계 (입력 파싱)**
  - newtype 패턴
  - `FromStr`과 `TryFrom` trait. clap `value_parser`와 serde `try_from`이 이 trait을 사용함
  - 생성자가 `Result`를 반환하는 패턴(`fn new(..) -> Result<Self>`)
- **4단계 (FileDiff 재설계)**
  - 데이터를 가진 enum, `match`의 exhaustiveness(모든 경우 처리 강제)와 구조 분해
  - match ergonomics: `match &x`와 `match x`의 차이. 참조로 match하면 바인딩이 참조가 되어 clone이 필요해짐
  - 컬렉션에서 값을 꺼낼 때의 소유권 이동: `HashMap::remove`, `into_iter()`
- **5단계 (render 분리)**
  - `std::io::Write` trait. `Vec<u8>`, `Stdout`, `BufWriter`가 모두 구현함
  - 인자 위치의 `impl Trait`(정적 디스패치)와 `&mut dyn Write`(동적 디스패치)의 차이
  - `io::Write`와 `fmt::Write`의 차이: `String`에 `write!`하려면 `fmt::Write`가 필요함
  - `io::ErrorKind`로 에러 종류 구분(`BrokenPipe`)
- **6~7단계 (plan / resolve / execute)**
  - lifetime: `Action<'a>`가 `&'a FileDiff`를 빌리면 `Vec<FileDiff>`가 Action보다 오래 살아야 함
  - 클로저와 `Fn` / `FnMut` / `FnOnce`의 차이: `choose`가 `GitHubClient`를 캡처하거나 상태를 바꾸면 `FnMut`이 필요함
  - iterator 어댑터(`filter`, `filter_map`, `partition`)로 Action 목록 생성과 필터링
- **8단계 (에러)**
  - `std::error::Error` trait과 `source()`를 통한 원인 체인
  - thiserror의 `#[source]`, anyhow의 `Context` trait(`with_context`)과 `downcast_ref`

### 함정

- `Action<'a>` 방식에서 diffs를 함수 안에서 만들고 Action만 반환하면 `returns a value referencing data owned by the current function` 에러가 발생함. diffs를 소유하는 호출부가 plan과 execute를 모두 호출하는 구조여야 함
- `BufWriter`에 쓴 뒤 `flush()`를 빠뜨리면 `Vec<u8>`을 쓰는 테스트에서는 드러나지 않고 실제 stdout에서만 출력 누락이나 에러 은폐가 발생할 수 있음

### 참고 자료

- The Rust Programming Language(공식 책)
  - 12장 "An I/O Project": `run()`과 lib 분리, stderr 출력을 다루며 1단계, 5단계와 구조가 거의 같음
  - 6장(enum), 9장(에러), 10장(제네릭, trait, lifetime), 11장(테스트), 13장(클로저, iterator)

## 테스트 전략

외부 의존이 있어 `assert_cmd` 통합 테스트만으로는 비용이 큼

- 외부 의존
  - 원격: `gh` CLI 프로세스 호출 (`src/github/api.rs`, `src/github/auth.rs`)
  - 로컬: `dirs::home_dir()` 기준 `~/.claude`, `~/.clync/config.toml`
- 통합 테스트로 고정하려면 `HOME`을 tempdir로 바꾸고 `PATH` 앞에 가짜 `gh` 스크립트를 두어야 함. config 서브커맨드는 `HOME`만 바꾸면 되므로 비교적 쉬움
- 동기화 로직은 순수 함수 단위 테스트가 더 효율적
  - `compute_diff`를 "로컬/원격 목록 수집"과 "path별 SHA 비교로 상태 분류"로 나누면 분류 로직은 `gh` 없이 테스트 가능
  - `plan_push`, `plan_pull`, `resolve`도 순수 함수이므로 단위 테스트 대상
  - 로컬 파일 IO가 필요한 테스트는 설계 4의 `Paths` 주입으로 tempdir 사용
- 출력 고정은 render 함수에 `Vec<u8>`을 넘겨 문자열 비교 (필요하면 insta 스냅샷)
  - 색상을 끄는 `console::set_colors_enabled(false)`는 전역 상태를 바꾸고 `cargo test`는 테스트를 병렬 실행하므로 다른 테스트에 영향을 줄 수 있음. stderr용 `set_colors_enabled_stderr`도 별도로 존재함. render 함수가 색상 사용 여부를 인자로 받아 `Style::force_styling(bool)`로 적용하면 전역 상태 없이 테스트 가능

## 점검 대상 패턴

리팩터링 중 함께 정리할 후보

- 불필요한 네트워크 호출: `compute_diff`가 `Modified`/`RemoteOnly` 파일마다 원격 내용을 받아옴 (`src/sync/diff.rs:98`). status는 내용이 필요 없으므로 파일 수만큼 불필요한 `gh api` 호출이 발생함. 설계 1의 `FileDiff` 재설계에서 원격 내용을 제외하면 해결됨
- 진행 표시가 보이지 않음: `print!("  → path ... ")` 후 flush 없이 네트워크 호출 (`src/sync/push.rs:108`, `src/sync/pull.rs:103`). stdout은 `LineWriter`라서 `done`과 함께 한 번에 출력됨
- 잘못된 입력을 조용히 무시: `WhitelistMatcher::new`가 `filter_map(|p| Pattern::new(p).ok())`로 잘못된 glob을 버림 (`src/whitelist/matcher.rs:27`). `config whitelist add`로 잘못된 패턴을 추가해도 경고 없이 동기화에서 빠짐
- `unwrap`
  - `check_gh_error(&output).unwrap()` (`src/github/api.rs:119`, `:159`): 성공 여부를 이미 확인했으므로 패닉은 나지 않지만 `Option<ClyncError>` 반환 설계가 어색함. `Result<()>` 반환으로 바꾸면 `?` 사용 가능
  - `interact().unwrap_or(...)` (`src/sync/push.rs:87`, `src/sync/pull.rs:84`): 비 TTY 환경 등 프롬프트 실패를 조용히 "취소/건너뛰기"로 처리함
- 불필요한 할당/복제
  - `contains(&path.to_string())` (`src/config/mod.rs:84`, `:128`): `iter().any(|p| p == path)`로 할당 없이 비교 가능
  - `compute_diff`의 `remote_files_map.get(&path).cloned()`: `remove(&path)`로 SHA를 소유권째 가져오면 clone 불필요
  - `compute_diff`의 `content.clone()`: `match &local_result` 대신 값으로 match하면 clone 불필요
- 문자열 누적: `format_diff`의 `output.push_str(&format!(...))` (`src/sync/diff.rs:51`)는 clippy `format_push_string` 대상. render로 옮기며 `write!(out, ...)`로 바꾸면 해결됨
- 중복 코드
  - `gh` 실행 에러 매핑이 `src/github/api.rs:10`과 `src/github/auth.rs:10`에 중복
  - `run_gh`와 `run_gh_with_extra_args`가 거의 동일
- 표기 오류: `config show` 제목이 이전 이름인 `Claudy Configuration`으로 출력됨 (`src/config/mod.rs:13`)
- `Debug` 출력을 사용자 메시지로 사용: `Sync mode: {:?}` (`src/sync/status.rs:70`, `src/config/mod.rs:22`)
- 백업 위치: `BackupManager`가 `~/.clync`가 아니라 현재 작업 디렉토리 아래 `.clync/backup`에 백업함 (`src/backup/manager.rs:18`). 의도인지 확인 필요
- 사용되지 않는 것으로 보이는 코드 (삭제 전 확인): `GitHubClient::list_files`, `list_files_recursive`, `WhitelistMatcher::read_local_file`, `delete_local_file`, `BackupManager::list_backups`, `sync::get_status` re-export, `ClyncError`의 `ConfigNotFound`/`FileNotFound`/`Conflict`
  - lib crate의 `pub` 항목이라 `dead_code` 경고가 발생하지 않으므로 수동으로 확인해야 함

## 미결정 사항

- 에러 타입: `anyhow` 전환 vs `thiserror` 유지
  - `anyhow`: `with_context`로 맥락 부여가 쉽고 `{e:#}`로 원인 체인을 출력할 수 있음. 현재 에러 종류별로 분기하는 호출부가 사실상 없으므로 전환 부담이 적음. `lib.rs`가 바이너리 전용 내부 라이브러리이므로 lib crate에서 `anyhow`를 쓰는 것도 문제되지 않음
  - `thiserror` 유지: `run()` + `ExitCode` 구조로 바꾸면 현재 출력 문제는 해결됨. `String` variant를 `#[source]`를 가진 variant로 바꾸는 작업이 별도로 필요
- 사용자 취소의 종료 코드
- status에서 repo 미설정, 빈 whitelist를 에러(exit 1)로 바꿀지 현재처럼 안내 후 exit 0을 유지할지
- push의 원격 삭제와 pull의 로컬 미삭제 비대칭을 유지할지
- Action이 `FileDiff`를 빌릴지 데이터를 소유할지
- Action 이름의 기준: 원격 기준(`Upload`/`Download`/`DeleteRemote`)으로 통일할지 다른 기준을 쓸지
- render 모듈을 단일 파일로 둘지 명령별 하위 모듈로 나눌지
