# Homebrew Tap 설정 가이드

## Tap 저장소 생성

1. GitHub에 `homebrew-tap` 저장소 생성
2. `clync.rb` 파일을 해당 저장소의 루트에 복사
3. SHA256 해시값을 실제 릴리즈 바이너리의 해시로 교체

## SHA256 해시 생성

릴리즈 후 다음 명령으로 해시 생성:

```bash
curl -sL https://github.com/novdov/clync/releases/download/v0.1.0/clync-darwin-arm64 | shasum -a 256
curl -sL https://github.com/novdov/clync/releases/download/v0.1.0/clync-darwin-x64 | shasum -a 256
curl -sL https://github.com/novdov/clync/releases/download/v0.1.0/clync-linux-x64 | shasum -a 256
```

## 사용자 설치 방법

```bash
brew tap novdov/tap
brew install clync
```

## 업데이트 방법

```bash
brew upgrade clync
```

## 자동화

GitHub Actions로 릴리즈 시 Formula 자동 업데이트:

```yaml
- name: Update Homebrew Formula
  env:
    HOMEBREW_TAP_TOKEN: ${{ secrets.HOMEBREW_TAP_TOKEN }}
  run: |
    # homebrew-tap 저장소 클론
    # version, sha256 업데이트
    # 커밋 및 푸시
```
