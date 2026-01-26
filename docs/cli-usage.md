# Claudy CLI 사용법

Claude Code 설정을 GitHub 저장소와 동기화하는 CLI 도구.

## 설치

```bash
cargo install --path .
```

## 사전 요구사항

- [GitHub CLI (gh)](https://cli.github.com) 설치 및 인증
  ```bash
  gh auth login
  ```

## 명령어

### claudy push

로컬 설정을 원격 저장소로 푸시합니다.

```bash
claudy push [OPTIONS]
```

**옵션:**
- `--dry-run` - 실제 변경 없이 예상 결과만 표시
- `--force` - 확인 프롬프트 생략

### claudy pull

원격 저장소에서 설정을 로컬로 풀합니다.

```bash
claudy pull [OPTIONS]
```

**옵션:**
- `--dry-run` - 실제 변경 없이 예상 결과만 표시
- `--force` - 확인 프롬프트 생략

**참고:** 풀 시 기존 파일은 자동으로 백업됩니다.

### claudy diff

로컬과 원격의 차이점을 표시합니다.

```bash
claudy diff
```

### claudy status

동기화 상태를 확인합니다.

```bash
claudy status
```

### claudy config

설정을 관리합니다.

#### claudy config show

현재 설정을 표시합니다.

```bash
claudy config show
```

#### claudy config repo

원격 저장소를 설정하거나 조회합니다.

```bash
# 저장소 설정
claudy config repo owner/repo

# 현재 저장소 조회
claudy config repo
```

#### claudy config whitelist

동기화할 파일 화이트리스트를 관리합니다.

```bash
# 목록 표시
claudy config whitelist list

# 경로 추가 (glob 패턴 지원)
claudy config whitelist add <path>

# 경로 제거
claudy config whitelist remove <path>
```

**glob 패턴 예시:**
- `settings.json` - 특정 파일
- `CLAUDE.md` - 특정 파일
- `commands/**/*.md` - commands 하위 모든 .md 파일
- `skills/*.md` - skills 디렉토리의 .md 파일

## 설정 파일

`.claudy/config.toml`에 저장됩니다.

```toml
repo = "owner/repo"
sync_mode = "whitelist"

[whitelist]
paths = [
    "settings.json",
    "CLAUDE.md",
    "commands/**/*.md"
]
```

## 사용 예시

```bash
# 1. 저장소 설정
claudy config repo myuser/claude-settings

# 2. 동기화할 파일 추가
claudy config whitelist add "settings.json"
claudy config whitelist add "CLAUDE.md"
claudy config whitelist add "commands/**/*.md"

# 3. 상태 확인
claudy status

# 4. 차이점 확인
claudy diff

# 5. 로컬 설정을 원격으로 푸시
claudy push

# 6. 원격 설정을 로컬로 풀
claudy pull
```
