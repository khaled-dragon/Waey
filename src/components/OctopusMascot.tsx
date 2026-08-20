export type OctopusState = "idle" | "thinking" | "replying";

interface Props {
  state?: OctopusState;
  size?: number;
  useThinkingAsset?: boolean;
}

export function OctopusMascot({ size = 38, state = "idle", useThinkingAsset = false }: Props) {
  return (
    <img
      alt=""
      aria-hidden="true"
      src={useThinkingAsset && state === "thinking" ? "/assets/waey-thinking.png" : "/assets/waey-logo.png"}
      style={{
        display: "block",
        height: size,
        objectFit: "contain",
        width: size,
      }}
    />
  );
}
