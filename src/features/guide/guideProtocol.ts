import type { CaptureRect, GuideCompletion, GuideOffer, GuideResponse, GuideStep, GuideTarget } from "../../shared/types";

const GUIDE_MARKER_PATTERN = /<!--\s*WAEY_GUIDE\s*:\s*([\s\S]*?)-->/gi;
const LEGACY_GUIDE_BLOCK_PATTERN = /```waey-guide\s*\r?\n([\s\S]*?)```/gi;
const MAX_CAPTION_LENGTH = 600;
const MAX_SUMMARY_LENGTH = 240;

export function extractGuideResponse(content: string): GuideResponse | null {
  const markerResponse = extractGuideMarker(content, GUIDE_MARKER_PATTERN);

  if (markerResponse) {
    return markerResponse;
  }

  const legacyResponse = extractGuideMarker(content, LEGACY_GUIDE_BLOCK_PATTERN);

  if (legacyResponse) {
    return legacyResponse;
  }

  return extractLooseGuideBlock(content)?.response ?? null;
}

export function stripGuideBlocks(content: string) {
  const withoutMarkers = content
    .replace(GUIDE_MARKER_PATTERN, "")
    .replace(LEGACY_GUIDE_BLOCK_PATTERN, "");
  const looseGuide = extractLooseGuideBlock(withoutMarkers);

  if (!looseGuide) {
    return normalizeWhitespace(withoutMarkers);
  }

  return normalizeWhitespace(
    `${withoutMarkers.slice(0, looseGuide.start)}${withoutMarkers.slice(looseGuide.end)}`,
  );
}

function extractGuideMarker(content: string, pattern: RegExp): GuideResponse | null {
  const blocks = [...content.matchAll(pattern)];

  for (const block of blocks.reverse()) {
    const parsed = parseGuideBlock(block[1] ?? "");

    if (parsed) {
      return parsed;
    }
  }

  return null;
}

function extractLooseGuideBlock(content: string) {
  for (const candidate of jsonObjectCandidates(content).reverse()) {
    const parsed = parseGuideBlock(candidate.value);

    if (parsed) {
      return { ...candidate, response: parsed };
    }
  }

  return null;
}

function parseGuideBlock(value: string): GuideResponse | null {
  try {
    const candidate: unknown = JSON.parse(value.trim());

    if (!isRecord(candidate) || typeof candidate.kind !== "string") {
      return null;
    }

    if (candidate.kind === "offer") {
      return parseOffer(candidate);
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

function parseOffer(candidate: Record<string, unknown>): GuideOffer | null {
  const summary = normalizedText(candidate.summary, MAX_SUMMARY_LENGTH);
  const estimatedSteps = positiveInteger(candidate.estimatedSteps);
  const firstStep = isRecord(candidate.firstStep) ? parseStep(candidate.firstStep) : null;

  if (!summary || estimatedSteps === null || !firstStep) {
    return null;
  }

  return { kind: "offer", summary, estimatedSteps, firstStep };
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

function jsonObjectCandidates(content: string) {
  const candidates: Array<{ value: string; start: number; end: number }> = [];
  let start = -1;
  let depth = 0;
  let inString = false;
  let escaped = false;

  for (let index = 0; index < content.length; index += 1) {
    const character = content[index];

    if (start === -1) {
      if (character === "{") {
        start = index;
        depth = 1;
      }
      continue;
    }

    if (inString) {
      if (escaped) {
        escaped = false;
      } else if (character === "\\") {
        escaped = true;
      } else if (character === '"') {
        inString = false;
      }
      continue;
    }

    if (character === '"') {
      inString = true;
    } else if (character === "{") {
      depth += 1;
    } else if (character === "}") {
      depth -= 1;

      if (depth === 0) {
        candidates.push({ value: content.slice(start, index + 1), start, end: index + 1 });
        start = -1;
      }
    }
  }

  return candidates;
}

function normalizeWhitespace(content: string) {
  return content.replace(/\n{3,}/g, "\n\n").trim();
}
