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

러너는 `src/assets/runners/master/`의 256×256 프레임을 화면 표시의 원본으로
사용한다. UI용 128×128 프레임은 다음 명령으로 생성하며 `npm.cmd run build`
실행 전에도 자동으로 갱신된다.

```powershell
npm.cmd run runners:ui
```

러너 프레임을 새로 만들 때는 노트북과 상체가 프레임마다 흔들리지 않도록 마스터 및
트레이 프레임에 안정화 단계를 적용한다. 1번 프레임을 고정 기준으로 삼고, 앞발과
키보드가 맞닿는 작은 영역만 2–4번 프레임에서 유지한다. 버튜버의 눈 깜빡임처럼
의도한 보조 동작은 러너별 허용 영역으로 명시한다.

```powershell
node scripts/stabilize-runner-animation.mjs coding-cat
node scripts/stabilize-runner-animation.mjs coding-cat src-tauri/icons/tray/coding 32
npm.cmd run runners:ui
```

React는 `src/assets/runners/ui/`만 사용하고 네이티브 트레이는
`src-tauri/icons/tray/`의 32×32 전용 애셋만 사용한다. 화면에서 트레이 프레임을
확대해 사용하지 않는다.

README에 들어가는 상태별 화면과 러너 GIF는 실제 사용자 데이터 대신 URL의 고정
미리보기 시나리오를 사용한다.

```powershell
npm.cmd run build
npm.cmd run docs:assets
```

생성 결과는 `docs/assets/readme/`에 저장한다. UI나 트레이 러너 변경이 `main`에
푸시되면 `readme-assets.yml`이 같은 명령을 실행하고, 결과가 달라졌을 때만
`github-actions[bot]` 커밋을 만든다.

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
