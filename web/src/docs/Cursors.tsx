import { useRef, useState, type CSSProperties } from "react";
import { AstridLogo } from "../components/AstridLogo";
import { cursors, type Cursor } from "../components/cursors";

const SIZE = 40;

/** One cursor with Astrid's mark trailing it. Inside its card it follows the
 * real mouse; otherwise it rests in the middle. */
export function CursorCard({ cursor, index }: { cursor: Cursor; index: number }) {
  const stage = useRef<HTMLDivElement>(null);
  const [at, setAt] = useState<[number, number] | null>(null);
  const scale = SIZE / 32;
  const [hx, hy] = cursor.hot;
  return (
    <article className="cc-card">
      <div
        ref={stage}
        className={`cc-stage ${at ? "live" : ""}`}
        onPointerMove={(event) => {
          const box = stage.current?.getBoundingClientRect();
          if (box) setAt([event.clientX - box.left, event.clientY - box.top]);
        }}
        onPointerLeave={() => setAt(null)}
      >
        <div
          className="cc-cursor"
          style={
            {
              "--x": at ? `${at[0]}px` : "50%",
              "--y": at ? `${at[1]}px` : "42%",
            } as CSSProperties
          }
        >
          <span
            className="cc-follow"
            style={{ left: (32 - hx) * scale + 2, top: (32 - hy) * scale - 4 }}
          >
            <AstridLogo tracking />
          </span>
          <svg
            className="cc-art"
            viewBox="0 0 32 32"
            width={SIZE}
            height={SIZE}
            style={{ left: -hx * scale, top: -hy * scale }}
            aria-hidden="true"
          >
            {cursor.art}
          </svg>
        </div>
      </div>
      <footer>
        <span>{String(index + 1).padStart(2, "0")}</span>
        <b>{cursor.name}</b>
        <small>
          hot {hx},{hy}
        </small>
      </footer>
    </article>
  );
}


export function CursorGrid() {
  return (
    <div className="cc-grid">
      {cursors.map((cursor, index) => (
        <CursorCard key={cursor.name} cursor={cursor} index={index} />
      ))}
    </div>
  );
}
