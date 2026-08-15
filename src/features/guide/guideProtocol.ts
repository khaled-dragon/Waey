import type { CaptureRect, GuideCompletion, GuideResponse, GuideStep, GuideTarget } from "../../shared/types";

const GUIDE_BLOCK_PATTERN = /```waey-guide\s*\r?\n([\s\S]*?)```/gi;
const MAX_CAPTION_LENGTH = 600;

export function extractGuideResponse(content: string): GuideResponse | null {
  const blocks = [...content.matchAll(GUIDE_BLOCK_PATTERN)];

  for (const block of blocks.reverse()) {
    const parsed = parseGuideBlock(block[1] ?? "");

    if (parsed) {
      return parsed;
    }
  }

  return null;
}

export function stripGuideBlocks(content: string) {
  return content.replace(GUIDE_BLOCK_PATTERN, "").replace(/\n{3,}/g, "\n\n").trim();
}

function parseGuideBlock(value: string): GuideResponse | null {
  try {
    const candidate: unknown = JSON.parse(value.trim());

    if (!isRecord(candidate) || typeof candidate.kind !== "string") {
      return null;
    }

    if (candidate.kind === "complete") {
      return parseCompletion(candidate);
    }

    if (candidate.kind === "step") {
      return parseStep(candidate);
    }
  } catch {
    return null;
  }

  return null;
}

function parseCompletion(candidate: Record<string, unknown>): GuideCompletion | null {
  const summary = normalizedText(candidate.summary, MAX_CAPTION_LENGTH);

  return summary ? { kind: "complete", summary } : null;
}

function parseStep(candidate: Record<string, unknown>): GuideStep | null {
  const caption = normalizedText(candidate.caption, MAX_CAPTION_LENGTH);
  const stepIndex = positiveInteger(candidate.stepIndex);
  const estimatedStepsLeft = nonNegativeInteger(candidate.estimatedStepsLeft);

  if (!caption || stepIndex === null || estimatedStepsLeft === null) {
    return null;
  }

  return {
    kind: "step",
    caption,
    target: parseTarget(candidate.target),
    stepIndex,
    estimatedStepsLeft,
  };
}

function parseTarget(value: unknown): GuideTarget | null {
  if (!isRecord(value)) {
    return null;
  }

  const label = normalizedText(value.label, 240);
  const automationId = normalizedText(value.automationId, 240);
  const bounds = parseBounds(value.bounds);

  if (!label && !automationId && !bounds) {
    return null;
  }

  return { label, automationId, bounds };
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

  if (!normalized || normalized.length > maximumLength) {
    return null;
  }

  return normalized;
}

function finiteInteger(value: unknown) {
  return typeof value === "number" && Number.isFinite(value) && Number.isInteger(value) ? value : null;
}

function positiveInteger(value: unknown) {
  const integer = finiteInteger(value);
  return integer !== null && integer > 0 ? integer : null;
}

function nonNegativeInteger(value: unknown) {
  const integer = finiteInteger(value);
  return integer !== null && integer >= 0 ? integer : null;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}
