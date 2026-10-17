import type {
  CaptureRect,
  DesktopAction,
  DesktopActionRole,
  DesktopActionTarget,
  DesktopScrollAction,
} from "../../shared/types";

const ACTION_MARKER_PATTERN = /<!--\s*WAEY_ACTION\s*:\s*([\s\S]*?)-->/gi;
const MAX_SUMMARY_LENGTH = 240;
const MAX_TARGET_TEXT_LENGTH = 240;
const MAX_ENTRY_TEXT_LENGTH = 2_000;
const CLICK_ROLES = new Set<DesktopActionRole>(["Button", "MenuItem", "Hyperlink"]);
const TEXT_ENTRY_ROLES = new Set<DesktopActionRole>(["Edit", "ComboBox"]);
const SCROLL_ROLES = new Set<DesktopActionRole>(["Document", "List", "DataGrid", "Tree"]);
const SECRET_TERMS = [
  "password", "passcode", "api key", "api_key", "access token", "refresh token", "bearer ",
  "secret", "private key", "ssh-rsa", "كلمة المرور", "كلمه المرور", "رمز المرور",
  "مفتاح api", "مفتاح الوصول", "رمز الوصول", "سر ",
];

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

    if (!isRecord(candidate)) {
      return null;
    }

    const summary = normalizedText(candidate.summary, MAX_SUMMARY_LENGTH);
    if (!summary) {
      return null;
    }

    switch (candidate.kind) {
      case "click": {
        const target = parseTarget(candidate.target, CLICK_ROLES);
        return target ? { kind: "click", summary, target } : null;
      }
      case "setText": {
        const target = parseTarget(candidate.target, TEXT_ENTRY_ROLES);
        const text = exactEntryText(candidate.text);
        return target && text !== null ? { kind: "setText", summary, target, text } : null;
      }
      case "scroll": {
        const target = parseTarget(candidate.target, SCROLL_ROLES);
        const direction = candidate.direction === "up" || candidate.direction === "down"
          ? candidate.direction
          : null;
        return target && direction
          ? ({ kind: "scroll", summary, target, direction } satisfies DesktopScrollAction)
          : null;
      }
      default:
        return null;
    }
  } catch {
    return null;
  }
}

function parseTarget(value: unknown, allowedRoles: Set<DesktopActionRole>): DesktopActionTarget | null {
  if (!isRecord(value) || !allowedRoles.has(value.role as DesktopActionRole)) {
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

function exactEntryText(value: unknown) {
  if (typeof value !== "string" || value.trim().length === 0 || value.length > MAX_ENTRY_TEXT_LENGTH) {
    return null;
  }

  const lowerCaseValue = value.toLocaleLowerCase();
  return value.includes("\0") || SECRET_TERMS.some((term) => lowerCaseValue.includes(term)) ? null : value;
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
