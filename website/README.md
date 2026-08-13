# RunDev website

RunDev 데스크톱 앱과 빌드 경계를 공유하지 않는 GitHub Pages 전용 정적 사이트다.

```powershell
npm.cmd ci --prefix website
npm.cmd run dev --prefix website
npm.cmd run build --prefix website
```

- 홈페이지 소스: `website/src/`
- 홈페이지 공개 에셋: `website/public/`
- 홈페이지 빌드 결과: `website/dist/`
- 배포: `.github/workflows/pages.yml`

루트의 `npm.cmd run build`와 Tauri 빌드는 이 디렉터리를 읽지 않는다. 반대로
홈페이지는 루트 `src/`, `src-tauri/`, `docs/`를 import하거나 빌드 입력으로 사용하지
않는다. 제품 이미지가 바뀌면 필요한 파일만 `website/public/assets/`에 명시적으로
복사한다.

GitHub 프로젝트 Pages 기본 경로는 `/rundev/`다. 사용자·조직 Pages 또는 사용자
도메인으로 옮길 때는 배포 워크플로의 `SITE_BASE_PATH`를 `/`로 변경한다.
