import { useEffect, useRef } from "react";
import { engineFigure, type EngineAnchor } from "../generated/engine-figure";

export type Stroke = "intake" | "compression" | "power" | "exhaust";
const strokes: Stroke[] = ["intake", "compression", "power", "exhaust"];
const CYCLE_MS = 3200;
const frameOf = (lift: number, count: number) =>
  Math.round(Math.max(0, Math.min(1, lift)) * (count - 1));

/** The Anatomy-built engine. One crank model drives the valves, flywheel,
 * spark and the stroke readout, so every visible state agrees. */
export function EngineFigure({
  focus,
  onStroke,
  theme = "dark",
  still = false,
}: {
  focus?: EngineAnchor;
  onStroke?: (stroke: Stroke, crank: number) => void;
  theme?: "dark" | "light";
  still?: boolean;
}) {
  const host = useRef<HTMLDivElement>(null);
  const report = useRef(onStroke);
  report.current = onStroke;

  useEffect(() => {
    const root = host.current;
    if (!root) return;
    const valveIn = root.querySelector('[data-part="valve-in"]');
    const valveEx = root.querySelector('[data-part="valve-ex"]');
    const spokes = root.querySelector('[data-part="spokes"]');
    const mark = root.querySelector('[data-part="mark"]');
    const plug = root.querySelector('[data-part="plug"]');
    const wire = root.querySelector('[data-part="wire"]');
    const reduced = matchMedia("(prefers-reduced-motion: reduce)");
    const shown = { in: -1, ex: -1, spoke: -1, stroke: "" };
    let frame = 0;
    let visible = false;
    let start = 0;

    const draw = (crank: number) => {
      const intake = crank < 180 ? Math.sin((crank / 180) * Math.PI) : 0;
      const exhaust = crank >= 540 ? Math.sin(((crank - 540) / 180) * Math.PI) : 0;
      const nextIn = frameOf(intake, engineFigure.valveFrames.length);
      const nextEx = frameOf(exhaust, engineFigure.exhaustFrames.length);
      if (valveIn && nextIn !== shown.in) {
        valveIn.innerHTML = engineFigure.valveFrames[nextIn];
        shown.in = nextIn;
      }
      if (valveEx && nextEx !== shown.ex) {
        valveEx.innerHTML = engineFigure.exhaustFrames[nextEx];
        shown.ex = nextEx;
      }
      const count = engineFigure.spokeFrames.length;
      const spoke = Math.floor(((crank % 90) / 90) * count) % count;
      if (spoke !== shown.spoke) {
        const next = engineFigure.spokeFrames[spoke];
        spokes?.setAttribute("d", next.spokes);
        mark?.setAttribute("cx", String(next.mark[0]));
        mark?.setAttribute("cy", String(next.mark[1]));
        shown.spoke = spoke;
      }
      const firing = crank > 350 && crank < 400;
      plug?.toggleAttribute("data-firing", firing);
      wire?.toggleAttribute("data-firing", crank > 320 && crank < 380);
      const stroke = strokes[Math.floor(crank / 180) % 4];
      if (stroke !== shown.stroke) {
        shown.stroke = stroke;
        report.current?.(stroke, crank);
      }
    };
    const tick = (now: number) => {
      if (!start) start = now;
      draw((((now - start) % CYCLE_MS) / CYCLE_MS) * 720);
      frame = visible ? requestAnimationFrame(tick) : 0;
    };
    const run = () => {
      if (frame || !visible || still || reduced.matches) return;
      frame = requestAnimationFrame(tick);
    };
    const observer = new IntersectionObserver(([entry]) => {
      visible = entry.isIntersecting;
      if (visible) run();
    });
    observer.observe(root);
    draw(still || reduced.matches ? 380 : 0);
    return () => {
      observer.disconnect();
      cancelAnimationFrame(frame);
    };
  }, [still]);

  return (
    <div
      ref={host}
      className="engine-figure iso"
      data-theme={theme}
      data-focus={focus}
      dangerouslySetInnerHTML={{ __html: engineFigure.svg }}
    />
  );
}

export const engineAnchors = engineFigure.anchors;
export const engineSize = { width: engineFigure.width, height: engineFigure.height };
