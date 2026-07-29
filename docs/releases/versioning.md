# 버전과 릴리스 관리

## Semantic Versioning

RunDev는 `MAJOR.MINOR.PATCH`를 사용한다.

`1.0.0` 이전:

- `0.MINOR.0`: 의미 있는 기능, 데이터 구조 또는 UX 변경
- `0.MINOR.PATCH`: 호환 가능한 버그 수정과 작은 개선

`1.0.0` 이후:

- `MAJOR`: 호환되지 않는 변경
- `MINOR`: 호환 가능한 기능
- `PATCH`: 호환 가능한 수정

DB migration 번호는 앱 버전과 별개로 단조 증가하며 기존 migration을 수정하지 않는다.

## 버전 원본

다음 네 파일은 반드시 같은 버전을 가져야 한다.

- `package.json`
- `package-lock.json`
- `src-tauri/Cargo.toml`
- `src-tauri/tauri.conf.json`

직접 각각 수정하지 않고 다음 명령을 사용한다.

```powershell
npm.cmd run version:set -- 0.2.0
npm.cmd run version:check
```

## 브랜치와 태그

- `main`은 항상 빌드 가능한 상태를 유지한다.
- 기능은 `feat/<scope>`, 수정은 `fix/<scope>`처럼 짧은 브랜치에서 작업할 수 있다.
- 별도 `develop` 브랜치는 두지 않는다.
- 배포할 커밋에만 `v0.2.0` 형태의 annotated tag를 생성한다.
- 태그 버전과 앱 버전이 다르면 릴리스 워크플로가 실패한다.

## 릴리스 순서

1. `CHANGELOG.md`의 `Unreleased`를 새 버전 섹션으로 이동한다.
2. `npm.cmd run version:set -- X.Y.Z`를 실행한다.
3. 전체 검증을 실행한다.
4. `chore(release): vX.Y.Z 준비` 커밋을 만든다.
5. `git tag -a vX.Y.Z -m "RunDev vX.Y.Z"`를 생성하고 푸시한다.
6. 태그 워크플로의 Windows installer와 Apple Silicon/Intel macOS DMG를 확인한다.

## 빌드 종류

- `main` macOS build: 최신 개발 상태를 확인하는 임시 artifact
- `v*` tag build: GitHub Release에 연결되는 버전 고정 테스트 배포물

현재 인증서와 notarization이 없으므로 태그 릴리스도 외부 공개용 정식 배포가 아니다.
서명 체계를 갖춘 뒤 별도 ADR로 안정 릴리스 정책을 확정한다.
