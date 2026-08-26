# 0039. 장시간 실행의 DB, 네이티브 입력, 절전 복귀 안전성

> 배터리 manager 수명주기 결정은 [ADR 0041](0041-native-sampler-and-crash-diagnostics.md)에서 대체했다.

- 상태: Accepted
- 날짜: 2026-08-19

## 맥락

Windows에서 RunDev를 장시간 실행한 뒤 SQLite `database is locked` 경고와
`STATUS_HEAP_CORRUPTION` 종료가 관찰됐다. 두 현상은 시간상 인접했지만 SQLite
오류는 안전한 오류 반환이고, 힙 손상은 네이티브 메모리 계약 위반에서 발생하므로
별도 원인으로 조사했다.

Windows와 macOS 모두 덮개를 닫거나 시스템을 절전한 뒤 앱이 종료되는 사례도
관찰됐다. Tokio interval의 기본 missed-tick 정책은 `Burst`라서 절전 중 놓친 트레이
애니메이션과 키보드 집계 tick을 복귀 직후 연속 실행한다. 또한 호스트 메트릭과
배터리 수집기는 절전 전에 만든 플랫폼 리소스를 복귀 후에도 계속 사용했다.

기존 DB 연결은 기본 rollback journal과 SQLx의 다중 연결 pool을 사용했다. 집중시간,
키보드 횟수, AI 사용량 동기화가 각자 쓰기를 수행하면서 앱 내부 연결끼리 하나뿐인
SQLite writer 자리를 경쟁했다.

Windows 키보드 수집은 `GetRawInputData` 출력에 정렬을 보장하지 않는 `Vec<u8>`를
사용하고, 전경 `WM_INPUT`도 `DefWindowProcW` 정리 경로로 전달하지 않았다. 같은
물리 입력을 Raw Input과 저수준 키보드 훅으로 중복 수집하기도 했다.

## 결정

- RunDev DB를 WAL과 `synchronous=NORMAL`로 연다.
- 앱의 SQLx pool은 연결 하나만 유지해 내부 읽기와 쓰기를 직렬화한다.
- busy timeout은 10초로 두어 외부의 짧은 writer와도 협력한다.
- Windows Raw Input은 정렬된 `MaybeUninit<RAWINPUT>`에 직접 수신한다.
- `RIM_INPUT`은 처리 후 `DefWindowProcW`로 전달한다.
- 중복된 `WH_KEYBOARD_LL` 훅은 제거하고 물리 키 집계는 Raw Input 하나로 소유한다.
- 트레이와 키보드 interval은 놓친 tick을 `Skip`한다.
- 30초 이상의 wall-clock 공백 후 호스트 메트릭 플랫폼 소스를 다시 만든다.
- 배터리 manager는 매 샘플마다 새로 만들어 절전 전 네이티브 핸들을 보존하지 않는다.

## 결과

### 장점

- 앱 내부 writer 경쟁으로 발생하는 SQLite 잠금을 제거한다.
- 외부 reader가 열린 동안에도 WAL을 통해 RunDev 기록을 계속할 수 있다.
- Windows 입력 처리의 미정의 동작과 중복 네이티브 훅을 제거한다.
- 절전 복귀 직후 네이티브 트레이 갱신 폭주와 오래된 시스템 핸들 재사용을 막는다.
- 키 값이나 순서를 추가로 수집하지 않아 기존 개인정보 경계를 유지한다.

### 단점

- 하나의 DB 연결을 공유하므로 긴 쿼리가 있으면 다른 작업이 순서대로 기다린다.
- WAL의 `-wal`, `-shm` 보조 파일이 앱 데이터 디렉터리에 생성된다.
- 외부 writer가 10초 이상 잠금을 점유하면 여전히 `SQLITE_BUSY`가 발생할 수 있다.

## 대안

- 다중 연결을 유지하고 모든 write 호출 앞에 별도 mutex를 두는 방안을 검토했다.
  읽기 트랜잭션과 신규 write 경로가 mutex를 우회할 수 있어 단일 연결보다 취약하다.
- rollback journal에서 timeout만 늘리는 방안을 검토했다. 경쟁을 숨길 뿐 읽기와 쓰기
  차단 구조를 해결하지 못해 제외했다.
- 저수준 키보드 훅만 유지하는 방안을 검토했다. Raw Input이 이미 백그라운드 물리 키
  집계에 충분하고 전역 훅보다 좁은 수집 경계라 제외했다.
