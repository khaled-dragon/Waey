import { listen } from "@tauri-apps/api/event";
import { useCallback, useEffect, useState } from "react";
import type { ScreenCapture, ScreenCaptureError } from "../../shared/types";

const CAPTURE_READY_EVENT = "capture-ready";
const CAPTURE_ERROR_EVENT = "capture-error";
const MAX_SCREEN_CAPTURES = 3;

export function useScreenCaptureEvents() {
  const [captures, setCaptures] = useState<ScreenCapture[]>([]);
  const [latestCapture, setLatestCapture] = useState<ScreenCapture | null>(null);
  const [captureError, setCaptureError] = useState<string | null>(null);

  useEffect(() => {
    let isMounted = true;
    let removeListeners: (() => void)[] = [];

    const pendingListeners = [
      listen<ScreenCapture>(CAPTURE_READY_EVENT, (event) => {
        if (isMounted) {
          setCaptures((currentCaptures) => {
            const nextCaptures = [...currentCaptures, event.payload].slice(-MAX_SCREEN_CAPTURES);
            setLatestCapture(nextCaptures[nextCaptures.length - 1] ?? null);
            return nextCaptures;
          });
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

  const removeCapture = useCallback((path: string) => {
    setCaptures((currentCaptures) => {
      const nextCaptures = currentCaptures.filter((capture) => capture.path !== path);
      setLatestCapture(nextCaptures[nextCaptures.length - 1] ?? null);
      return nextCaptures;
    });
  }, []);

  const clearCaptures = useCallback(() => {
    setCaptures([]);
    setLatestCapture(null);
  }, []);

  return { captureError, captures, clearCaptures, latestCapture, removeCapture, setCaptureError, setCaptures, setLatestCapture };
}
