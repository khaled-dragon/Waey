import { listen } from "@tauri-apps/api/event";
import { useCallback, useEffect, useRef, useState } from "react";
import type { ChatMessage, GuideOffer, GuideOverlayRequest, GuideStep, UiContextSnapshot } from "../../shared/types";
import { cancelGuideStep, showGuideStep } from "./guideCommands";
import { extractGuideResponse } from "./guideProtocol";
import { reconcileGuideStepTarget } from "./guideTargeting";

type GuidePhase = "idle" | "awaiting" | "offer" | "active" | "adjusting";

interface GuideSessionState {
  phase: GuidePhase;
  step: GuideStep | null;
  offer: GuideOffer | null;
  startedAt: number;
  knownAssistantMessageIds: string[];
}

interface UseGuideSessionOptions {
  isDark: boolean;
  isRtl: boolean;
  messages: ChatMessage[];
  uiContext: UiContextSnapshot | null;
  onContinueGuide: (step: GuideStep, onContextCaptured: () => void) => Promise<void>;
  onGuideAdjustmentRequested: (step: GuideStep) => void;
  onError: (message: string) => void;
  streamState: "idle" | "streaming" | "error";
}

const idleGuideState: GuideSessionState = {
  phase: "idle",
  step: null,
  offer: null,
  startedAt: 0,
  knownAssistantMessageIds: [],
};

export function useGuideSession({
  isDark,
  isRtl,
  messages,
  uiContext,
  onContinueGuide,
  onGuideAdjustmentRequested,
  onError,
  streamState,
}: UseGuideSessionOptions) {
  const [guideState, setGuideState] = useState<GuideSessionState>(idleGuideState);
  const guideStateRef = useRef(guideState);
  const assistantMessageIdsRef = useRef<string[]>([]);
  const activateStep = useCallback((step: GuideStep, startedAt: number) => {
    const reconciledStep = reconcileGuideStepTarget(step, uiContext);

    setGuideState({
      phase: "active",
      step: reconciledStep,
      offer: null,
      startedAt,
      knownAssistantMessageIds: assistantMessageIdsRef.current,
    });
    void showGuideStep(toStepOverlayRequest(reconciledStep, isDark, isRtl)).catch((error) => {
      setGuideState(idleGuideState);
      onError(error instanceof Error ? error.message : String(error));
    });
  }, [isDark, isRtl, onError, uiContext]);

  useEffect(() => {
    guideStateRef.current = guideState;
  }, [guideState]);

  useEffect(() => {
    assistantMessageIdsRef.current = messages
      .filter((message) => message.role === "assistant")
      .map((message) => message.id);
  }, [messages]);

  useEffect(() => {
    if (guideState.phase === "awaiting" && streamState === "error") {
      setGuideState(idleGuideState);
    }
  }, [guideState.phase, streamState]);

  useEffect(() => {
    if (guideState.phase !== "awaiting" || streamState !== "idle") {
      return;
    }

    const response = messages
      .slice()
      .reverse()
      .find((message) => message.role === "assistant"
        && message.content.trim()
        && !guideState.knownAssistantMessageIds.includes(message.id));

    if (!response) {
      return;
    }
    const guideResponse = response ? extractGuideResponse(response.content) : null;

    if (!guideResponse) {
      setGuideState(idleGuideState);
      void cancelGuideStep().catch(() => undefined);
      onError(isRtl
        ? "تعذر على Waey التحقق من خطوة الإرشاد. جرّب طلب الإرشاد مرة أخرى."
        : "Waey could not validate the guide response. Please start the guide again.");
      return;
    }

    if (guideResponse.kind === "complete") {
      setGuideState(idleGuideState);
      void cancelGuideStep().catch((error) => {
        onError(error instanceof Error ? error.message : String(error));
      });
      return;
    }

    if (guideResponse.kind === "offer") {
      const offerRequest = toOfferOverlayRequest(guideResponse, isDark, isRtl);
      setGuideState({
        phase: "offer",
        step: null,
        offer: guideResponse,
        startedAt: guideState.startedAt,
        knownAssistantMessageIds: guideState.knownAssistantMessageIds,
      });
      void showGuideStep(offerRequest).catch((error) => {
        setGuideState(idleGuideState);
        onError(error instanceof Error ? error.message : String(error));
      });
      return;
    }

    activateStep(guideResponse, guideState.startedAt);
  }, [activateStep, guideState.phase, isDark, isRtl, messages, onError, streamState]);

  useEffect(() => {
    let isMounted = true;

    const listeners = Promise.all([
      listen("guide-offer-started", () => {
        const offer = guideStateRef.current.offer;

        if (!offer || guideStateRef.current.phase !== "offer") {
          return;
        }

        activateStep(offer.firstStep, guideStateRef.current.startedAt);
      }),
      listen("guide-step-confirmed", () => {
        const activeStep = guideStateRef.current.step;

        if (!activeStep || guideStateRef.current.phase !== "active") {
          return;
        }

        if (activeStep.estimatedStepsLeft === 0) {
          setGuideState(idleGuideState);
          void cancelGuideStep().catch((error) => {
            if (isMounted) {
              onError(error instanceof Error ? error.message : String(error));
            }
          });
          return;
        }

        setGuideState({
          phase: "awaiting",
          step: activeStep,
          offer: null,
          startedAt: Date.now(),
          knownAssistantMessageIds: assistantMessageIdsRef.current,
        });
        void onContinueGuide(activeStep, () => {
          void showGuideStep(toThinkingOverlayRequest(isDark, isRtl)).catch((error) => {
            if (isMounted) {
              onError(error instanceof Error ? error.message : String(error));
            }
          });
        }).catch((error) => {
          if (isMounted) {
            setGuideState(idleGuideState);
            onError(error instanceof Error ? error.message : String(error));
          }
        });
      }),
      listen("guide-cancelled", () => {
        setGuideState(idleGuideState);
      }),
      listen("guide-adjustment-requested", () => {
        const activeStep = guideStateRef.current.step;

        if (!activeStep || guideStateRef.current.phase !== "active") {
          return;
        }

        setGuideState({
          phase: "adjusting",
          step: activeStep,
          offer: null,
          startedAt: guideStateRef.current.startedAt,
          knownAssistantMessageIds: guideStateRef.current.knownAssistantMessageIds,
        });
        onGuideAdjustmentRequested(activeStep);
      }),
    ]);

    return () => {
      isMounted = false;
      void listeners.then((unlisteners) => unlisteners.forEach((unlisten) => unlisten()));
    };
  }, [activateStep, isDark, isRtl, onContinueGuide, onError, onGuideAdjustmentRequested]);

  const beginGuide = useCallback(() => {
    setGuideState({
      phase: "awaiting",
      step: null,
      offer: null,
      startedAt: Date.now(),
      knownAssistantMessageIds: assistantMessageIdsRef.current,
    });
  }, []);

  const cancelGuide = useCallback(async () => {
    if (guideStateRef.current.phase !== "idle") {
      await cancelGuideStep().catch((error) => {
        onError(error instanceof Error ? error.message : String(error));
      });
    }

    setGuideState(idleGuideState);
  }, [onError]);

  const beginGuideAdjustmentFollowUp = useCallback(() => {
    const currentGuide = guideStateRef.current;

    if (currentGuide.phase !== "adjusting" || !currentGuide.step) {
      return null;
    }

    setGuideState({
      phase: "awaiting",
      step: currentGuide.step,
      offer: null,
      startedAt: Date.now(),
      knownAssistantMessageIds: assistantMessageIdsRef.current,
    });

    return currentGuide.step;
  }, []);

  return {
    beginGuide,
    beginGuideAdjustmentFollowUp,
    cancelGuide,
    guideState,
  };
}

function toOfferOverlayRequest(offer: GuideOffer, isDark: boolean, isRtl: boolean): GuideOverlayRequest {
  return {
    mode: "offer",
    caption: offer.summary,
    target: null,
    stepIndex: 1,
    estimatedStepsLeft: Math.max(0, offer.estimatedSteps - 1),
    theme: isDark ? "dark" : "light",
    isRtl,
  };
}

function toStepOverlayRequest(step: GuideStep, isDark: boolean, isRtl: boolean): GuideOverlayRequest {
  return {
    mode: "step",
    caption: step.caption,
    target: step.target,
    stepIndex: step.stepIndex,
    estimatedStepsLeft: step.estimatedStepsLeft,
    theme: isDark ? "dark" : "light",
    isRtl,
  };
}

function toThinkingOverlayRequest(isDark: boolean, isRtl: boolean): GuideOverlayRequest {
  return {
    mode: "thinking",
    caption: isRtl ? "Waey يفحص التغيير على الشاشة ويجهز الخطوة التالية." : "Waey is checking the screen and preparing the next step.",
    target: null,
    stepIndex: 0,
    estimatedStepsLeft: 0,
    theme: isDark ? "dark" : "light",
    isRtl,
  };
}
