import type { GuideStep, UiContextSnapshot, UiElementSummary } from "../../shared/types";

const minimumLabelSimilarity = 0.78;

export function reconcileGuideStepTarget(step: GuideStep, context: UiContextSnapshot | null): GuideStep {
  if (!step.target || !context) {
    return step;
  }

  const matchedElement = findGuideTarget(step.target.label, step.target.automationId, context);

  if (!matchedElement) {
    return {
      ...step,
      target: {
        label: step.target.label ?? step.target.automationId ?? null,
        automationId: step.target.automationId ?? null,
        bounds: null,
      },
    };
  }

  return {
    ...step,
    target: {
      label: matchedElement.name || step.target.label || null,
      automationId: matchedElement.automationId || step.target.automationId || null,
      bounds: matchedElement.bounds,
    },
  };
}

function findGuideTarget(label: string | null | undefined, automationId: string | null | undefined, context: UiContextSnapshot) {
  const candidates = uniqueElements([
    context.pointedElement,
    context.focusedElement,
    ...context.elements,
  ]);
  const normalizedAutomationId = normalize(automationId);

  if (normalizedAutomationId) {
    const automationMatch = candidates.find((element) => normalize(element.automationId) === normalizedAutomationId);

    if (automationMatch) {
      return automationMatch;
    }
  }

  const normalizedLabel = normalize(label);

  if (!normalizedLabel) {
    return null;
  }

  const exactMatch = candidates.find((element) => normalize(element.name) === normalizedLabel);

  if (exactMatch) {
    return exactMatch;
  }

  return candidates
    .map((element) => ({ element, similarity: labelSimilarity(normalizedLabel, normalize(element.name)) }))
    .filter((candidate) => candidate.similarity >= minimumLabelSimilarity)
    .sort((left, right) => right.similarity - left.similarity)[0]?.element ?? null;
}

function uniqueElements(elements: Array<UiElementSummary | null | undefined>) {
  const known = new Set<string>();

  return elements.filter((element): element is UiElementSummary => {
    if (!element) {
      return false;
    }

    const key = `${element.role}:${element.automationId ?? ""}:${element.name}:${element.bounds.x}:${element.bounds.y}`;

    if (known.has(key)) {
      return false;
    }

    known.add(key);
    return true;
  });
}

function normalize(value: string | null | undefined) {
  return (value ?? "")
    .toLocaleLowerCase()
    .replace(/[^\p{L}\p{N}]+/gu, " ")
    .trim()
    .replace(/\s+/g, " ");
}

function labelSimilarity(left: string, right: string) {
  if (!left || !right) {
    return 0;
  }

  if (left.includes(right) || right.includes(left)) {
    return Math.min(left.length, right.length) / Math.max(left.length, right.length);
  }

  const leftTokens = new Set(left.split(" "));
  const rightTokens = new Set(right.split(" "));
  const shared = [...leftTokens].filter((token) => rightTokens.has(token)).length;

  return shared / Math.max(leftTokens.size, rightTokens.size);
}
