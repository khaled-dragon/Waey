import { listen } from "@tauri-apps/api/event";
import { useEffect, useState } from "react";
import { completeGuideStep, cancelGuideStep } from "../features/guide/guideCommands";
import type { GuideOverlayRequest } from "../shared/types";
import { OctopusMascot } from "./OctopusMascot";

export function GuideOverlay() {
  const [guide, setGuide] = useState<GuideOverlayRequest | null>(null);

  useEffect(() => {
    let isMounted = true;

    const listener = listen<GuideOverlayRequest>("guide-overlay-step", (event) => {
      if (isMounted) {
        setGuide(event.payload);
      }
    });

    return () => {
      isMounted = false;
      void listener.then((unlisten) => unlisten());
    };
  }, []);

  if (!guide) {
    return <div className="guide-overlay-stage" />;
  }

  const direction = guide.target?.bounds ? "targeted" : "thinking";
  const estimatedTotal = guide.stepIndex + guide.estimatedStepsLeft;
  const progress = guide.isRtl
    ? (guide.estimatedStepsLeft > 0 ? `الخطوة ${guide.stepIndex} من حوالي ${estimatedTotal}` : `الخطوة ${guide.stepIndex}`)
    : (guide.estimatedStepsLeft > 0 ? `Step ${guide.stepIndex} of about ${estimatedTotal}` : `Step ${guide.stepIndex}`);
  const cancelLabel = guide.isRtl ? "إلغاء" : "Cancel";
  const doneLabel = guide.isRtl ? "تم" : "Done";

  return (
    <div className={`guide-overlay-stage guide-overlay-stage--${guide.theme}`} dir={guide.isRtl ? "rtl" : "ltr"}>
      <section className={`guide-popover guide-popover--${direction}`} aria-live="polite">
        <div className="guide-popover-mascot">
          <OctopusMascot size={48} state={direction === "thinking" ? "thinking" : "idle"} />
        </div>
        <div className="guide-popover-copy">
          <div className="guide-popover-progress">{progress}</div>
          <div className="guide-popover-caption">{guide.caption}</div>
        </div>
        <div className="guide-popover-actions">
          <button className="guide-action guide-action--quiet" onClick={() => void cancelGuideStep()} type="button">{cancelLabel}</button>
          <button className="guide-action guide-action--confirm" onClick={() => void completeGuideStep()} type="button">{doneLabel}</button>
        </div>
      </section>
    </div>
  );
}
