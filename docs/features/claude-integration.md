# Claude Code 사용량 연동

## 사용자 흐름

1. RunDev에서 Claude Code 연동을 선택한다.
2. 변경할 `~/.claude/settings.json` 경로와 기존 설정 충돌 여부를 확인한다.
3. 사용자가 승인하면 RunDev가 Claude Code의 공식 OpenTelemetry 로그 목적지를
   로컬 수집기로 설정한다.
4. 실행 중인 Claude Code 프로세스를 다시 시작한다.
5. 이후 API 요청의 토큰 이벤트가 RunDev SQLite에 저장된다.

대화는 새로 만들 필요가 없다. Claude Code를 다시 실행한 뒤 기존 대화를 resume하면
그 이후 요청부터 수집된다.

## 데이터 흐름

```text
Claude Code
  → OTLP/HTTP JSON
  → http://127.0.0.1:43182/v1/logs
  → RunDev Rust collector
  → ai_usage_events
  → Tauri command
  → React
```

OpenTelemetry는 외부 RunDev 서버가 아니라 Claude Code와 로컬 수집기가 사용하는
전송 규격이다.

## 여러 세션

연동 이후 시작한 여러 터미널과 IDE의 Claude Code 프로세스는 모두 같은 로컬
수집기로 전송한다. 요청 ID를 기준으로 중복 이벤트를 제거한다. 연동 전에 이미 실행
중이던 프로세스는 설정을 다시 읽지 않으므로 재시작 전 요청은 수집되지 않는다.

## 재시작과 누락

- 연동 설정은 파일에 유지되므로 RunDev를 실행할 때마다 다시 연동하지 않는다.
- Claude Code 프로세스는 시작 시 환경 설정을 읽으므로 최초 연동과 해제 뒤 재시작이
  필요하다.
- RunDev가 꺼진 동안 발생한 이벤트는 로컬 수집기가 없어 누락될 수 있다.
- 연동 이전의 과거 사용량은 공식 OTel 이벤트로 소급 수집하지 않는다.

## 개인정보와 해제

프롬프트, 응답, 도구 상세와 원시 API 본문 로그는 명시적으로 비활성화한다. 연동 전에
존재하던 관리 대상 환경 변수는 보관하며, 해제 시 사용자가 별도로 바꾸지 않은 항목만
복원한다.
