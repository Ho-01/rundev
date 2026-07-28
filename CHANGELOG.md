# Changelog

RunDev의 사용자에게 의미 있는 변경을 기록한다. 커밋 목록이 아니라 기능, 동작,
호환성 변화 중심으로 작성한다.

## Unreleased

### Added

- Tauri 2 기반 Windows/macOS 트레이 애플리케이션
- React 대시보드와 Rust command 경계
- SQLite 로컬 저장소와 migration
- PNG 프레임 기반 트레이 애니메이션
- 단일 인스턴스, 팝오버, 우클릭 종료 메뉴
- Claude Code OpenTelemetry 기반 로컬 토큰 사용량 연동
- Codex 계정 사용량 연동
- macOS universal DMG 자동 테스트 빌드
- 앱 버전 동기화와 검증 도구

### Changed

- AI 공급자별 연동, 대기, 오류 상태를 구분해 표시
- 트레이 캐릭터와 320×480 팝오버 레이아웃 개선
