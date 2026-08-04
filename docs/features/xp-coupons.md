# 경험치 쿠폰

## 사용자 흐름

1. RunDev 상단의 정보 버튼을 연다.
2. `쿠폰 입력`을 눌러 쿠폰 번호를 붙여 넣는다.
3. 앱이 서명, 배수, 적용 시간과 등록 기한을 로컬에서 확인한다.
4. 사용자가 확인하면 부스트를 적용하거나 기존 예약 뒤에 추가한다.
5. 활성 부스트의 배수와 남은 시간은 상단에 표시한다.

앱은 쿠폰 원문을 저장하지 않고 쿠폰 ID, 배수, 등록 시각과 적용 구간만 저장한다.

## 운영 키 생성

비밀키는 저장소 바깥의 백업 가능한 안전한 경로에 생성한다.

```powershell
npm.cmd run coupon -- keygen --private D:\secure\rundev-coupon-private.pem
```

출력된 `RUNDEV_COUPON_PUBLIC_KEY` 값만 GitHub 저장소의 Actions 변수에 등록한다.
비밀키 파일은 GitHub, RunDev 저장소 또는 배포 파일에 올리지 않는다.

## 쿠폰 발급

```powershell
npm.cmd run coupon -- issue `
  --private D:\secure\rundev-coupon-private.pem `
  --id launch-2026-user-001 `
  --multiplier 2 `
  --minutes 120 `
  --redeem-before 2026-12-31
```

쿠폰 ID는 재사용하지 않는다. 지원 범위는 2배 또는 3배, 최대 30일이다.

## 한계

서버가 없으므로 사용자가 로컬 DB를 삭제하거나 다른 기기로 옮겨 같은 쿠폰을 다시
사용하는 것은 완전히 차단할 수 없다. 현금성 상품이나 계정 단위 1회 사용이 필요하면
별도 서버 검증 결정이 필요하다.
