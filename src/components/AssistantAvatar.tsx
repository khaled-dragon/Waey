type AssistantState = "idle" | "listening" | "thinking" | "answering" | "error";

interface AssistantAvatarProps {
  state: AssistantState;
}

const stateLabel: Record<AssistantState, string> = {
  idle: "Ready",
  listening: "Listening",
  thinking: "Thinking",
  answering: "Answering",
  error: "Needs attention",
};

export function AssistantAvatar({ state }: AssistantAvatarProps) {
  return (
    <div className="flex items-center gap-3 rounded-full border border-white/10 bg-black/20 px-3 py-2">
      <div className="relative grid size-12 place-items-center overflow-hidden rounded-full bg-waey-bright shadow-lg shadow-waey-bright/20">
        <video
          aria-label="Waey mascot"
          autoPlay
          className="size-full object-cover"
          loop
          muted
          playsInline
          poster="/assets/waey-logo.png"
        >
          <source src="/assets/waey-octopus.mp4" type="video/mp4" />
        </video>
      </div>
      <div className="hidden sm:block">
        <p className="text-sm font-medium">{stateLabel[state]}</p>
        <p className="text-xs text-white/55">Animation ready</p>
      </div>
    </div>
  );
}
