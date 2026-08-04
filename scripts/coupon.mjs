import { generateKeyPairSync, sign } from "node:crypto";
import { readFileSync, writeFileSync } from "node:fs";
import { resolve } from "node:path";

const [, , command, ...args] = process.argv;

function option(name) {
  const index = args.indexOf(`--${name}`);
  return index >= 0 ? args[index + 1] : undefined;
}

function required(name) {
  const value = option(name);
  if (!value) throw new Error(`--${name} 값이 필요합니다.`);
  return value;
}

function base64Url(value) {
  return Buffer.from(value).toString("base64url");
}

if (command === "keygen") {
  const privatePath = resolve(required("private"));
  const { privateKey, publicKey } = generateKeyPairSync("ed25519");
  writeFileSync(privatePath, privateKey.export({ type: "pkcs8", format: "pem" }), {
    encoding: "utf8",
    mode: 0o600,
    flag: "wx"
  });
  const publicDer = publicKey.export({ type: "spki", format: "der" });
  const rawPublicKey = publicDer.subarray(publicDer.length - 32);
  console.log(`private key: ${privatePath}`);
  console.log(`RUNDEV_COUPON_PUBLIC_KEY=${base64Url(rawPublicKey)}`);
} else if (command === "issue") {
  const multiplier = Number(required("multiplier"));
  const durationMinutes = Number(required("minutes"));
  const redeemBefore = new Date(required("redeem-before"));
  if (![2, 3].includes(multiplier)) throw new Error("--multiplier는 2 또는 3이어야 합니다.");
  if (!Number.isInteger(durationMinutes) || durationMinutes < 1 || durationMinutes > 43_200) {
    throw new Error("--minutes는 1~43200 범위의 정수여야 합니다.");
  }
  if (Number.isNaN(redeemBefore.getTime())) throw new Error("--redeem-before 날짜가 올바르지 않습니다.");
  const payload = {
    couponId: required("id"),
    multiplier,
    durationMinutes,
    redeemBefore: redeemBefore.toISOString()
  };
  const encodedPayload = base64Url(JSON.stringify(payload));
  const privateKey = readFileSync(resolve(required("private")), "utf8");
  const signature = sign(null, Buffer.from(encodedPayload), privateKey);
  console.log(`RDC1.${encodedPayload}.${base64Url(signature)}`);
} else {
  console.error("사용법:");
  console.error("  npm run coupon -- keygen --private <안전한-경로.pem>");
  console.error("  npm run coupon -- issue --private <키.pem> --id <ID> --multiplier 2 --minutes 120 --redeem-before 2026-12-31");
  process.exitCode = 1;
}
