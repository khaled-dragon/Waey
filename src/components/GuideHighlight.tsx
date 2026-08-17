import { listen } from "@tauri-apps/api/event";
import { useEffect, useState } from "react";
import type { GuideTheme } from "../shared/types";

export function GuideHighlight() {
  const [theme, setTheme] = useState<GuideTheme>("dark");

  useEffect(() => {
    const listener = listen<GuideTheme>("guide-highlight-target", (event) => {
      setTheme(event.payload);
    });

    return () => {
      void listener.then((unlisten) => unlisten());
    };
  }, []);

  return (
    <div className={`guide-highlight guide-highlight--${theme}`} aria-hidden="true">
      <div className="guide-highlight-ring" />
    </div>
  );
}
