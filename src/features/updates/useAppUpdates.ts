import { useCallback, useEffect, useRef, useState } from "react";
import { getVersion } from "@tauri-apps/api/app";
import { check, type DownloadEvent, type Update } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";

type UpdateStatus = "idle" | "checking" | "available" | "notAvailable" | "downloading" | "installing" | "error";

export interface AppUpdateState {
  body?: string;
  currentVersion: string;
  downloadedBytes: number;
  errorMessage: string | null;
  latestVersion: string | null;
  status: UpdateStatus;
  totalBytes: number | null;
}

const initialUpdateState: AppUpdateState = {
  currentVersion: "",
  downloadedBytes: 0,
  errorMessage: null,
  latestVersion: null,
  status: "idle",
  totalBytes: null,
};

export function useAppUpdates() {
  const pendingUpdateRef = useRef<Update | null>(null);
  const hasCheckedAutomatically = useRef(false);
  const [updateState, setUpdateState] = useState<AppUpdateState>(initialUpdateState);

  const checkForUpdate = useCallback(async (isManual = false) => {
    setUpdateState((state) => ({
      ...state,
      errorMessage: null,
      status: "checking",
    }));

    try {
      const [currentVersion, update] = await Promise.all([
        getVersion(),
        check({ timeout: 8000 }),
      ]);

      pendingUpdateRef.current = update;

      if (!update) {
        setUpdateState({
          ...initialUpdateState,
          currentVersion,
          status: isManual ? "notAvailable" : "idle",
        });
        return;
      }

      setUpdateState({
        ...initialUpdateState,
        body: update.body,
        currentVersion,
        latestVersion: update.version,
        status: "available",
      });
    } catch (error) {
      pendingUpdateRef.current = null;
      setUpdateState((state) => ({
        ...state,
        errorMessage: error instanceof Error ? error.message : String(error),
        status: isManual ? "error" : "idle",
      }));
    }
  }, []);

  const installUpdate = useCallback(async () => {
    const pendingUpdate = pendingUpdateRef.current;

    if (!pendingUpdate) {
      await checkForUpdate(true);
      return;
    }

    setUpdateState((state) => ({
      ...state,
      downloadedBytes: 0,
      errorMessage: null,
      status: "downloading",
      totalBytes: null,
    }));

    try {
      await pendingUpdate.downloadAndInstall((event: DownloadEvent) => {
        setUpdateState((state) => reduceDownloadEvent(state, event));
      });

      setUpdateState((state) => ({
        ...state,
        status: "installing",
      }));

      await relaunch();
    } catch (error) {
      setUpdateState((state) => ({
        ...state,
        errorMessage: error instanceof Error ? error.message : String(error),
        status: "error",
      }));
    }
  }, [checkForUpdate]);

  const dismissUpdate = useCallback(() => {
    setUpdateState((state) => ({
      ...state,
      status: "idle",
    }));
  }, []);

  useEffect(() => {
    if (hasCheckedAutomatically.current) return;

    hasCheckedAutomatically.current = true;
    window.setTimeout(() => {
      void checkForUpdate(false);
    }, 1800);
  }, [checkForUpdate]);

  return {
    checkForUpdate,
    dismissUpdate,
    installUpdate,
    updateState,
  };
}

function reduceDownloadEvent(state: AppUpdateState, event: DownloadEvent): AppUpdateState {
  if (event.event === "Started") {
    return {
      ...state,
      downloadedBytes: 0,
      status: "downloading",
      totalBytes: event.data.contentLength ?? null,
    };
  }

  if (event.event === "Progress") {
    return {
      ...state,
      downloadedBytes: state.downloadedBytes + event.data.chunkLength,
      status: "downloading",
    };
  }

  return {
    ...state,
    status: "installing",
  };
}
