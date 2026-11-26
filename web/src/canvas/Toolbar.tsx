import { useState } from "react";
import { range, type Live } from "./interact";

/** The canvas toolbar (Astrid Weaver). Each control appears only when the
 * drawing has something for it: motion, steps, exploded parts, a casing to cut
 * away or live values. Export is always there. */
export type ToolbarProps = {
  motion: boolean;
  paused: boolean;
  onPause(): void;
  tempo: number;
  onTempo(): void;
  steps: number;
  step: number | null;
  onStep(next: number | null): void;
  explodable: boolean;
  explode: number;
  onExplode(t: number): void;
  shelled: boolean;
  cut: boolean;
  onCut(): void;
  values: Live[];
  onValue(id: string, value: number): void;
  /** Whether this host can save a file (the desktop app or a browser). */
  exportable: boolean;
  onExport(format: "svg" | "png"): void;
};

const Icon = ({ d }: { d: string }) => (
  <svg viewBox="0 0 16 16" width="14" height="14" aria-hidden="true">
    <path d={d} />
  </svg>
);

export function Toolbar(p: ToolbarProps) {
  const [menu, setMenu] = useState(false);
  const fmt = (n: number) => n.toLocaleString(undefined, { maximumFractionDigits: Math.abs(n) < 10 ? 1 : 0 });
  return (
    <div className="cv-tools" role="toolbar" aria-label="Drawing controls" onPointerDown={(e) => e.stopPropagation()}>
      {p.motion && (
        <>
          <button type="button" className="cv-tool" aria-label={p.paused ? "Play motion" : "Pause motion"} onClick={p.onPause}>
            <Icon d={p.paused ? "M5 3.5v9l7-4.5z" : "M5 3.5v9M11 3.5v9"} />
          </button>
          <button type="button" className="cv-tool cv-tool-text" aria-label="Motion speed" title="Motion speed" onClick={p.onTempo}>
            {p.tempo}×
          </button>
        </>
      )}
      {p.steps > 0 &&
        (p.step === null ? (
          <button type="button" className="cv-tool cv-tool-text" onClick={() => p.onStep(0)}>
            Walk through
          </button>
        ) : (
          <span className="cv-steps">
            <button type="button" className="cv-tool" aria-label="Previous step" disabled={p.step === 0} onClick={() => p.onStep(p.step! - 1)}>
              <Icon d="M10 3.5 5.5 8l4.5 4.5" />
            </button>
            <span className="cv-step-count">
              {p.step + 1} of {p.steps}
            </span>
            <button type="button" className="cv-tool" aria-label="Next step" disabled={p.step === p.steps - 1} onClick={() => p.onStep(p.step! + 1)}>
              <Icon d="M6 3.5 10.5 8 6 12.5" />
            </button>
            <button type="button" className="cv-tool" aria-label="End walkthrough" onClick={() => p.onStep(null)}>
              <Icon d="m4.5 4.5 7 7m0-7-7 7" />
            </button>
          </span>
        ))}
      {p.explodable && (
        <label className="cv-slider">
          <span>Explode</span>
          <input type="range" min={0} max={1} step={0.01} value={p.explode} onChange={(e) => p.onExplode(Number(e.target.value))} />
        </label>
      )}
      {p.shelled && (
        <button type="button" className={`cv-tool cv-tool-text${p.cut ? " on" : ""}`} aria-pressed={p.cut} onClick={p.onCut}>
          Cutaway
        </button>
      )}
      {p.values.slice(0, 2).map((v) => {
        const [lo, hi] = range(v);
        return (
          <label key={v.id} className="cv-slider">
            <span>{v.shape.label ?? v.id}</span>
            <input type="range" min={lo} max={hi} step={(hi - lo) / 200} value={v.now} onChange={(e) => p.onValue(v.id, Number(e.target.value))} />
            <output>
              {fmt(v.now)}
              {v.shape.unit ? ` ${v.shape.unit}` : ""}
            </output>
          </label>
        );
      })}
      {p.exportable && <span className="cv-export">
        <button type="button" className="cv-tool" aria-label="Export drawing" aria-expanded={menu} onClick={() => setMenu((m) => !m)}>
          <Icon d="M8 2.5v7.5M5 7l3 3 3-3M3 12.5h10" />
        </button>
        {menu && (
          <span className="cv-menu" role="menu">
            {(["png", "svg"] as const).map((f) => (
              <button
                key={f}
                type="button"
                role="menuitem"
                onClick={() => {
                  setMenu(false);
                  p.onExport(f);
                }}
              >
                {f === "png" ? "PNG image" : "SVG vector"}
              </button>
            ))}
          </span>
        )}
      </span>}
    </div>
  );
}
