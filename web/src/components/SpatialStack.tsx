import { AstridCursor } from "./AstridCursor";
import { useInView } from "./Bento";
import { sketchArrow, sketchEllipse } from "./sketch";

const layers = [
  {
    id: "presence",
    index: "03",
    name: "Presence",
    detail: "Cursor, voice and subtitles. Always visibly Astrid.",
  },
  {
    id: "ink",
    index: "02",
    name: "Ink",
    detail: "Sketches, highlights and figures, drawn above your apps.",
  },
  {
    id: "screen",
    index: "01",
    name: "Your screen",
    detail: "Untouched. Seen one frame at a time, only when you ask.",
  },
];

/** Exploded view of how Astrid layers over the desktop. */
export function SpatialStack() {
  const { ref, seen } = useInView<HTMLDivElement>();
  const arrow = sketchArrow([96, 150], [196, 96], 0.2);
  return (
    <div ref={ref} className={`spatial ${seen ? "open" : ""}`}>
      <div className="spatial-scene" aria-hidden="true">
        <div className="plane plane-screen">
          <div className="plane-window">
            <span className="traffic">
              <i />
              <i />
              <i />
            </span>
            <i className="reader-line" style={{ width: "62%" }} />
            <i className="reader-line" style={{ width: "80%" }} />
            <i className="reader-line" style={{ width: "44%" }} />
            <span className="plane-chart">
              {[40, 64, 52, 82, 70].map((height, index) => (
                <b key={index} style={{ height: `${height}%` }} />
              ))}
            </span>
          </div>
          <div className="plane-window small">
            <i className="reader-line" style={{ width: "70%" }} />
            <i className="reader-line" style={{ width: "50%" }} />
          </div>
        </div>
        <div className="plane plane-ink">
          <svg viewBox="0 0 320 220">
            <path d={sketchEllipse(232, 132, 48, 40, 4)} />
            <path d={arrow.line + arrow.head} />
            <path d="M40 176h120" className="soft" />
          </svg>
        </div>
        <div className="plane plane-presence">
          <AstridCursor x={0} y={0} className="plane-cursor" speaking />
          <span className="plane-subtitle">“This bar is the outlier. Here’s why.”</span>
        </div>
      </div>
      <ol className="spatial-legend">
        {layers.map((layer) => (
          <li key={layer.id} data-layer={layer.id}>
            <span>{layer.index}</span>
            <div>
              <h3>{layer.name}</h3>
              <p>{layer.detail}</p>
            </div>
          </li>
        ))}
      </ol>
    </div>
  );
}
