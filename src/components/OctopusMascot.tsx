export type OctopusState = "idle" | "thinking" | "replying";

interface Props {
  state?: OctopusState;
  size?: number;
}

export function OctopusMascot({ size = 38 }: Props) {
  return (
    <img
      alt=""
      aria-hidden="true"
      src="/assets/waey-logo.png"
      style={{
        display: "block",
        height: size,
        objectFit: "contain",
        width: size,
      }}
    />
  );
}
