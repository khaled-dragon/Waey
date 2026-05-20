import { listen } from "@tauri-apps/api/event";
import { useEffect, useState } from "react";
import type { ScreenCapture } from "../../shared/types";

const CAPTURE_READY_EVENT = "capture-ready";

export function useScreenCaptureEvents() {
  const [latestCapture, setLatestCapture] = useState<ScreenCapture | null>(null);

  useEffect(() => {
    let isMounted = true;
    let removeListener: (() => void) | undefined;

    void listen<ScreenCapture>(CAPTURE_READY_EVENT, (event) => {
      if (isMounted) {
        setLatestCapture(event.payload);
      }
    }).then((unlisten) => {
      if (!isMounted) {
        unlisten();
        return;
      }

      removeListener = unlisten;
    });

    return () => {
      isMounted = false;
      removeListener?.();
    };
  }, []);

  return { latestCapture, setLatestCapture };
}
