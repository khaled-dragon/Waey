import { useEffect, useRef, useState } from "react";

export type OctopusState = "idle" | "thinking" | "replying";

interface Props {
  state?: OctopusState;
  size?: number;
}

const EYE_L = { cx: 108, cy: 88 };
const EYE_R = { cx: 148, cy: 88 };
const EYE_WHITE_R = 18;
const PUPIL_TRAVEL = 9;

const C = {
  body:       "#c22b31",
  bodyDeep:   "#8b0000",
  bodyLight:  "#ef3f42",
  bodySheen:  "#ff6b6b",
  line:       "#5a0010",
  belly:      "#fde8e8",
  bellyShade: "#f5c5c5",
  iris:       "#1a0810",
  irisRing:   "#3d0015",
  pupil:      "#0a0305",
  hi:         "#ffffff",
  sucker:     "rgba(139,0,0,0.25)",
  cheek:      "rgba(239,63,66,0.3)",
};

export function OctopusMascot({ state = "idle", size = 38 }: Props) {
  const svgRef = useRef<SVGSVGElement>(null);
  const [pupilOffset, setPupilOffset] = useState({ dx: 0, dy: 0 });
  const [blink, setBlink] = useState(false);
  const [replyPop, setReplyPop] = useState(false);
  const [tentaclePhase, setTentaclePhase] = useState(0);

  // Eye tracking
  useEffect(() => {
    const apply = (clientX: number, clientY: number) => {
      if (!svgRef.current) return;
      const rect = svgRef.current.getBoundingClientRect();
      const cx = rect.left + rect.width / 2;
      const cy = rect.top + rect.height / 2;
      const dx = clientX - cx;
      const dy = clientY - cy;
      const dist = Math.hypot(dx, dy);
      const scale = Math.min(1, dist / 200) * PUPIL_TRAVEL;
      setPupilOffset({ dx: (dx / (dist || 1)) * scale, dy: (dy / (dist || 1)) * scale });
    };
    const onMove = (e: MouseEvent) => apply(e.clientX, e.clientY);
    window.addEventListener("mousemove", onMove);
    return () => window.removeEventListener("mousemove", onMove);
  }, []);

  // Blink
  useEffect(() => {
    let cancelled = false;
    const loop = () => {
      if (cancelled) return;
      setBlink(true);
      setTimeout(() => setBlink(false), 110);
      setTimeout(loop, 2200 + (Math.random() - 0.5) * 900);
    };
    const t = setTimeout(loop, 1500);
    return () => { cancelled = true; clearTimeout(t); };
  }, []);

  // Tentacle animation
  useEffect(() => {
    let frame: number;
    let start: number | null = null;
    const animate = (ts: number) => {
      if (!start) start = ts;
      setTentaclePhase((ts - start) / 1000);
      frame = requestAnimationFrame(animate);
    };
    frame = requestAnimationFrame(animate);
    return () => cancelAnimationFrame(frame);
  }, []);

  // Reply pop
  useEffect(() => {
    if (state === "replying") {
      setReplyPop(true);
      const t = setTimeout(() => setReplyPop(false), 600);
      return () => clearTimeout(t);
    }
  }, [state]);

  const thinking = state === "thinking";
  const [thinkOffset, setThinkOffset] = useState({ dx: 0, dy: 0 });
  useEffect(() => {
    if (!thinking) { setThinkOffset({ dx: 0, dy: 0 }); return; }
    let cancelled = false;
    const poses = [
      { dx: PUPIL_TRAVEL * 0.9, dy: -PUPIL_TRAVEL * 0.6 },
      { dx: -PUPIL_TRAVEL * 0.9, dy: -PUPIL_TRAVEL * 0.6 },
      { dx: PUPIL_TRAVEL * 0.8, dy: 0 },
      { dx: -PUPIL_TRAVEL * 0.8, dy: 0 },
      { dx: 0, dy: -PUPIL_TRAVEL * 0.8 },
      { dx: 0, dy: 0 },
    ];
    let last = -1;
    const step = () => {
      if (cancelled) return;
      let i = Math.floor(Math.random() * poses.length);
      if (i === last) i = (i + 1) % poses.length;
      last = i;
      setThinkOffset(poses[i]);
      setTimeout(step, 1000 + Math.random() * 600);
    };
    const t = setTimeout(step, 200);
    return () => { cancelled = true; clearTimeout(t); };
  }, [thinking]);

  const eDx = thinking ? thinkOffset.dx : pupilOffset.dx;
  const eDy = thinking ? thinkOffset.dy : pupilOffset.dy;
  const p = tentaclePhase;

  // Tentacle wave function
  const tw = (base: number, amp: number, freq: number, offset: number) =>
    base + amp * Math.sin(p * freq + offset);

  return (
    <svg
      ref={svgRef}
      viewBox="0 0 256 256"
      width={size}
      height={size}
      style={{
        display: "block",
        transition: "transform 200ms ease",
        transform: replyPop ? "scale(1.1)" : "scale(1)",
        overflow: "visible",
      }}
      aria-hidden="true"
    >
      <defs>
        <clipPath id="eyeClipL_oct">
          <circle cx={EYE_L.cx} cy={EYE_L.cy} r={EYE_WHITE_R} />
        </clipPath>
        <clipPath id="eyeClipR_oct">
          <circle cx={EYE_R.cx} cy={EYE_R.cy} r={EYE_WHITE_R} />
        </clipPath>
        <radialGradient id="bodyGrad" cx="45%" cy="35%" r="65%">
          <stop offset="0%" stopColor={C.bodySheen} />
          <stop offset="50%" stopColor={C.body} />
          <stop offset="100%" stopColor={C.bodyDeep} />
        </radialGradient>
      </defs>

      {/* Shadow */}
      <ellipse cx="128" cy="248" rx="55" ry="5" fill={C.line} opacity="0.15" />

      {/* Tentacles — 8 animated legs */}
      {[
        { x: 90,  rot: -50, phase: 0    },
        { x: 105, rot: -30, phase: 0.8  },
        { x: 118, rot: -12, phase: 1.6  },
        { x: 131, rot:   0, phase: 2.2  },
        { x: 144, rot:  12, phase: 1.6  },
        { x: 157, rot:  30, phase: 0.8  },
        { x: 172, rot:  50, phase: 0    },
        { x: 128, rot: 180, phase: 3.0  },
      ].map((t, i) => {
        const wave = Math.sin(p * 1.8 + t.phase) * 10;
        const wave2 = Math.sin(p * 2.2 + t.phase + 1) * 7;
        const baseY = 185;
        const endY = baseY + 55 + Math.abs(wave) * 0.3;
        const cx1 = t.x + wave * 0.5;
        const cy1 = baseY + 20;
        const cx2 = t.x + wave2;
        const cy2 = baseY + 40;
        const ex = t.x + wave;
        return (
          <g key={i}>
            <path
              d={`M ${t.x} ${baseY} C ${cx1} ${cy1} ${cx2} ${cy2} ${ex} ${endY}`}
              fill="none"
              stroke={C.bodyDeep}
              strokeWidth="9"
              strokeLinecap="round"
            />
            <path
              d={`M ${t.x} ${baseY} C ${cx1} ${cy1} ${cx2} ${cy2} ${ex} ${endY}`}
              fill="none"
              stroke={C.body}
              strokeWidth="6"
              strokeLinecap="round"
            />
            {/* Suckers */}
            {[0.35, 0.65, 0.88].map((frac, j) => {
              const sx = t.x + (ex - t.x) * frac + wave * frac * 0.3;
              const sy = baseY + (endY - baseY) * frac;
              return <circle key={j} cx={sx} cy={sy} r="3.5" fill={C.sucker} stroke={C.bodyDeep} strokeWidth="0.8" />;
            })}
          </g>
        );
      })}

      {/* Body dome */}
      <ellipse
        cx="128" cy="148"
        rx="72" ry="68"
        fill="url(#bodyGrad)"
        stroke={C.line}
        strokeWidth="3.5"
      />

      {/* Belly */}
      <ellipse cx="128" cy="158" rx="38" ry="42" fill={C.belly} stroke={C.bellyShade} strokeWidth="1.5" />
      <ellipse cx="128" cy="178" rx="28" ry="22" fill={C.bellyShade} opacity="0.5" />

      {/* Smile */}
      <path
        d="M 116 172 Q 128 182 140 172"
        fill="none"
        stroke={C.bodyDeep}
        strokeWidth="2.2"
        strokeLinecap="round"
        opacity="0.65"
      />

      {/* Cheeks */}
      <ellipse cx="100" cy="108" rx="12" ry="8" fill={C.cheek} />
      <ellipse cx="156" cy="108" rx="12" ry="8" fill={C.cheek} />

      {/* Head bump */}
      <ellipse cx="128" cy="72" rx="52" ry="38" fill="url(#bodyGrad)" stroke={C.line} strokeWidth="3" />

      {/* Sheen on head */}
      <ellipse cx="112" cy="60" rx="18" ry="10" fill={C.bodySheen} opacity="0.3" transform="rotate(-20,112,60)" />

      {/* Eye whites */}
      <circle cx={EYE_L.cx} cy={EYE_L.cy} r={EYE_WHITE_R} fill={C.hi} stroke={C.line} strokeWidth="2.5" />
      <circle cx={EYE_R.cx} cy={EYE_R.cy} r={EYE_WHITE_R} fill={C.hi} stroke={C.line} strokeWidth="2.5" />

      {/* Pupils + iris tracking */}
      <g style={{ transform: `translate(${eDx}px,${eDy}px)`, transition: "transform 80ms linear" }}>
        <g clipPath="url(#eyeClipL_oct)">
          <circle cx={EYE_L.cx} cy={EYE_L.cy} r="13" fill={C.irisRing} />
          <circle cx={EYE_L.cx} cy={EYE_L.cy} r="10" fill={C.iris} />
          <circle cx={EYE_L.cx} cy={EYE_L.cy} r="5.5" fill={C.pupil} />
          <circle cx={EYE_L.cx + 2} cy={EYE_L.cy - 3} r="2.5" fill={C.hi} />
        </g>
        <g clipPath="url(#eyeClipR_oct)">
          <circle cx={EYE_R.cx} cy={EYE_R.cy} r="13" fill={C.irisRing} />
          <circle cx={EYE_R.cx} cy={EYE_R.cy} r="10" fill={C.iris} />
          <circle cx={EYE_R.cx} cy={EYE_R.cy} r="5.5" fill={C.pupil} />
          <circle cx={EYE_R.cx + 2} cy={EYE_R.cy - 3} r="2.5" fill={C.hi} />
        </g>
      </g>

      {/* Eyelids */}
      <g fill={C.body} stroke={C.line} strokeWidth="2.5">
        <circle cx={EYE_L.cx} cy={EYE_L.cy} r={EYE_WHITE_R}
          style={{ transformOrigin: `${EYE_L.cx}px ${EYE_L.cy - EYE_WHITE_R}px`, transformBox: "fill-box", transform: blink ? "scaleY(1)" : "scaleY(0)", transition: "transform 55ms ease-in-out" }} />
        <circle cx={EYE_R.cx} cy={EYE_R.cy} r={EYE_WHITE_R}
          style={{ transformOrigin: `${EYE_R.cx}px ${EYE_R.cy - EYE_WHITE_R}px`, transformBox: "fill-box", transform: blink ? "scaleY(1)" : "scaleY(0)", transition: "transform 55ms ease-in-out" }} />
      </g>
    </svg>
  );
}
