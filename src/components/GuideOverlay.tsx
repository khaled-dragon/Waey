import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { useEffect, useState, type PointerEvent } from "react";
import { completeGuideStep, cancelGuideStep, requestGuideAdjustment, startGuideOffer } from "../features/guide/guideCommands";
import type { GuideOverlayRequest } from "../shared/types";
import { OctopusMascot } from "./OctopusMascot";

export function GuideOverlay() {
  const [guide, setGuide] = useState<GuideOverlayRequest | null>(null);
  const guideWindow = getCurrentWindow();

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

  const direction = guide.mode === "step" && guide.target?.bounds ? "targeted" : "thinking";
  const estimatedTotal = guide.stepIndex + guide.estimatedStepsLeft;
  const progress = guide.mode === "thinking"
    ? (guide.isRtl ? "جارٍ التحقق من الخطوة" : "Checking the next step")
    : guide.mode === "offer"
      ? (guide.isRtl ? `حوالي ${estimatedTotal} خطوات` : `About ${estimatedTotal} steps`)
      : guide.isRtl
    ? (guide.estimatedStepsLeft > 0 ? `الخطوة ${guide.stepIndex} من حوالي ${estimatedTotal}` : `الخطوة ${guide.stepIndex}`)
    : (guide.estimatedStepsLeft > 0 ? `Step ${guide.stepIndex} of about ${estimatedTotal}` : `Step ${guide.stepIndex}`);
  const cancelLabel = guide.isRtl ? "إلغاء" : "Cancel";
  const adjustmentLabel = guide.isRtl ? "شيء آخر" : "Something else";
  const primaryLabel = guide.mode === "offer"
    ? (guide.isRtl ? "ابدأ الإرشاد" : "Start guide")
    : (guide.isRtl ? "تم" : "I did it");
  const targetLabel = guide.target?.label?.trim();

  function beginDragging(event: PointerEvent<HTMLElement>) {
    const target = event.target instanceof Element ? event.target : null;

    if (event.button !== 0 || target?.closest("button")) {
      return;
    }

    void guideWindow.startDragging();
  }

  return (
    <div className={`guide-overlay-stage guide-overlay-stage--${guide.theme}`} dir={guide.isRtl ? "rtl" : "ltr"}>
      <section
        className={`guide-popover guide-popover--${direction}`}
        aria-live="polite"
        onPointerDownCapture={beginDragging}
        title={guide.isRtl ? "اسحب لتحريك المرشد" : "Drag to move the guide"}
      >
        <div
          aria-hidden="true"
          className="guide-popover-drag-handle"
          onPointerDown={beginDragging}
        />
        <div className="guide-popover-mascot">
          <OctopusMascot size={48} state={direction === "thinking" ? "thinking" : "idle"} />
        </div>
        <div className="guide-popover-copy">
          <div className="guide-popover-progress">{progress}</div>
          <div className="guide-popover-caption">{guide.caption}</div>
          {targetLabel && guide.mode === "step" && <div className="guide-popover-target">{targetLabel}</div>}
        </div>
        <div className="guide-popover-actions">
          <button className="guide-action guide-action--quiet" onClick={() => void cancelGuideStep()} type="button">{cancelLabel}</button>
          {guide.mode === "step" && (
            <button className="guide-action guide-action--quiet" onClick={() => void requestGuideAdjustment()} type="button">{adjustmentLabel}</button>
          )}
          {guide.mode !== "thinking" && (
            <button
              className="guide-action guide-action--confirm"
              onClick={() => void (guide.mode === "offer" ? startGuideOffer() : completeGuideStep())}
              type="button"
            >
              {primaryLabel}
            </button>
          )}
        </div>
      </section>
    </div>
  );
}
