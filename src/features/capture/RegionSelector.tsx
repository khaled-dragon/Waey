import { useEffect, useMemo, useState, type PointerEvent } from "react";
import { cancelRegionSelection, captureSelectedRegion } from "./captureCommands";
import type { CaptureRect } from "../../shared/types";

interface SelectionPoint {
  x: number;
  y: number;
}

interface SelectionBox extends CaptureRect {
  isReady: boolean;
}

export function RegionSelector() {
  const [startPoint, setStartPoint] = useState<SelectionPoint | null>(null);
  const [currentPoint, setCurrentPoint] = useState<SelectionPoint | null>(null);

  const selectionBox = useMemo(
    () => buildSelectionBox(startPoint, currentPoint),
    [currentPoint, startPoint],
  );

  useEffect(() => {
    function handleKeyDown(event: KeyboardEvent) {
      if (event.key === "Escape") {
        event.preventDefault();
        void cancelRegionSelection();
      }
    }

    window.addEventListener("keydown", handleKeyDown);

    return () => {
      window.removeEventListener("keydown", handleKeyDown);
    };
  }, []);

  function handlePointerDown(event: PointerEvent<HTMLDivElement>) {
    const point = { x: Math.round(event.clientX), y: Math.round(event.clientY) };

    event.currentTarget.setPointerCapture(event.pointerId);
    setStartPoint(point);
    setCurrentPoint(point);
  }

  function handlePointerMove(event: PointerEvent<HTMLDivElement>) {
    if (!startPoint) {
      return;
    }

    setCurrentPoint({ x: Math.round(event.clientX), y: Math.round(event.clientY) });
  }

  function handlePointerUp(event: PointerEvent<HTMLDivElement>) {
    event.currentTarget.releasePointerCapture(event.pointerId);

    if (selectionBox?.isReady) {
      const { x, y, width, height } = selectionBox;

      void captureSelectedRegion({ x, y, width, height });
    }

    setStartPoint(null);
    setCurrentPoint(null);
  }

  return (
    <main
      className="relative h-screen w-screen cursor-crosshair overflow-hidden bg-black/35 text-white"
      onPointerDown={handlePointerDown}
      onPointerMove={handlePointerMove}
      onPointerUp={handlePointerUp}
    >
      <div className="pointer-events-none absolute left-1/2 top-6 -translate-x-1/2 rounded-full border border-white/15 bg-waey-ink/85 px-5 py-3 text-sm shadow-2xl shadow-black/40 backdrop-blur">
        Drag to Smart Crop. Press Esc to cancel.
      </div>

      {selectionBox ? (
        <div
          className="pointer-events-none absolute border-2 border-waey-coral bg-waey-bright/15 shadow-[0_0_0_9999px_rgba(0,0,0,0.42)]"
          style={{
            height: selectionBox.height,
            left: selectionBox.x,
            top: selectionBox.y,
            width: selectionBox.width,
          }}
        >
          <div className="absolute -top-9 left-0 rounded-full bg-waey-bright px-3 py-1 text-xs font-semibold">
            {selectionBox.width} x {selectionBox.height}
          </div>
        </div>
      ) : null}
    </main>
  );
}

function buildSelectionBox(
  startPoint: SelectionPoint | null,
  currentPoint: SelectionPoint | null,
): SelectionBox | null {
  if (!startPoint || !currentPoint) {
    return null;
  }

  const x = Math.min(startPoint.x, currentPoint.x);
  const y = Math.min(startPoint.y, currentPoint.y);
  const width = Math.abs(currentPoint.x - startPoint.x);
  const height = Math.abs(currentPoint.y - startPoint.y);

  return {
    x,
    y,
    width,
    height,
    isReady: width >= 12 && height >= 12,
  };
}
