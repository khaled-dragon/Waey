import { listen } from "@tauri-apps/api/event";
import { useCallback, useEffect, useRef, useState } from "react";
import type { ChatMessage, GuideStep } from "../../shared/types";
import { cancelGuideStep, showGuideStep } from "./guideCommands";
import { extractGuideResponse } from "./guideProtocol";

type GuidePhase = "idle" | "awaiting" | "active";

interface GuideSessionState {
  phase: GuidePhase;
  step: GuideStep | null;
  startedAt: number;
}

interface UseGuideSessionOptions {
  isDark: boolean;
  isRtl: boolean;
  messages: ChatMessage[];
  onContinueGuide: (step: GuideStep) => Promise<void>;
  onError: (message: string) => void;
  streamState: "idle" | "streaming" | "error";
}

const idleGuideState: GuideSessionState = { phase: "idle", step: null, startedAt: 0 };

export function useGuideSession({
  isDark,
  isRtl,
  messages,
  onContinueGuide,
  onError,
  streamState,
}: UseGuideSessionOptions) {
  const [guideState, setGuideState] = useState<GuideSessionState>(idleGuideState);
  const guideStateRef = useRef(guideState);

  useEffect(() => {
    guideStateRef.current = guideState;
  }, [guideState]);

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
        && (message.createdAt * 1000) >= guideState.startedAt - 1_000);

    if (!response) {
      return;
    }
    const guideResponse = response ? extractGuideResponse(response.content) : null;

    if (!guideResponse) {
      setGuideState(idleGuideState);
      return;
    }

    if (guideResponse.kind === "complete") {
      setGuideState(idleGuideState);
      return;
    }

    setGuideState({ phase: "active", step: guideResponse, startedAt: guideState.startedAt });
    void showGuideStep({
      ...guideResponse,
      theme: isDark ? "dark" : "light",
      isRtl,
    }).catch((error) => {
      setGuideState(idleGuideState);
      onError(error instanceof Error ? error.message : String(error));
    });
  }, [guideState.phase, isDark, isRtl, messages, onError, streamState]);

  useEffect(() => {
    let isMounted = true;

    const listeners = Promise.all([
      listen("guide-step-confirmed", () => {
        const activeStep = guideStateRef.current.step;

        if (!activeStep || guideStateRef.current.phase !== "active") {
          return;
        }

        setGuideState({ phase: "awaiting", step: activeStep, startedAt: Date.now() });
        void onContinueGuide(activeStep).catch((error) => {
          if (isMounted) {
            setGuideState(idleGuideState);
            onError(error instanceof Error ? error.message : String(error));
          }
        });
      }),
      listen("guide-cancelled", () => {
        setGuideState(idleGuideState);
      }),
    ]);

    return () => {
      isMounted = false;
      void listeners.then((unlisteners) => unlisteners.forEach((unlisten) => unlisten()));
    };
  }, [onContinueGuide, onError]);

  const beginGuide = useCallback(() => {
    setGuideState({ phase: "awaiting", step: null, startedAt: Date.now() });
  }, []);

  const cancelGuide = useCallback(async () => {
    if (guideStateRef.current.phase === "active") {
      await cancelGuideStep().catch((error) => {
        onError(error instanceof Error ? error.message : String(error));
      });
    }

    setGuideState(idleGuideState);
  }, [onError]);

  return {
    beginGuide,
    cancelGuide,
    guideState,
  };
}
