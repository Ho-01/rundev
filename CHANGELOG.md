# Changelog

RunDev의 사용자에게 의미 있는 변경을 기록한다. 커밋 목록이 아니라 기능, 동작,
호환성 변화 중심으로 작성한다.

## Unreleased

### Added

- 주황 고양이, 하양 고양이, 핑크 버튜버 러너
- 노트북하는 노란 물고기 러너와 러너 선택·영속화
- 주요 UI 상태 스크린샷과 지원 러너 GIF를 표시하는 README 갤러리
- UI·러너 변경 시 README 애셋을 자동 갱신하는 GitHub Actions
- Tauri 2 기반 Windows/macOS 트레이 애플리케이션
- React 대시보드와 Rust command 경계
- SQLite 로컬 저장소와 migration
- PNG 프레임 기반 트레이 애니메이션
- 단일 인스턴스, 팝오버, 우클릭 종료 메뉴
- Claude Code OpenTelemetry 기반 로컬 토큰 사용량 연동
- Codex 계정 사용량 연동
- macOS universal DMG 자동 테스트 빌드
- 앱 버전 동기화와 검증 도구
- 익명 세션 키 기반 Claude Code 세션 수와 활성 AI 표시
- Codex와 Claude Code 공급자 아이콘

### Changed

- 모든 러너의 피사체 크기와 픽셀 구도를 기본 검은 고양이 기준으로 통일
- 좌측 영역 고정 후처리를 제거해 모든 러너의 전체 프레임 움직임 복원
- 핑크 버튜버를 눈 깜빡임과 윙크가 원본에 포함된 깨끗한 스프라이트로 교체
- AI 공급자별 연동, 대기, 오류 상태를 구분해 표시
- 트레이 캐릭터와 320×480 팝오버 레이아웃 개선
