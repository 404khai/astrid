import { useEffect, useId, useRef } from "react";
import "./astrid-logo.css";

/** The original Astrid mark, with independently animated eyes. */
/** Eye expressions for agent states; omit for the original mark. */
export type EyeExpression =
  | "working"
  | "searching"
  | "solving"
  | "listening"
  | "connecting"
  | "weaving"
  | "composing"
  | "sketching"
  | "idle";

/** Per-eye corner radius in mark units: rounder eyes read softer. */
const eyeRadius: Record<EyeExpression, number> = {
  working: 0.2,
  searching: 0.3,
  solving: 0.2,
  listening: 1.4,
  connecting: 0.6,
  weaving: 0.5,
  composing: 0.4,
  sketching: 0.8,
  idle: 1,
};

export function AstridLogo({
  animated = false,
  tracking = false,
  expression,
}: {
  animated?: boolean;
  tracking?: boolean;
  expression?: EyeExpression;
}) {
  const id = useId();
  const logo = useRef<SVGSVGElement>(null);
  const gaze = useRef<SVGGElement>(null);
  useEffect(() => {
    if (!tracking) return;
    const reducedMotion = window.matchMedia("(prefers-reduced-motion: reduce)");
    const reset = () => {
      gaze.current?.classList.remove("tracking");
      logo.current?.style.removeProperty("--eye-x");
      logo.current?.style.removeProperty("--eye-y");
    };
    const follow = (event: PointerEvent) => {
      if (
        event.pointerType !== "mouse" ||
        reducedMotion.matches ||
        !logo.current
      )
        return;
      const bounds = logo.current.getBoundingClientRect();
      const dx = event.clientX - (bounds.left + bounds.width / 2);
      const dy = event.clientY - (bounds.top + bounds.height / 2);
      // Bound travel in SVG coordinates so the eyes stay inside the original mark.
      logo.current.style.setProperty(
        "--eye-x",
        `${Math.tanh(dx / 180) * 1.2}px`,
      );
      logo.current.style.setProperty("--eye-y", `${Math.tanh(dy / 180) * 1}px`);
      gaze.current?.classList.add("tracking");
    };
    window.addEventListener("pointermove", follow, { passive: true });
    document.addEventListener("pointerleave", reset);
    window.addEventListener("blur", reset);
    reducedMotion.addEventListener("change", reset);
    return () => {
      window.removeEventListener("pointermove", follow);
      document.removeEventListener("pointerleave", reset);
      window.removeEventListener("blur", reset);
      reducedMotion.removeEventListener("change", reset);
      reset();
    };
  }, [tracking]);
  return (
    <svg
      ref={logo}
      className="astrid-logo"
      aria-hidden="true"
      viewBox="0 0 20 16"
      fill="none"
      xmlns="http://www.w3.org/2000/svg"
    >
      <path
        d="M19.9531 2.8951H17.7133C17.3275 2.8951 16.9572 2.7432 16.6826 2.4721L14.607 0.4231C14.3324 0.152 13.9621 0 13.5762 0H6.4269C6.0411 0 5.6708 0.152 5.3962 0.4231L3.3206 2.4721C3.046 2.7432 2.6757 2.8951 2.2898 2.8951H0.0501C0.0219 2.8951 0 2.9139 0 2.9421V5.5524C0 5.5775 0.0219 5.5931 0.0501 5.5931H3.1447C3.5321 5.5931 3.9039 5.4398 4.1787 5.1668L6.2 3.1587C6.4749 2.8857 6.8465 2.7324 7.234 2.7324H12.8134C13.2006 2.7324 13.572 2.8854 13.8469 3.1582L15.8714 5.1674C16.1463 5.4401 16.5177 5.5931 16.9049 5.5931H19.9531C19.9812 5.5931 20 5.5775 20 5.5524V2.9421C20 2.9139 19.9812 2.8951 19.9531 2.8951Z"
        fill={`url(#${id}-top)`}
      />
      <path
        d="M18.5665 10.3552C18.5665 10.3492 18.5616 10.3443 18.5556 10.3443H16.929C16.5424 10.3443 16.1714 10.4969 15.8967 10.769L13.8498 12.796C13.5751 13.068 13.2041 13.2207 12.8174 13.2207H7.2411C6.855 13.2207 6.4845 13.0685 6.2098 12.7971L4.1782 10.7897C3.9036 10.5184 3.5331 10.3662 3.1471 10.3662H0.0532C0.0219 10.3662 0.0031 10.385 0.0031 10.41V13.0203C0.0031 13.0454 0.0219 13.061 0.0532 13.061H2.2835C2.6712 13.061 3.0432 13.2145 3.3181 13.488L5.3955 15.5543C5.6704 15.8277 6.0424 15.9812 6.4301 15.9812H13.5748C13.9615 15.9812 14.3325 15.8286 14.6072 15.5565L16.6979 13.4858C16.9726 13.2137 17.3437 13.061 17.7303 13.061H19.9468C19.975 13.061 19.9969 13.0454 19.9969 13.0203V10.41C19.9969 10.385 19.975 10.3662 19.9468 10.3662H18.5775C18.5714 10.3662 18.5665 10.3613 18.5665 10.3552Z"
        fill={`url(#${id}-bottom)`}
      />
      <g ref={gaze} className={animated ? "logo-gaze animated" : "logo-gaze"}>
        {expression ? (
          <g className="logo-eyes logo-expression" data-expression={expression}>
            <rect className="eye eye-l" x="6.01" y="5.5" width="2.98" height="4.93" rx={eyeRadius[expression]} fill="#00F7D5" />
            <rect className="eye eye-r" x="11.02" y="5.5" width="3.02" height="4.93" rx={eyeRadius[expression]} fill="#00F7D5" />
          </g>
        ) : (
          <g className={animated ? "logo-eyes animated" : "logo-eyes"}>
            <path
              d="M8.9452 5.4898H6.0501C6.025 5.4898 6.0094 5.5055 6.0094 5.5243V10.3787C6.0094 10.4006 6.025 10.4194 6.0501 10.4194H8.9452C8.9703 10.4194 8.989 10.4006 8.989 10.3787V5.5243C8.989 5.5055 8.9703 5.4898 8.9452 5.4898Z"
              fill="#00F7D5"
            />
            <path
              d="M13.9812 5.4898H11.0579C11.036 5.4898 11.0203 5.5055 11.0203 5.5243V10.3787C11.0203 10.4006 11.036 10.4194 11.0579 10.4194H13.9812C14.0125 10.4194 14.0376 10.3975 14.0376 10.3693V5.543C14.0376 5.5117 14.0125 5.4898 13.9812 5.4898Z"
              fill="#00F7D5"
            />
          </g>
        )}
      </g>
      <defs>
        <linearGradient
          id={`${id}-top`}
          x1="3.9374"
          y1="0.9077"
          x2="17.8122"
          y2="6.7011"
          gradientUnits="userSpaceOnUse"
        >
          <stop stopColor="#4459F9" />
          <stop offset="0.5" stopColor="#1A32EC" />
          <stop offset="1" stopColor="#4459F9" />
        </linearGradient>
        <linearGradient
          id={`${id}-bottom`}
          x1="2.0595"
          y1="9.5462"
          x2="15.5399"
          y2="16.2128"
          gradientUnits="userSpaceOnUse"
        >
          <stop stopColor="#4459F9" />
          <stop offset="0.5" stopColor="#1A32EC" />
          <stop offset="1" stopColor="#4459F9" />
        </linearGradient>
      </defs>
    </svg>
  );
}
