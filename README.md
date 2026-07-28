# RunDev

로컬 우선 개발 활동 컴패니언. Tauri 2, React, TypeScript, Rust, SQLite로 만든다.

RunDev는 개발 활동과 AI 도구 사용량을 로컬에서 집계하고, XP와 트레이 캐릭터로
보여주는 Windows/macOS 앱이다. 키 입력, 프롬프트, 소스 코드 내용은 수집하지 않는다.

## 개발

```powershell
npm.cmd install
npm.cmd run tauri dev
```

브라우저에서 UI만 확인하려면 `npm.cmd run dev`를 사용한다. 이때는 미리보기 데이터가
표시된다.

## 현재 구현

- 트레이 상주 및 클릭으로 창 열기
- 창을 닫아도 백그라운드 유지
- 단일 인스턴스와 우클릭 종료 메뉴
- 32×32 PNG 프레임 트레이 애니메이션
- SQLite 자동 생성과 초기 마이그레이션
- Rust command를 통한 오늘 요약/캐릭터 상태 조회
- React + Zustand 대시보드

Updater 의존성은 포함되어 있지만, 서명키와 배포 endpoint를 발급하기 전까지
런타임 초기화는 하지 않는다.

## 문서

- [문서 인덱스](docs/README.md)
- [시스템 아키텍처](docs/architecture/overview.md)
- [ADR 인덱스](docs/decisions/README.md)
- [AI 및 기여자 작업 규칙](AGENTS.md)
