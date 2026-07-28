# RunDev Agent Guide

이 문서는 RunDev 저장소에서 사람과 AI 코딩 에이전트가 따라야 할 기본 규칙이다.

## 제품 원칙

- RunDev는 Windows와 macOS를 지원하는 로컬 우선 트레이 앱이다.
- 사용자 활동 원문, 키 입력, 프롬프트, 소스 코드는 수집하지 않는다.
- React는 화면 표시만 담당하며 SQLite에 직접 접근하지 않는다.
- 활동 판정, XP 계산, 영속성, 트레이 상태는 Rust가 소유한다.
- 서버 기능은 명시적인 제품 결정 전까지 추가하지 않는다.

## 기술 경계

- 프론트엔드: `src/`
- 네이티브 애플리케이션: `src-tauri/src/`
- DB 마이그레이션: `src-tauri/migrations/`
- 트레이 애셋: `src-tauri/icons/tray/`
- 아키텍처 문서: `docs/architecture/`
- 결정 기록: `docs/decisions/`

프론트엔드에서 네이티브 기능이 필요하면 `src/services/`에 Tauri command 호출을
캡슐화한다. 컴포넌트에서 `invoke`를 직접 호출하지 않는다.

Rust 모듈은 다음 방향을 유지한다.

```text
Tauri command / tray
        ↓
application service
        ↓
domain logic
        ↓
database / OS adapter
```

도메인 로직이 Tauri window나 React 타입에 의존하면 안 된다.

## 개인정보 규칙

기본 수집 허용 범위:

- 활성 앱의 분류와 실행 시간
- 마지막 입력 이후 경과 시간
- 화면 잠금 여부
- 사용자가 명시적으로 연결한 AI 도구의 집계 사용량

기본 수집 금지 범위:

- 키 입력 내용
- 클립보드 내용
- 창 본문과 화면 캡처
- 프롬프트 및 응답 원문
- 소스 코드 내용

새로운 수집 항목은 구현 전에 ADR을 작성해야 한다.

## DB 변경

- 기존 migration을 수정하지 않고 새 번호의 migration을 추가한다.
- React에서 SQLite를 직접 열지 않는다.
- XP 변경은 합계만 수정하지 않고 `xp_events` 원장에 기록한다.
- 외부 이벤트는 가능한 경우 `source_event_id`로 중복 처리를 막는다.

## UI 및 트레이

- 기본 창은 일반 데스크톱 창이 아닌 고정 크기 트레이 팝오버다.
- 타이틀바, 작업 표시줄 항목, 최대화/최소화 버튼을 추가하지 않는다.
- 좌클릭은 팝오버 토글, 우클릭은 네이티브 메뉴다.
- 트레이 애셋은 투명 32×32 PNG이며 실제 피사체는 안전 여백 안에 둔다.
- 플랫폼별 표현 차이가 생기면 Windows와 macOS 애셋/코드를 분리한다.

## 변경 후 검증

최소 검증:

```powershell
npm.cmd run version:check
npm.cmd run build
npm.cmd test
cargo check --manifest-path src-tauri/Cargo.toml
git diff --check
```

DB 또는 Rust 도메인 로직을 변경했다면:

```powershell
cargo test --manifest-path src-tauri/Cargo.toml --lib
```

의존성을 변경했다면:

```powershell
npm.cmd audit
```

## 커밋 메시지

```text
feat(english-scope): 한글 설명
fix(english-scope): 한글 설명
docs(english-scope): 한글 설명
refactor(english-scope): 한글 설명
test(english-scope): 한글 설명
chore(english-scope): 한글 설명
```

- scope는 소문자 영어 kebab-case를 사용한다.
- 제목은 명령형으로 간결하게 쓴다.
- 서로 독립적인 기능은 가능한 한 별도 커밋으로 나눈다.

## 문서 동기화

다음 변경은 코드와 같은 PR/커밋에서 문서를 갱신한다.

- 기술 또는 데이터 소유권 결정: ADR
- 모듈 경계나 데이터 흐름 변경: architecture
- 실행 및 검증 명령 변경: README와 이 문서

앱 버전은 개별 파일을 직접 수정하지 않고 다음 명령으로 변경한다.

```powershell
npm.cmd run version:set -- X.Y.Z
```

버전 정책, CHANGELOG 형식 또는 릴리스 절차 변경은
`docs/releases/versioning.md`와 `CHANGELOG.md`를 함께 확인한다.
