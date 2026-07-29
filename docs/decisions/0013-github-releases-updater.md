# 0013. GitHub Releases 정적 JSON으로 앱 업데이트

- 상태: Accepted
- 날짜: 2026-07-29

## 맥락

RunDev는 로컬 우선 트레이 앱이며 전용 업데이트 서버를 운영할 단계는 아니다.
의존성에는 Tauri Updater가 이미 포함되어 있었지만 서명 키와 endpoint가 없어
초기화하지 않았다. ADR 0002는 서명키와 배포 endpoint가 준비된 뒤 별도 ADR로
활성화하도록 남겨 두었다.

## 결정

- 업데이트 배포 저장소로 GitHub Releases를 사용한다.
- endpoint는
  `https://github.com/Ho-01/rundev/releases/latest/download/latest.json` 이다.
- Tauri updater 서명 키로 릴리스 아티팩트를 서명한다. 공개키만 앱에 포함하고
  비밀키는 GitHub Actions secret으로만 보관한다.
- `createUpdaterArtifacts: true`와 `tauri-action`의 `latest.json` 병합 업로드로
  Windows NSIS와 macOS 대상별 updater 번들을 배포한다.
- 앱은 시작 시, 15분마다, 정보 다이얼로그 열 때 업데이트를 확인한다. 수동
  “업데이트 확인”은 throttle 없이 즉시 요청한다.
- 새 버전이 있으면 알림을 띄우고, 설치는 정보 창의 “다운로드 및 재시작”으로만
  수행한다. 자동 설치는 코드 서명·notarization이 안정된 뒤 검토한다.
- Updater가 처음 들어간 버전은 bootstrap release다. 그 이전 설치본은 자동으로
  새 버전을 받지 못하므로 한 번 수동 설치가 필요하다.
- Updater 서명과 Apple/Windows 코드 서명은 별개다. macOS ad-hoc 서명 상태에서는
  Gatekeeper 경고가 계속 나타날 수 있다.

## 결과

### 장점

- 전용 서버 없이 공개 릴리스와 자동 확인을 함께 운영할 수 있다.
- 업데이트 파일 변조를 updater 서명으로 막을 수 있다.
- 설치 시점을 사용자가 고르므로 활동 기록 중 갑작스러운 재시작을 줄인다.

### 단점

- GitHub/CDN 요청 로그에 IP, User-Agent, 앱 버전·OS/arch가 남을 수 있다.
- bootstrap 이전 설치본은 수동 업그레이드가 필요하다.
- 비밀키를 분실하면 기존 설치 사용자에게 새 서명을 이어갈 수 없다.
- macOS notarization 전까지는 업데이트 설치 후에도 OS 보안 경고가 남을 수 있다.

## 대안

- 전용 업데이트 서버는 채널 분리·점진 배포·강제 업데이트가 필요할 때 도입한다.
- 정적 JSON 없이 GitHub API만 쓰는 방식은 Tauri Updater 기본 흐름과 맞지 않아
  선택하지 않았다.
