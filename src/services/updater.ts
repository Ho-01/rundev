import { check } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";
import {
  isPermissionGranted,
  requestPermission,
  sendNotification
} from "@tauri-apps/plugin-notification";

export const UPDATE_CHECK_INTERVAL_MS = 15 * 60 * 1000;

const LAST_CHECKED_KEY = "updater.lastCheckedAt";
const AVAILABLE_VERSION_KEY = "updater.availableVersion";
const NOTIFIED_VERSION_KEY = "updater.notifiedVersion";

export type UpdateStatus = {
  available: boolean;
  version: string | null;
  currentVersion: string;
};

function isTauri() {
  return "__TAURI_INTERNALS__" in window;
}

function readCachedAvailableVersion(): string | null {
  return localStorage.getItem(AVAILABLE_VERSION_KEY);
}

function writeCheckResult(availableVersion: string | null) {
  localStorage.setItem(LAST_CHECKED_KEY, String(Date.now()));
  if (availableVersion) {
    localStorage.setItem(AVAILABLE_VERSION_KEY, availableVersion);
  } else {
    localStorage.removeItem(AVAILABLE_VERSION_KEY);
  }
}

export function getCachedUpdateStatus(currentVersion: string): UpdateStatus {
  const version = readCachedAvailableVersion();
  return {
    available: Boolean(version && version !== currentVersion),
    version,
    currentVersion
  };
}

export async function checkForAppUpdate(options?: {
  force?: boolean;
  currentVersion: string;
}): Promise<UpdateStatus> {
  const currentVersion = options?.currentVersion ?? "";
  const force = options?.force ?? false;

  if (!isTauri()) {
    return { available: false, version: null, currentVersion };
  }

  const lastChecked = Number(localStorage.getItem(LAST_CHECKED_KEY) ?? 0);
  if (!force && Date.now() - lastChecked < UPDATE_CHECK_INTERVAL_MS) {
    return getCachedUpdateStatus(currentVersion);
  }

  try {
    const update = await check();
    if (update) {
      writeCheckResult(update.version);
      return {
        available: true,
        version: update.version,
        currentVersion
      };
    }
    writeCheckResult(null);
    return { available: false, version: null, currentVersion };
  } catch {
    return getCachedUpdateStatus(currentVersion);
  }
}

export async function notifyIfUpdateAvailable(version: string): Promise<void> {
  if (!isTauri() || localStorage.getItem(NOTIFIED_VERSION_KEY) === version) {
    return;
  }

  try {
    let granted = await isPermissionGranted();
    if (!granted) {
      granted = (await requestPermission()) === "granted";
    }
    if (!granted) return;

    sendNotification({
      title: "RunDev",
      body: `새 버전 v${version}을 사용할 수 있습니다.`
    });
    localStorage.setItem(NOTIFIED_VERSION_KEY, version);
  } catch {
    // Ignore notification failures; the info dialog still shows the update.
  }
}

export async function downloadAndInstallAppUpdate(): Promise<void> {
  if (!isTauri()) {
    throw new Error("데스크톱 앱에서만 업데이트를 설치할 수 있습니다.");
  }

  const update = await check();
  if (!update) {
    throw new Error("설치할 업데이트가 없습니다.");
  }

  await update.downloadAndInstall();
  localStorage.removeItem(AVAILABLE_VERSION_KEY);
  await relaunch();
}
