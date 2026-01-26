# Claudy: 내가 만든 첫 번째 Rust CLI 도구 이야기

이 문서는 Claudy 프로젝트를 만들면서 배운 것들, 기술적 결정의 이유, 그리고 앞으로 기억해야 할 교훈들을 정리한 것이다. 딱딱한 기술 문서가 아니라, 미래의 나에게 보내는 편지처럼 읽혔으면 좋겠다.

---

## Claudy가 뭐야?

Claude Code는 훌륭한 AI 코딩 어시스턴트지만, 한 가지 불편한 점이 있었다. 설정 파일들이 로컬에만 저장된다는 것. 노트북을 바꾸거나 다른 컴퓨터에서 작업하면 처음부터 다시 설정해야 한다. 마치 게임 세이브 파일이 로컬에만 있는 것처럼.

Claudy는 이 문제를 해결한다. `~/.claude/` 디렉토리에 있는 설정 파일들을 GitHub private repo와 양방향으로 동기화한다. Dropbox가 파일을 클라우드와 동기화하듯이, Claudy는 Claude Code 설정을 GitHub과 동기화한다.

### 왜 GitHub인가?

처음엔 직접 클라우드 스토리지를 구현할까 고민했다. AWS S3? Google Cloud Storage? 하지만 곧 깨달았다 - 개발자들은 이미 GitHub을 매일 쓰고 있다. 새로운 인증 시스템을 만들 필요가 없다. `gh` CLI가 이미 설치되어 있고, 인증도 되어 있다. **기존 인프라를 재활용하는 것이 새로 만드는 것보다 항상 낫다.**

---

## 기술 스택을 선택한 이유

### Rust를 선택한 이유

Python이나 Go로도 만들 수 있었다. 하지만 Rust를 선택한 이유:

- **단일 바이너리 배포**: 사용자가 Python 환경을 설정하거나 의존성을 설치할 필요 없음
- **크로스 플랫폼 빌드**: GitHub Actions에서 macOS, Linux, Windows 바이너리를 쉽게 빌드
- **타입 안전성**: CLI 도구에서 가장 흔한 버그(null 참조, 타입 에러)를 컴파일 타임에 잡음
- **성능**: 파일 동기화는 빨라야 함

처음엔 Rust의 borrow checker와 싸우느라 시간이 걸렸다. 하지만 일단 컴파일되면 런타임 에러가 거의 없다는 걸 경험하고 나니, 그 시간이 아깝지 않았다.

### 핵심 의존성들

```toml
clap = { version = "4", features = ["derive"] }  # CLI 파싱
thiserror = "2"                                   # 에러 타입 정의
similar = "2"                                     # diff 알고리즘
dialoguer = "0.11"                                # 대화형 프롬프트
console = "0.15"                                  # 터미널 색상
```

**clap**: derive 매크로 덕분에 CLI 정의가 매우 선언적이다. 예전에 직접 argparse 로직을 짜던 것에 비하면 천국.

**thiserror**: Rust에서 커스텀 에러 타입 만들 때 필수. `#[derive(Error)]`만 붙이면 `Display`, `Error` trait이 자동 구현된다.

**similar**: diff 라이브러리. Python의 `difflib`처럼 쓸 수 있지만 훨씬 빠르다.

---

## 아키텍처: 어떻게 연결되어 있나

```
┌─────────────────────────────────────────────────────────────┐
│                         main.rs                              │
│              (CLI 진입점 - 교통정리 역할)                      │
└─────────────────────────────────────────────────────────────┘
                              │
          ┌───────────────────┼───────────────────┐
          ▼                   ▼                   ▼
    ┌──────────┐        ┌──────────┐        ┌──────────┐
    │  config  │        │   sync   │        │  github  │
    │  모듈    │◀──────▶│   모듈   │◀──────▶│   모듈   │
    └──────────┘        └──────────┘        └──────────┘
          │                   │                   │
          │                   ▼                   │
          │             ┌──────────┐              │
          │             │whitelist │              │
          └────────────▶│   모듈   │◀─────────────┘
                        └──────────┘
                              │
                              ▼
                        ┌──────────┐
                        │  backup  │
                        │   모듈   │
                        └──────────┘
```

### 모듈별 역할

**main.rs - 교통정리 담당**

```rust
fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Push { dry_run, force } => {
            github::check_auth()?;  // 항상 인증부터 확인
            sync::push(dry_run, force)?;
        }
        // ...
    }
    Ok(())
}
```

`main.rs`는 교차로의 신호등 같다. 명령어를 받아서 적절한 모듈로 라우팅만 한다. 비즈니스 로직은 없다. 이렇게 하면:
- 테스트하기 쉽다 (각 모듈을 독립적으로 테스트)
- 코드를 찾기 쉽다 (push 버그? `sync/push.rs`만 보면 됨)
- 의존성이 명확하다

**github/api.rs - GitHub과의 대화 담당**

여기서 중요한 설계 결정이 있다: **직접 HTTP 요청을 보내지 않고 `gh` CLI를 래핑했다.**

```rust
fn run_gh(args: &[&str]) -> Result<Output> {
    Command::new("gh")
        .args(args)
        .output()
        .map_err(map_gh_error)
}
```

왜 이렇게 했나?

1. **인증 재사용**: `gh auth login`으로 이미 인증된 상태를 그대로 씀
2. **토큰 관리 불필요**: 토큰 저장, 갱신, 보안 처리를 신경 쓸 필요 없음
3. **유지보수 간소화**: GitHub API 변경되면 `gh`가 처리해줌

단점도 있다. `gh`가 설치되어 있어야 하고, 프로세스 fork 오버헤드가 있다. 하지만 이 도구의 사용 패턴(가끔 동기화)을 고려하면 이 트레이드오프는 충분히 합리적이다.

**sync/diff.rs - 두뇌 역할**

diff 계산은 이 프로젝트의 심장이다. push, pull, status 모두 diff에 의존한다.

```rust
pub fn compute_diff(
    client: &GitHubClient,
    matcher: &WhitelistMatcher,
    sync_mode: &SyncMode
) -> Result<Vec<FileDiff>> {
    // 1. 로컬 파일 목록 (화이트리스트 기준)
    let local_files: HashSet<String> = matcher.list_local_files()?.into_iter().collect();

    // 2. 원격 파일 목록
    let remote_files: HashSet<String> = client
        .list_files_recursive("")?
        .into_iter()
        .filter(|f| sync_mode == &SyncMode::Remote || matcher.matches(&f.path))
        .map(|f| f.path)
        .collect();

    // 3. 합집합으로 모든 파일 처리
    let all_files: HashSet<String> = local_files.union(&remote_files).cloned().collect();
    // ...
}
```

집합(Set) 연산을 쓴 이유? 파일이 "로컬에만 있음", "원격에만 있음", "둘 다 있음"을 깔끔하게 판단할 수 있다. 이건 Git의 3-way merge와 비슷한 개념이다.

---

## 핵심 설계 결정과 그 이유

### 1. 화이트리스트 기반 동기화

처음엔 `.claude/` 전체를 동기화하려 했다. 하지만 곧 문제를 발견했다:
- 민감한 정보가 포함된 파일이 있을 수 있음
- 캐시나 임시 파일도 동기화될 수 있음
- 사용자가 무엇이 동기화되는지 명확히 알아야 함

그래서 **명시적 화이트리스트** 방식을 선택했다. "모든 걸 동기화하되 일부 제외"가 아니라 "명시한 것만 동기화"다.

```toml
[whitelist]
paths = [
    "settings.json",
    "CLAUDE.md",
    "commands/**/*.md",
    "skills/**/*.md",
]
```

glob 패턴을 지원해서 유연성도 확보했다.

### 2. 자동 백업 시스템

Pull 작업은 로컬 파일을 덮어쓴다. 실수로 중요한 설정을 날릴 수 있다. 그래서 **모든 pull 전에 자동 백업**을 만든다.

```rust
if !files_to_backup.is_empty() {
    let backup_manager = BackupManager::new();
    let backup_path = backup_manager.create_backup(&files_to_backup)?;
    println!("Backup created: {}", backup_path.display());
}
```

백업은 최대 10개까지 유지하고, 오래된 것부터 자동 삭제한다. 무한정 쌓이면 디스크를 잡아먹으니까.

### 3. 충돌 시 항상 사용자에게 물어보기

Git처럼 자동 병합을 시도할 수도 있었다. 하지만 **설정 파일은 자동 병합하면 안 된다**. JSON 설정이 잘못 병합되면 Claude Code가 깨질 수 있다.

```rust
if diff.status == FileStatus::Modified && !force {
    println!("Conflict: {}", diff.path);
    println!("{}", diff.format_diff());

    let choices = vec!["Overwrite with remote", "Keep local", "Skip"];
    let selection = Select::new()
        .with_prompt("How would you like to proceed?")
        .items(&choices)
        .interact()?;
    // ...
}
```

diff를 보여주고, 선택지를 제공한다. `--force`를 쓰지 않는 한 절대 조용히 덮어쓰지 않는다.

### 4. Dry-run 모드

모든 변경 작업에 `--dry-run` 옵션을 넣었다. 실제로 실행하기 전에 무엇이 변경될지 미리 볼 수 있다.

```bash
claudy push --dry-run
# Files to push:
#   + settings.json
#   M commands/git/commit.md
# [dry-run] No actual push was made
```

실수를 방지하는 가장 좋은 방법은 **되돌리기 쉽게 만드는 것**이고, 그다음은 **미리보기를 제공하는 것**이다.

---

## 디렉토리 구조 해부

```
src/
├── main.rs              # 진입점 (55줄)
├── lib.rs               # 라이브러리 루트
├── error.rs             # 에러 타입 (49줄)
├── update.rs            # 자체 업데이트
├── cli/
│   ├── mod.rs
│   └── commands.rs      # clap CLI 정의
├── config/
│   ├── mod.rs           # config 서브커맨드 핸들러
│   ├── model.rs         # Config, Whitelist 구조체
│   └── loader.rs        # TOML 로드/저장
├── github/
│   ├── mod.rs
│   ├── auth.rs          # gh auth status 확인
│   └── api.rs           # GitHubClient (API 래핑)
├── sync/
│   ├── mod.rs
│   ├── diff.rs          # diff 계산 (핵심!)
│   ├── push.rs          # push 로직
│   ├── pull.rs          # pull 로직
│   └── status.rs        # status 출력
├── whitelist/
│   ├── mod.rs
│   └── matcher.rs       # glob 패턴 매칭
└── backup/
    ├── mod.rs
    └── manager.rs       # 백업 생성/정리
```

**왜 이렇게 나눴나?**

각 디렉토리가 하나의 "관심사(concern)"를 담당한다:
- `github/`: GitHub과의 통신
- `sync/`: 동기화 로직
- `config/`: 설정 관리
- `whitelist/`: 파일 필터링
- `backup/`: 백업 관리

이렇게 하면 "push 버그는 `sync/push.rs`에 있겠구나" 하고 직관적으로 찾을 수 있다.

---

## 배운 교훈들

### 1. Base64의 함정

GitHub API는 파일 내용을 Base64로 인코딩해서 보낸다. 처음에 디코딩이 계속 실패해서 당황했다.

```rust
let encoded_clean = encoded.replace('\n', "");  // 이게 핵심!
let decoded = base64::engine::general_purpose::STANDARD.decode(&encoded_clean)?;
```

**문제**: GitHub API 응답의 Base64 문자열에 줄바꿈(`\n`)이 포함되어 있었다.

**해결**: 디코딩 전에 줄바꿈을 제거했다.

**교훈**: API 응답을 그대로 믿지 마라. 항상 실제 응답을 찍어보고 확인하라.

### 2. SHA가 필요한 이유

GitHub Contents API로 파일을 업데이트할 때 기존 파일의 SHA가 필요하다.

```rust
pub fn put_file(&self, path: &str, content: &str, sha: Option<&str>, message: &str) -> Result<()> {
    // sha가 있으면 업데이트, 없으면 새 파일 생성
    let output = if let Some(s) = sha {
        run_gh_with_extra_args(&base_args, &["-f".to_string(), format!("sha={}", s)])?
    } else {
        run_gh(&base_args)?
    };
    // ...
}
```

**왜?** GitHub은 이걸로 동시 수정 충돌을 방지한다. 내가 파일을 읽은 후 다른 사람이 수정했다면, 내 SHA는 더 이상 유효하지 않다. 일종의 낙관적 잠금(Optimistic Locking)이다.

**교훈**: API를 쓸 때는 "왜 이 파라미터가 필요한가?"를 이해해야 한다.

### 3. 에러 타입 설계의 중요성

처음엔 모든 에러를 `anyhow::Error`로 처리했다. 편하긴 했지만, 에러를 구분해서 처리하기 어려웠다.

```rust
#[derive(Error, Debug)]
pub enum ClaudyError {
    #[error("GitHub CLI (gh) is not installed. Install from https://cli.github.com")]
    GhNotInstalled,

    #[error("GitHub CLI authentication required. Run 'gh auth login'")]
    NotAuthenticated,

    #[error("Repository not configured. Set with 'claudy config repo <owner/repo>'")]
    RepoNotConfigured,
    // ...
}
```

각 에러 케이스가 **해결 방법을 포함**한다. "gh가 없습니다"가 아니라 "gh가 없습니다. https://cli.github.com에서 설치하세요"라고 말한다.

**교훈**: 좋은 에러 메시지는 "무엇이 잘못됐는지"와 "어떻게 고치는지"를 둘 다 알려준다.

### 4. Option과 Result 활용

Rust의 `Option`과 `Result`는 처음엔 번거롭지만, 익숙해지면 버그를 줄여준다.

```rust
pub fn get_file_content(&self, path: &str) -> Result<Option<(String, String)>> {
    // Result<...>: 네트워크 에러 등 실패 가능
    // Option<...>: 파일이 없을 수도 있음 (이건 에러가 아님!)

    if is_not_found(&output) {
        return Ok(None);  // 파일 없음 = 정상 케이스
    }
    // ...
}
```

"파일이 없다"와 "네트워크 오류"를 구분할 수 있다. 둘 다 "실패"지만 의미가 다르다.

### 5. 테스트 가능한 코드 구조

각 모듈이 독립적이어서 단위 테스트가 쉽다.

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_glob_pattern() {
        let matcher = WhitelistMatcher::new(&["commands/**/*.md".to_string()]);
        assert!(matcher.matches("commands/git/commit.md"));
        assert!(!matcher.matches("settings.json"));
    }
}
```

`WhitelistMatcher`가 파일시스템에 직접 의존하지 않아서, 실제 파일 없이도 패턴 매칭 로직을 테스트할 수 있다.

---

## 피해야 할 함정들

### 1. `unwrap()` 남용

```rust
// 나쁜 예
let content = fs::read_to_string(path).unwrap();

// 좋은 예
let content = fs::read_to_string(path)
    .map_err(|e| ClaudyError::FileRead(format!("{}: {}", path, e)))?;
```

`unwrap()`은 패닉을 일으킨다. CLI 도구가 갑자기 죽으면 사용자는 무엇이 잘못됐는지 알 수 없다.

### 2. 경로 처리 실수

```rust
// 나쁜 예
let path = format!("{}/{}", base_dir, filename);

// 좋은 예
let path = base_dir.join(filename);
```

Windows는 `\`를 쓰고 Unix는 `/`를 쓴다. `PathBuf::join()`을 쓰면 자동으로 처리된다.

### 3. 무한 재귀 가능성

```rust
pub fn list_files_recursive(&self, path: &str) -> Result<Vec<RepoContent>> {
    let contents = self.list_files(path)?;

    for item in contents {
        if item.content_type == "dir" {
            // 심볼릭 링크가 순환 참조하면?
            let sub_files = self.list_files_recursive(&item.path)?;
            // ...
        }
    }
}
```

GitHub에선 심볼릭 링크 순환이 없어서 괜찮지만, 로컬 파일시스템을 재귀 탐색할 땐 주의해야 한다. `walkdir` 크레이트는 이런 케이스를 자동으로 처리해준다.

---

## 좋은 엔지니어의 사고방식

이 프로젝트를 하면서 배운 것들:

### "작동하는 가장 간단한 해결책"부터 시작

처음엔 GitHub API를 직접 호출하려 했다. 토큰 관리, 갱신, 저장... 복잡해지기 시작했다. 그때 생각했다: "잠깐, `gh`가 이미 다 해주는 거 아냐?"

**가장 간단한 해결책이 종종 가장 좋은 해결책이다.**

### 경계 조건을 먼저 생각

- 화이트리스트가 비어 있으면?
- 원격 저장소가 설정 안 됐으면?
- 네트워크가 끊겼으면?
- 파일이 없으면?

코드를 짜기 전에 이런 케이스를 먼저 나열하고, 각각 어떻게 처리할지 정했다. 덕분에 나중에 "어, 이 경우는 어떡하지?" 하는 일이 줄었다.

### 사용자 경험 우선

```rust
#[error("Repository not configured. Set with 'claudy config repo <owner/repo>'")]
RepoNotConfigured,
```

에러 메시지에 해결 방법을 넣었다. 터미널에서 "Repository not configured"만 보면 막막하지만, 그 옆에 명령어가 있으면 바로 실행할 수 있다.

### 되돌리기 쉽게 만들기

모든 파괴적 작업(pull로 파일 덮어쓰기) 전에 백업을 만든다. 실수해도 복구할 수 있다. **실수를 불가능하게 만드는 것보다 실수를 복구 가능하게 만드는 게 현실적이다.**

---

## 앞으로 개선하고 싶은 것들

### 더 나은 병합 전략

지금은 "로컬 vs 원격" 중 하나를 선택해야 한다. JSON 설정 같은 경우 필드 단위로 병합할 수 있으면 좋겠다.

### 병렬 파일 처리

지금은 파일을 하나씩 순차적으로 처리한다. 파일이 많으면 느리다. `tokio`나 `rayon`으로 병렬 처리하면 빨라질 것이다.

### 더 나은 오프라인 지원

네트워크가 없을 때의 동작이 아직 어색하다. "마지막 동기화 상태"를 캐시하고, 오프라인에서도 status를 볼 수 있게 하면 좋겠다.

---

## 마무리

Claudy는 작은 프로젝트지만, 많은 것을 배울 수 있었다:

- **Rust 생태계**: clap, thiserror, serde 같은 필수 크레이트들
- **CLI 설계**: 좋은 에러 메시지, dry-run 모드, 대화형 프롬프트
- **API 통합**: 기존 도구(gh CLI)를 래핑하는 실용적 접근
- **안전한 파일 처리**: 백업, 충돌 해결, 사용자 확인

코드는 1,423줄밖에 안 되지만, 그 안에 많은 결정과 트레이드오프가 담겨 있다. 이 문서가 나중에 비슷한 프로젝트를 할 때 도움이 됐으면 좋겠다.

마지막으로, **좋은 도구는 사용자가 실수하기 어렵게 만드는 도구다.** 그게 이 프로젝트에서 가장 중요하게 생각한 원칙이다.
