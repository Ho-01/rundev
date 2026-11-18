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

### 한시적인 개발 의존성 보안 예외

CI에서는 `npm run audit:check`를 실행한다. 이 명령은 정책 회귀 테스트 후 배포
의존성(`npm audit --omit=dev`)과 전체 의존성(`npm audit`)을 각각 검사한다.
배포 의존성의 취약점과 새 보안 공지는 계속 릴리스를 차단한다.

2026-11-18 사용자 승인에 따라 개발 도구 Vite/Vitest가 참조하는 picomatch의
[GHSA-fc5q-39xc-vjpf](https://github.com/advisories/GHSA-fc5q-39xc-vjpf)만
2026-12-18 00:00 UTC 전까지 예외로 허용한다. 승인 시점에는 공식 수정 버전이 없다.
공격자가 제공한 반복 extglob 패턴 처리 시 ReDoS가 발생할 수 있으며, RunDev는
해당 도구에 사용자 제공 glob 패턴을 전달하지 않는다. 빌드·테스트 의존성으로만
사용하며 배포 번들에서는 실행하지 않는다. 개발 서버를 외부에 공개하지 않는다.

예외 검사는 package-lock의 모든 관련 설치 경로가 `dev: true`인지 검증하고
해당 공지만 원인인 의존성 전파에 한해서 허용한다. 다른 공지나 불명확한 검사
결과는 실패한다. 수정 버전이 나오면 업데이트하고 예외를 제거하며, 만료 후에는
자동으로 실패하므로 재검토 없이 기한을 연장하지 않는다.

릴리스 본문은 태그 버전에 해당하는 `CHANGELOG.md` 섹션에서 자동 생성된다.
버전 섹션이 없거나 비어 있으면 릴리스 생성을 실패시킨다.

## 빌드 종류

- `main` macOS build: 최신 개발 상태를 확인하는 임시 artifact
- `v*` tag build: GitHub Release에 연결되는 버전 고정 배포물과 updater 아티팩트

현재 macOS 코드 서명과 notarization이 없으므로 태그 릴리스도 Gatekeeper 경고가
남을 수 있다. Updater 서명(변조 방지)과 OS 코드 서명은 별개다. OS 서명 체계를
갖춘 뒤 별도 ADR로 안정 릴리스 정책을 확정한다.

## Updater 서명 키와 bootstrap

릴리스 빌드는 Tauri updater 비밀키로 `.sig`를 만들고 `tauri-action`이
`latest.json`을 Release에 병합한다.

macOS 빌드는 `app,dmg` 번들을 함께 생성한다. DMG는 수동 설치용이며, `app`
번들에서 생성되는 `.app.tar.gz`와 `.sig`가 자동 업데이트에 사용된다.
`latest.json`에는 `darwin-aarch64`와 `darwin-x86_64`가 모두 포함되어야 한다.

GitHub repository secrets:

- `TAURI_SIGNING_PRIVATE_KEY`: `tauri signer generate`로 만든 비밀키 전체
- `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`: 키에 비밀번호를 걸었다면 함께 등록

공개키만 `src-tauri/tauri.conf.json`의 `plugins.updater.pubkey`에 커밋한다.
비밀키는 저장소에 넣지 않는다.

### Updater bootstrap release

Updater 기능이 처음 들어간 버전은 bootstrap release다. 그 이전 설치본에는
업데이트 확인 코드가 없으므로 자동으로 올라가지 않는다. 해당 버전을 한 번
수동 설치한 뒤부터 다음 태그 릴리스를 자동 감지할 수 있다.

앱은 시작 시·15분마다·정보 창에서 업데이트를 확인하고, 설치는 사용자가
“다운로드 및 재시작”을 누를 때만 수행한다.
