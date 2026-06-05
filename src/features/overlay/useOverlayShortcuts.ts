import { useEffect } from "react";
import { hideOverlayWindow } from "./overlayCommands";

export function useOverlayShortcuts() {
  useEffect(() => {
    function handleOverlayKeyDown(event: KeyboardEvent) {
      if (event.key !== "Escape") {
        return;
      }

      event.preventDefault();
      void hideOverlayWindow();
    }

    window.addEventListener("keydown", handleOverlayKeyDown);

    return () => {
      window.removeEventListener("keydown", handleOverlayKeyDown);
    };
  }, []);
}
