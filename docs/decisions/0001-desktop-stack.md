# 0001. Tauri 2 기반 데스크톱 스택

- 상태: Accepted
- 날짜: 2026-07-28

## 맥락

RunDev는 Windows와 macOS에서 항상 실행되는 트레이 앱이다. 낮은 메모리 사용량,
네이티브 OS API 접근, 빠른 UI 개발, 단일 코드베이스가 모두 필요하다.

## 결정

다음 스택을 사용한다.

- Tauri 2
- React + TypeScript + Vite
- Tailwind CSS와 제한적인 shadcn/ui
- Zustand, Recharts
- Rust + Tokio
- SQLite + sqlx
- notify, tracing, chrono, serde
- windows-rs와 objc2 계열 플랫폼 API

## 결과

### 장점

- 웹 기술로 UI를 빠르게 개발하면서 핵심 수집기는 네이티브 성능을 유지한다.
- Electron보다 배포 크기와 상주 메모리를 줄일 수 있다.
- 공통 도메인 로직을 Rust로 공유하고 OS 어댑터만 분리할 수 있다.

### 단점

- WebView 동작 차이를 플랫폼별로 검증해야 한다.
- Rust와 TypeScript 사이 command 타입을 함께 관리해야 한다.
- macOS 빌드와 서명 검증은 macOS 환경이 필요하다.

## 대안

- Electron: 생태계는 크지만 상주 앱에 필요한 자원 비용이 크다.
- Flutter: 크로스 플랫폼 UI는 좋지만 Rust/OS API 결합이 더 복잡하다.
- 완전 네이티브 이중 구현: 품질은 높지만 초기 개발 비용이 지나치게 크다.

