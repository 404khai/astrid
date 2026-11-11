import type { CSSProperties } from "react";
import { AstridLogo } from "./AstridLogo";
import { astridCursor } from "./cursors";

const TIP = 30;
const [hx, hy] = astridCursor.hot;

/** Astrid's own pointer: the Arrow click cursor (outline arrow with click sparks), with the
 * Astrid mark (eyes tracking) trailing it; distinct from the system cursor so it is always clear who
 * is pointing. `stick` adds a whiteboard pointer whose tip is the hotspot. */
export function AstridCursor({
  x,
  y,
  className = "",
  stick = false,
}: {
  x: number;
  y: number;
  /** Kept for callers; Astrid no longer wears a name pill. */
  label?: string;
  speaking?: boolean;
  className?: string;
  stick?: boolean;
}) {
  return (
    <div
      className={`astrid-cursor ${stick ? "with-stick" : ""} ${className}`}
      style={{ "--x": `${x}px`, "--y": `${y}px` } as CSSProperties}
      aria-hidden="true"
    >
      <span className="cursor-follow">
        {stick && <i className="pointer-stick" />}
        <AstridLogo tracking />
      </span>
      <svg
        className="cursor-tip"
        viewBox="0 0 32 32"
        width={TIP}
        height={TIP}
        style={{ left: -hx * (TIP / 32), top: -hy * (TIP / 32) }}
      >
        {astridCursor.art}
      </svg>
    </div>
  );
}

/** The user's macOS-style system pointer. */
export function SystemCursor({ x, y }: { x: number; y: number }) {
  return (
    <svg
      className="system-cursor"
      style={{ left: x, top: y }}
      viewBox="0 0 16 22"
      width="16"
      height="22"
      aria-hidden="true"
    >
      <path d="M1 1v17l4.4-4.2 2.8 6.6 2.7-1.2-2.8-6.4H14Z" />
    </svg>
  );
}
