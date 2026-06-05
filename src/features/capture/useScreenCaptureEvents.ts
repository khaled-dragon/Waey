import { listen } from "@tauri-apps/api/event";
import { useEffect, useState } from "react";
import type { ScreenCapture, ScreenCaptureError } from "../../shared/types";

const CAPTURE_READY_EVENT = "capture-ready";
const CAPTURE_ERROR_EVENT = "capture-error";

export function useScreenCaptureEvents() {
  const [latestCapture, setLatestCapture] = useState<ScreenCapture | null>(null);
  const [captureError, setCaptureError] = useState<string | null>(null);

  useEffect(() => {
    let isMounted = true;
    let removeListeners: (() => void)[] = [];

    const pendingListeners = [
      listen<ScreenCapture>(CAPTURE_READY_EVENT, (event) => {
        if (isMounted) {
          setLatestCapture(event.payload);
          setCaptureError(null);
        }
      }),
      listen<ScreenCaptureError>(CAPTURE_ERROR_EVENT, (event) => {
        if (isMounted) {
          setCaptureError(event.payload.message);
        }
      }),
    ];

    void Promise.all(pendingListeners).then((listeners) => {
      if (!isMounted) {
        listeners.forEach((unlisten) => unlisten());
        return;
      }

      removeListeners = listeners;
    });

    return () => {
      isMounted = false;
      removeListeners.forEach((unlisten) => unlisten());
    };
  }, []);

  return { captureError, latestCapture, setCaptureError, setLatestCapture };
}
