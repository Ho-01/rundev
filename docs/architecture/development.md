# 개발 및 검증

## 요구 사항

- Node.js 22 이상
- npm 10 이상
- Rust stable
- Windows: WebView2
- macOS 빌드: Xcode command line tools

## 설치와 실행

```powershell
npm.cmd install
npm.cmd run tauri dev
```

앱은 창이 아닌 트레이에서 시작한다. 기존 개발 프로세스가 남아 포트 충돌이 나면
실행한 터미널에서 `Ctrl+C`로 종료한다.

UI만 확인:

```powershell
npm.cmd run dev
```

브라우저 모드에서는 SQLite 대신 preview 데이터가 표시된다.

## 검증

```powershell
npm.cmd run version:check
npm.cmd run build
npm.cmd test
cargo check --manifest-path src-tauri/Cargo.toml
cargo test --manifest-path src-tauri/Cargo.toml --lib
npm.cmd audit
git diff --check
```

`v*` 릴리스 태그를 만들기 전에는 태그와 앱 버전도 검증한다.

```powershell
npm.cmd run version:check-tag -- v0.2.0
```

## 트레이 프레임 생성

ImageGen에서 만든 마젠타 chroma-key 4열 스프라이트를 입력으로 사용한다.

```powershell
npm.cmd run tray:frames -- <sprite.png> src-tauri/icons/tray/coding
```

출력:

```text
src-tauri/icons/tray/coding/
├─ 01.png
├─ 02.png
├─ 03.png
└─ 04.png
```

각 프레임은 투명한 32×32 PNG다. 생성 후 실제 Windows 배율 100%, 125%, 150%와
macOS Retina에서 확인한다.

## migration

- 파일명은 `NNNN_description.sql` 형식을 사용한다.
- 배포되었거나 커밋된 migration은 수정하지 않는다.
- 스키마 변경에는 쿼리와 migration 검증 테스트를 함께 추가한다.

## 플랫폼 검증 체크리스트

Windows:

- 트레이 아이콘 하나만 표시
- 좌클릭 위치와 우클릭 메뉴
- 작업 표시줄 위/아래/좌/우
- 다중 모니터와 배율
- 창 포커스 상실 시 숨김

macOS:

- template icon 색상
- 메뉴바 아래 팝오버 위치
- Retina 프레임 선명도
- 잠금/절전 복귀
- Accessibility 권한 없이 가능한 기본 기능

## 자동 검증

- `CI`: PR과 `main` 푸시에서 Windows 빌드, 프론트/Rust 테스트, 버전 일치와 npm
  audit을 검사한다.
- `macOS test build`: 앱 관련 `main` 변경에서 universal DMG를 만든다.
- `Test release`: `v*` 태그에서 태그와 앱 버전을 확인하고 Windows NSIS와 macOS
  universal DMG를 draft prerelease에 업로드한다.
