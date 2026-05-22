import { useEffect, useMemo, useState, type PointerEvent } from "react";
import { cancelRegionSelection, captureSelectedRegion } from "./captureCommands";
import type { CaptureRect } from "../../shared/types";

interface SelectionPoint { x: number; y: number; }
interface SelectionBox extends CaptureRect { isReady: boolean; }

export function RegionSelector() {
  const [startPoint, setStartPoint] = useState<SelectionPoint | null>(null);
  const [currentPoint, setCurrentPoint] = useState<SelectionPoint | null>(null);
  const selectionBox = useMemo(() => buildSelectionBox(startPoint, currentPoint), [currentPoint, startPoint]);

  useEffect(() => {
    function handleKeyDown(event: KeyboardEvent) {
      if (event.key === "Escape") { event.preventDefault(); void cancelRegionSelection(); }
    }
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, []);

  function handlePointerDown(event: PointerEvent<HTMLDivElement>) {
    const point = { x: Math.round(event.clientX), y: Math.round(event.clientY) };
    event.currentTarget.setPointerCapture(event.pointerId);
    setStartPoint(point); setCurrentPoint(point);
  }

  function handlePointerMove(event: PointerEvent<HTMLDivElement>) {
    if (!startPoint) return;
    setCurrentPoint({ x: Math.round(event.clientX), y: Math.round(event.clientY) });
  }

  function handlePointerUp(event: PointerEvent<HTMLDivElement>) {
    event.currentTarget.releasePointerCapture(event.pointerId);
    if (selectionBox?.isReady) {
      const { x, y, width, height } = selectionBox;
      void captureSelectedRegion({ x, y, width, height });
    }
    setStartPoint(null); setCurrentPoint(null);
  }

  return (
    <div className="region-overlay" onPointerDown={handlePointerDown} onPointerMove={handlePointerMove} onPointerUp={handlePointerUp}>
      <div className="region-hint">Drag to select region · Esc to cancel</div>
      {selectionBox && (
        <div className="region-selection" style={{ left: selectionBox.x, top: selectionBox.y, width: selectionBox.width, height: selectionBox.height }}>
          <div className="region-size">{selectionBox.width} × {selectionBox.height}</div>
        </div>
      )}
    </div>
  );
}

function buildSelectionBox(startPoint: SelectionPoint | null, currentPoint: SelectionPoint | null): SelectionBox | null {
  if (!startPoint || !currentPoint) return null;
  const x = Math.min(startPoint.x, currentPoint.x);
  const y = Math.min(startPoint.y, currentPoint.y);
  const width = Math.abs(currentPoint.x - startPoint.x);
  const height = Math.abs(currentPoint.y - startPoint.y);
  return { x, y, width, height, isReady: width >= 12 && height >= 12 };
}
