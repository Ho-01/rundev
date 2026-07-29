# RunDev

로컬 우선 개발 활동 컴패니언. Tauri 2, React, TypeScript, Rust, SQLite로 만든
Windows/macOS 트레이 앱입니다.

RunDev는 개발 활동과 사용자가 연결한 AI 도구의 집계 사용량을 로컬에서 기록하고,
XP와 작은 개발자 캐릭터로 보여줍니다. 키 입력 내용은 읽거나 저장하지 않고 누른
횟수만 집계하며, 프롬프트·응답·소스 코드 내용은 수집하지 않습니다.

## 주요 화면

| 연동 전 | AI 활동 중 |
| --- | --- |
| <img src="docs/assets/readme/dashboard-disconnected.png" width="280" alt="AI 도구 연동 전 RunDev 화면"> | <img src="docs/assets/readme/dashboard-active.png" width="280" alt="Codex와 Claude Code 활동이 집계된 RunDev 화면"> |
| **연동 후 데이터 대기** | **개발자 변경** |
| <img src="docs/assets/readme/dashboard-connected.png" width="280" alt="AI 도구 연동 후 데이터를 기다리는 RunDev 화면"> | <img src="docs/assets/readme/runner-picker.png" width="280" alt="RunDev 개발자 캐릭터 선택 화면"> |

화면 이미지는 320×480 팝오버 크기의 고정된 미리보기 데이터로 생성됩니다. 실제
SQLite 데이터나 계정 정보는 문서 이미지 생성에 사용하지 않습니다.

## 개발자 레벨

<img src="docs/assets/readme/level-progression.png" width="320" alt="RunDev 개발자 레벨 등급 미리보기">

활동 XP 100마다 레벨이 오르며, 레벨 구간에 따라 배지 색상과 개발자 등급이
변합니다.

| 레벨 | 등급 |
| --- | --- |
| 1–4 | 새싹 개발자 |
| 5–9 | 루키 빌더 |
| 10–19 | 코드 메이커 |
| 20–34 | 디버거 |
| 35–49 | 코드 해커 |
| 50–69 | 시스템 설계자 |
| 70–99 | 아키텍트 |
| 100–149 | 테크 리드 |
| 150–249 | 마스터 개발자 |
| 250+ | 레전드 개발자 |

## 지원 개발자 캐릭터

| 코딩 고양이 | 주황 고양이 | 주황 새우 | 노란 물고기 | 핑크 버튜버 |
| --- | --- | --- | --- | --- |
| <img src="docs/assets/readme/coding-cat.gif" width="96" height="96" alt="노트북하는 코딩 고양이 애니메이션"> | <img src="docs/assets/readme/coding-orange-cat.gif" width="96" height="96" alt="노트북하는 주황 고양이 애니메이션"> | <img src="docs/assets/readme/coding-shrimp.gif" width="96" height="96" alt="노트북하는 주황 새우 애니메이션"> | <img src="docs/assets/readme/coding-fish.gif" width="96" height="96" alt="노트북하는 노란 물고기 애니메이션"> | <img src="docs/assets/readme/coding-vtuber.gif" width="96" height="96" alt="노트북하는 분홍머리 버튜버 애니메이션"> |

개발자 캐릭터는 앱의 **개발자 변경** 메뉴에서 선택하며, 헤더와 네이티브 트레이 애니메이션에
동시에 적용됩니다.

## 개발

```powershell
npm.cmd install
npm.cmd run tauri dev
```

브라우저에서 UI만 확인하려면 `npm.cmd run dev`를 사용합니다. README용 주요 화면과
개발자 캐릭터 GIF를 로컬에서 다시 만들려면 다음 명령을 실행합니다.

```powershell
npx.cmd playwright install chromium
npm.cmd run build
npm.cmd run docs:assets
```

`main`에 UI 또는 개발자 캐릭터 애셋 변경이 푸시되면 GitHub Actions가 이미지를 다시 만들고,
변경된 결과만 자동 커밋합니다.

## 버전

앱 버전은 여러 패키지와 번들 설정에서 동일하게 유지합니다. 직접 개별 파일을
수정하지 않고 다음 명령을 사용합니다.

```powershell
npm.cmd run version:check
npm.cmd run version:set -- 0.2.0
```

릴리스 정책과 태그 절차는 [버전과 릴리스 관리](docs/releases/versioning.md)를 참고합니다.

## 현재 구현

- 트레이 상주 및 좌클릭으로 창 토글
- 창을 닫아도 백그라운드 유지
- 단일 인스턴스와 우클릭 종료 메뉴
- 32×32 PNG 프레임 트레이 애니메이션
- 256×256 마스터에서 생성하는 128×128 고해상도 화면 개발자 캐릭터
- 고양이 2종, 주황 새우, 노란 물고기, 핑크 버튜버 선택
- SQLite 자동 생성과 초기 마이그레이션
- Codex·Cursor 계정 사용량과 Claude Code OpenTelemetry 집계
- 오늘 누른 키보드 횟수 집계와 2,000회당 10 XP 지급
- 개발 도구 활동 30분당 10 XP 지급과 최근 20주 활동 잔디
- 레벨 구간별 개발자 등급과 성장 배지
- Rust command를 통한 요약·캐릭터·AI 상태 조회
- React + Zustand 대시보드
- GitHub Releases 기반 앱 업데이트 확인과 수동 설치

Updater는 서명된 `latest.json` endpoint로 새 버전을 확인합니다. Updater가 처음
포함된 빌드는 한 번 수동 설치해야 이후 버전을 자동으로 감지합니다.

## 문서

- [문서 인덱스](docs/README.md)
- [시스템 아키텍처](docs/architecture/overview.md)
- [ADR 인덱스](docs/decisions/README.md)
- [제품 로드맵](docs/product/roadmap.md)
- [변경 기록](CHANGELOG.md)
- [AI 및 기여자 작업 규칙](AGENTS.md)
