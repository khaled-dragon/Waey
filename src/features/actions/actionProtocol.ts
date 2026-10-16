import type { CaptureRect, DesktopAction, DesktopActionRole, DesktopActionTarget } from "../../shared/types";

const ACTION_MARKER_PATTERN = /<!--\s*WAEY_ACTION\s*:\s*([\s\S]*?)-->/gi;
const MAX_SUMMARY_LENGTH = 240;
const MAX_TARGET_TEXT_LENGTH = 240;
const SAFE_ROLES = new Set<DesktopActionRole>(["Button", "MenuItem", "Hyperlink"]);

export function extractDesktopAction(content: string): DesktopAction | null {
  const markers = [...content.matchAll(ACTION_MARKER_PATTERN)];

  for (const marker of markers.reverse()) {
    const action = parseDesktopAction(marker[1] ?? "");

    if (action) {
      return action;
    }
  }

  return null;
}

export function stripDesktopActionBlocks(content: string) {
  return content.replace(ACTION_MARKER_PATTERN, "").replace(/\n{3,}/g, "\n\n").trim();
}

function parseDesktopAction(value: string): DesktopAction | null {
  try {
    const candidate: unknown = JSON.parse(value.trim());

    if (!isRecord(candidate) || candidate.kind !== "click") {
      return null;
    }

    const summary = normalizedText(candidate.summary, MAX_SUMMARY_LENGTH);
    const target = parseTarget(candidate.target);

    return summary && target ? { kind: "click", summary, target } : null;
  } catch {
    return null;
  }
}

function parseTarget(value: unknown): DesktopActionTarget | null {
  if (!isRecord(value) || !SAFE_ROLES.has(value.role as DesktopActionRole)) {
    return null;
  }

  const label = normalizedText(value.label, MAX_TARGET_TEXT_LENGTH);
  const automationId = normalizedText(value.automationId, MAX_TARGET_TEXT_LENGTH);
  const bounds = parseBounds(value.bounds);

  if ((!label && !automationId) || !bounds) {
    return null;
  }

  return {
    role: value.role as DesktopActionRole,
    label,
    automationId,
    bounds,
  };
}

function parseBounds(value: unknown): CaptureRect | null {
  if (!isRecord(value)) {
    return null;
  }

  const x = finiteInteger(value.x);
  const y = finiteInteger(value.y);
  const width = positiveInteger(value.width);
  const height = positiveInteger(value.height);

  if (x === null || y === null || width === null || height === null || width > 10_000 || height > 10_000) {
    return null;
  }

  return { x, y, width, height };
}

function normalizedText(value: unknown, maximumLength: number) {
  if (typeof value !== "string") {
    return null;
  }

  const normalized = value.trim();
  return normalized.length > 0 && normalized.length <= maximumLength ? normalized : null;
}

function finiteInteger(value: unknown) {
  return typeof value === "number" && Number.isFinite(value) && Number.isInteger(value) ? value : null;
}

function positiveInteger(value: unknown) {
  const integer = finiteInteger(value);
  return integer !== null && integer > 0 ? integer : null;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}
