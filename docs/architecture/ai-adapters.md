# AI 어댑터

## 경계

AI 어댑터는 Rust가 소유한다. React는 `src/services/rundev.ts`에 캡슐화된 Tauri
command로 연결 상태와 집계 결과만 조회한다.

```text
AI provider or local tool
        ↓
Rust adapter
        ↓
normalized snapshot/event
        ↓
SQLite
        ↓
Tauri command
        ↓
Zustand / React
```

## Codex

Codex 어댑터는 사용자가 승인한 로그인 환경의 계정 사용량을 주기적으로 조회해
`ai_usage_snapshots`에 일간 스냅샷으로 저장한다. 특정 CLI 프로세스의 이벤트를
수집하는 구조가 아니다.

## Claude Code

Claude 어댑터는 `127.0.0.1:43182`에서 OTLP/HTTP JSON 로그를 받는다. 연동마다
생성한 비밀 헤더가 일치하고 `claude_code.api_request`인 이벤트만 정규화해
`ai_usage_events`에 기록한다.

## Cursor

Cursor의 요청 기반 팀 플랜은 `/api/usage-summary`의 plan 값을 달러로 해석하지
않는다. 해당 값은 센트 환산 크레딧이므로 요청당 4센트 기준으로 요청 수를
계산한다. 오늘 토큰과 요청 수는 `get-filtered-usage-events`를 최대 5페이지까지
조회해 메모리에서 합산하고, 원본 이벤트·대화 ID·모델명은 저장하지 않는다.
온디맨드가 활성화된 경우에만 별도의 금액 사용량을 해석한다.

Cursor 어댑터는 사용자의 명시 동의 후 Cursor `globalStorage/state.vscdb`를
read-only로 열어 `cursorAuth/accessToken` 값 하나만 읽는다. 토큰은 메모리에서
`cursor.com` 요청에만 사용하고 저장하거나 로그에 남기지 않는다.

계정 확인, 오늘 집계 및 현재 결제 주기 한도를 조회해 account-key별
`cursor_usage_snapshots`에 저장한다. 상세 이벤트에는 안정적인 외부 ID가 없으므로
1차 어댑터는 이벤트 원장을 만들지 않는다. 기본 polling은 300초이며 RunDev
팝오버를 열 때 마지막 시도 후 60초가 지났으면 Codex와 병렬 갱신한다.

`external_event_id`의 유니크 인덱스로 여러 세션이나 재전송의 중복을 막는다.
`session.id`는 SHA-256 `session_key`로 변환해 요청 이벤트에 저장하며 원본 식별자는
폐기한다. 오늘의 세션 수와 최근 15분 활성 세션은 이 익명 키를 기준으로 계산한다.
프롬프트, 응답, 도구 내용 속성은 설정에서 비활성화하며 DB 모델에도 원문 필드를
두지 않는다.

## 공통 상태

`ai_adapter_state`는 마지막 성공 시각과 오류를 저장한다. 연결 여부와 사용자 설정은
`app_settings`에 저장한다. 공급자별 원본 형식은 UI로 직접 전달하지 않는다.

상단 `활성 AI`에서 Claude는 최근 요청 이벤트로 확인하고, Codex는 최근 두 계정
스냅샷의 토큰 증가로 추론한다. Codex 계정 API는 세션 식별자를 제공하지 않으므로
Codex 세션 수는 만들지 않는다.
