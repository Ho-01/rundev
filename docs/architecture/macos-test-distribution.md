# macOS 테스트 배포

## 빌드 방식

Windows에서는 macOS 앱 번들을 만들지 않는다. `.github/workflows/macos-test-build.yml`
워크플로가 GitHub의 macOS 러너에서 다음 두 Rust 타깃을 결합한 universal DMG를
생성한다.

- `aarch64-apple-darwin`: Apple Silicon
- `x86_64-apple-darwin`: Intel

초기 테스트 빌드는 `APPLE_SIGNING_IDENTITY=-`를 사용한 ad-hoc 서명이다. Apple
notarization을 거치지 않으므로 테스터가 macOS 개인정보 보호 및 보안 설정에서
실행을 승인해야 할 수 있다. 외부 공개 배포 전에는 Developer ID Application
서명과 notarization을 추가한다.

## 실행

1. GitHub 저장소의 **Actions** 탭을 연다.
2. **macOS test build**를 선택한다.
3. **Run workflow**를 누른다.
4. 성공한 실행의 **Artifacts**에서 `RunDev-macOS-universal-*`을 내려받는다.
5. ZIP 안의 DMG를 테스터에게 전달한다.

테스터는 DMG를 열고 RunDev를 Applications로 옮긴다. 최초 실행이 차단되면
Applications에서 앱을 우클릭해 **열기**를 선택하거나, 시스템 설정의
**개인정보 보호 및 보안**에서 실행을 승인한다.

## Codex CLI 탐색

Finder에서 실행한 GUI 앱은 터미널 로그인 셸의 `PATH`를 그대로 받지 않는다.
RunDev는 다음 순서로 Codex 실행 파일을 찾는다.

1. 명시적인 `CODEX_PATH`
2. 현재 프로세스의 `PATH`
3. `/opt/homebrew/bin`과 `/usr/local/bin`
4. 사용자 홈의 npm, Bun, Volta, asdf, Nix, pnpm 설치 위치
5. nvm 및 fnm의 Node 버전별 설치 위치

Codex 인증은 실행 파일 위치와 별개다. 기본적으로 Codex의 기본 인증 환경을
사용하고, `CODEX_HOME`이 설정되어 있으면 해당 사용자 지정 인증 환경을 사용한다.

