# Architecture Decision Records

ADR은 이미 내려진 중요한 기술 결정을 기록한다. 결정이 바뀌어도 기존 문서를
삭제하거나 내용을 반대로 고치지 않고, 새로운 ADR에서 이전 결정을 대체한다.

## 상태

- `Proposed`: 검토 중
- `Accepted`: 현재 적용 중
- `Superseded`: 새로운 ADR로 대체됨
- `Deprecated`: 더 이상 권장하지 않음

## 목록

| 번호 | 결정 | 상태 |
| --- | --- | --- |
| [0001](0001-desktop-stack.md) | Tauri 2 기반 데스크톱 스택 | Accepted |
| [0002](0002-local-first-data-ownership.md) | Rust가 로컬 데이터를 소유 | Accepted |
| [0003](0003-tray-popover-lifecycle.md) | 트레이 팝오버 수명주기 | Accepted |
| [0004](0004-event-ledger-xp.md) | 이벤트 원장 기반 XP | Accepted |
| [0005](0005-privacy-minimal-collection.md) | 개인정보 최소 수집 | Accepted |
| [0006](0006-png-frame-tray-animation.md) | PNG 프레임 트레이 애니메이션 | Accepted |
| [0007](0007-ai-usage-observation-model.md) | AI 사용량 이벤트와 스냅샷 분리 | Accepted |
| [0008](0008-claude-opentelemetry-usage.md) | Claude Code 로컬 OpenTelemetry 사용량 수집 | Accepted |
| [0009](0009-ai-session-activity.md) | 익명 키 기반 AI 세션 활동 집계 | Accepted |
| [0010](0010-keyboard-count-collection.md) | 키 입력 횟수의 로컬 집계 | Accepted |
| [0011](0011-focus-time-classification.md) | 활성 개발 앱과 입력 유휴 시간으로 집중시간 집계 | Accepted |
| [0012](0012-developer-level-tiers.md) | 개발자 레벨을 열 개 등급의 성장 배지로 표현 | Accepted |
| [0013](0013-github-releases-updater.md) | GitHub Releases 정적 JSON으로 앱 업데이트 | Accepted |
| [0014](0014-local-host-metrics-display.md) | 로컬 호스트 지표의 표시 전용 수집 | Accepted |
| [0015](0015-runner-whip-local-count.md) | 개발자 캐릭터 채찍질 횟수의 로컬 집계 | Accepted |
| [0016](0016-cursor-dashboard-usage.md) | Cursor 비공식 Dashboard 사용량 연동 | Accepted |
| [0017](0017-expandable-host-metrics-detail.md) | 장치 상태 요약과 상세 패널 분리 | Accepted |
| [0018](0018-local-diagnostic-logs.md) | 제한된 로컬 진단 로그 | Accepted |
| [0019](0019-signed-local-xp-coupons.md) | 서명된 로컬 XP 쿠폰 | Accepted |
| [0020](0020-weekly-ai-usage-xp.md) | 프로바이더 통합 주간 AI 사용 XP | Accepted |
| [0021](0021-traits-and-activity-statistics.md) | 로컬 활동 통계와 특성 성장 | Accepted |
| [0022](0022-character-pointer-following.md) | 화면 캐릭터의 전역 포인터 따라다니기 | Accepted |
| [0035](0035-character-file-drop-to-trash.md) | 플로팅 캐릭터 파일 드롭 휴지통 처리 | Accepted |

## 새 ADR 작성 형식

```markdown
# NNNN. 결정 제목

- 상태: Proposed
- 날짜: YYYY-MM-DD

## 맥락

## 결정

## 결과

### 장점

### 단점

## 대안
```
