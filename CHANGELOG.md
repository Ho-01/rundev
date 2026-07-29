# Changelog

RunDev의 사용자에게 의미 있는 변경을 기록한다. 커밋 목록이 아니라 기능, 동작,
호환성 변화 중심으로 작성한다.

## Unreleased

### Added

- GitHub Releases `latest.json` 기반 앱 업데이트 확인과 수동 설치
- RunDev 정보 창의 업데이트 확인 / 다운로드 및 재시작

### Changed

- 릴리스 워크플로에 Tauri updater 서명 아티팩트와 `latest.json` 업로드 추가

### Fixed

- macOS에서 Dock과 Cmd+Tab에 나타나던 트레이 앱 아이콘을 숨김

## 0.3.1 - 2026-07-29

### Fixed

- macOS 릴리스 빌드를 Apple Silicon과 Intel DMG로 분리해 릴리스 자산 누락을 방지

## 0.3.0 - 2026-07-29

### Added

- 개발자 레벨을 10단계 등급과 등급별 엠블럼으로 확장
- 최근 20주 개발 활동 잔디와 날짜별 활동 강도 표시
- 상단 RunDev 정보 아이콘과 앱 정보 모달
- GitHub Release에 Windows NSIS와 macOS universal DMG 자동 업로드

### Changed

- 개발자 상태 배지를 숫자 네모에서 등급 엠블럼으로 변경
- 트레이 팝오버 콘텐츠가 길어질 때 내부 세로 스크롤 지원

## 0.2.0 - 2026-07-28

### Added

- 주황 고양이, 주황 새우, 핑크 버튜버 개발자 캐릭터
- 노트북하는 노란 물고기 러너와 러너 선택·영속화
- 주요 UI 상태 스크린샷과 지원 개발자 캐릭터 GIF를 표시하는 README 갤러리
- UI·개발자 캐릭터 변경 시 README 애셋을 자동 갱신하는 GitHub Actions
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
- 입력 내용을 저장하지 않는 일별 키보드 횟수 집계와 2,000회당 10 XP 보상
- macOS 입력 모니터링 권한 확인과 시스템 설정 연결

### Changed

- 러너 애셋을 256×256 마스터, 128×128 UI, 32×32 트레이 계층으로 분리
- 팝오버와 README에서 트레이 확대 이미지 대신 고해상도 UI 프레임 사용
- 모든 러너의 피사체 크기와 픽셀 구도를 기본 검은 고양이 기준으로 통일
- 좌측 영역 고정 후처리를 제거해 모든 러너의 전체 프레임 움직임 복원
- 핑크 버튜버를 눈 깜빡임과 윙크가 원본에 포함된 깨끗한 스프라이트로 교체
- AI 공급자별 연동, 대기, 오류 상태를 구분해 표시
- 트레이 캐릭터와 320×480 팝오버 레이아웃 개선
