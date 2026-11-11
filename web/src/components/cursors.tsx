import type { ReactNode } from "react";

/** The 20 cursors from references/cursors.jpg, redrawn on a 32×32 grid.
 * `hot` is each cursor's hotspot; Astrid's mark trails from there. */
export type Cursor = { name: string; hot: [number, number]; art: ReactNode };

const B = "currentColor";
const line = { fill: "none", stroke: B, strokeWidth: 1.4, strokeLinecap: "round", strokeLinejoin: "round" } as const;
const outlineArrow = "M5 4 26 14 15.5 16.5 11 27Z";
const sparkle = (x: number, y: number, r = 3) => (
  <g {...line} strokeWidth={1.1}>
    <path d={`M${x} ${y - r}v-2M${x - r} ${y - r + 1}l-1.5-1.5M${x + r} ${y - r + 1}l1.5-1.5M${x - r - 1} ${y + 1}h-2`} />
  </g>
);

export const cursors: Cursor[] = [
  {
    name: "Select",
    hot: [6, 6],
    art: (
      <g>
        <rect x="6" y="6" width="20" height="20" {...line} strokeDasharray="2 2.4" />
        {[[6, 6], [16, 6], [26, 6], [6, 16], [26, 16], [6, 26], [16, 26], [26, 26]].map(([x, y]) => (
          <rect key={`${x}${y}`} x={x - 2} y={y - 2} width="4" height="4" fill={B} />
        ))}
      </g>
    ),
  },
  {
    name: "Close",
    hot: [16, 16],
    art: (
      <g>
        <circle cx="16" cy="16" r="12" fill={B} />
        <path d="m11.5 11.5 9 9m0-9-9 9" stroke="#fff" strokeWidth="1.6" strokeLinecap="round" />
      </g>
    ),
  },
  {
    name: "Text select",
    hot: [16, 16],
    art: (
      <g>
        <rect x="3" y="10" width="26" height="12" rx="3" {...line} strokeDasharray="2 2.2" />
        <path d="M13 6c2 0 3 1 3 3v14c0 2 1 3 3 3M19 6c-2 0-3 1-3 3m0 14c0 2-1 3-3 3" {...line} />
      </g>
    ),
  },
  {
    name: "Zoom in",
    hot: [13, 13],
    art: (
      <g>
        <circle cx="13" cy="13" r="10" fill={B} />
        <path d="M13 8v10M8 13h10" stroke="#fff" strokeWidth="1.6" strokeLinecap="round" />
        <path d="m20.5 20.5 7 7" {...line} />
      </g>
    ),
  },
  {
    name: "Zoom out",
    hot: [13, 13],
    art: (
      <g>
        <circle cx="13" cy="13" r="10" fill={B} />
        <path d="M8 13h10" stroke="#fff" strokeWidth="1.6" strokeLinecap="round" />
        <path d="m20.5 20.5 7 7" {...line} />
      </g>
    ),
  },
  {
    name: "Grab",
    hot: [16, 6],
    art: (
      <path
        fill={B}
        d="M9 27v-3l-5.5-7.5c-1-1.5 1-3 2.3-1.8L9 18V7.5a1.8 1.8 0 0 1 3.6 0V15V5a1.8 1.8 0 0 1 3.6 0v10V6.3a1.8 1.8 0 0 1 3.6 0V15V9a1.8 1.8 0 0 1 3.6 0v10.5c0 3-1.4 4.5-3 6V27Z"
      />
    ),
  },
  {
    name: "Crosshair",
    hot: [16, 16],
    art: (
      <g>
        <path d="M16 3v9M16 20v9M3 16h9M20 16h9" {...line} strokeWidth={1.1} />
        <circle cx="16" cy="16" r="1.4" fill={B} />
      </g>
    ),
  },
  {
    name: "Pen",
    hot: [5, 5],
    art: (
      <g>
        <path d="M5 5 19 9l4 9-5 5-9-4Z" {...line} />
        <rect x="11.5" y="11.5" width="3" height="3" fill={B} />
        <path d="m5 5 7 7" {...line} />
        <path d="m20 25 5-5 3 3-5 5Z" fill={B} />
      </g>
    ),
  },
  {
    name: "Resize",
    hot: [16, 16],
    art: (
      <g>
        <path d="M4 16h24" {...line} />
        <path d="M16 5v22" {...line} />
        <path d="m11.5 9 4.5-6 4.5 6Zm0 14 4.5 6 4.5-6Z" fill={B} />
      </g>
    ),
  },
  {
    name: "Click",
    hot: [11, 7],
    art: (
      <g>
        <path d="M11 7 28 25l-6.5.6 4 6.5-3 1.6-4-6.6-4.5 4.4Z" fill={B} transform="scale(.94)" />
        {sparkle(8, 6)}
      </g>
    ),
  },
  {
    name: "Focus",
    hot: [16, 16],
    art: (
      <g {...line} strokeWidth={1}>
        <path d="M16 2v8M16 22v8M2 16h8M22 16h8M7 7l4 4M25 7l-4 4M7 25l4-4M25 25l-4-4M11 4l2 5M21 4l-2 5M4 11l5 2M4 21l5-2M28 11l-5 2M28 21l-5-2M11 28l2-5M21 28l-2-5" />
        <circle cx="16" cy="16" r="2.2" fill={B} stroke="none" />
      </g>
    ),
  },
  {
    name: "Eyedropper",
    hot: [5, 27],
    art: (
      <g>
        <path d="M5 27c0-2 1-3 2-4l9-9 3 3-9 9c-1 1-2 2-4 2Z" {...line} />
        <path d="m15 11 6 6" stroke={B} strokeWidth="3" strokeLinecap="round" />
        <path d="M19 9c2-2 3-5 5-5s4 2 4 4-3 3-5 5Z" fill={B} />
      </g>
    ),
  },
  {
    name: "Arrow",
    hot: [5, 4],
    art: <path d="M5 4 27 14l-11 2.5-6 11Z" fill={B} />,
  },
  {
    name: "Arrow outline",
    hot: [5, 4],
    art: <path d={outlineArrow} {...line} />,
  },
  {
    name: "Not allowed",
    hot: [5, 4],
    art: (
      <g>
        <path d={outlineArrow} {...line} />
        <circle cx="24" cy="8" r="4" {...line} strokeWidth={1.1} />
        <path d="m21.3 10.7 5.4-5.4" {...line} strokeWidth={1.1} />
      </g>
    ),
  },
  {
    name: "Copy file",
    hot: [5, 4],
    art: (
      <g>
        <path d={outlineArrow} {...line} />
        <path d="M19 19h7l2 2v8h-9Z" fill={B} />
        <path d="M21 23h5M21 25.5h5" stroke="#fff" strokeWidth=".9" />
      </g>
    ),
  },
  {
    name: "Arrow click",
    hot: [7, 6],
    art: (
      <g>
        <path d="M7 6 28 16l-10.5 2.5L13 29Z" {...line} />
        {sparkle(5, 5, 2.5)}
      </g>
    ),
  },
  {
    name: "Pointer tap",
    hot: [12, 5],
    art: (
      <g>
        <path {...line} strokeWidth={1.1} d="M12 2.4V.6M8.6 3.8 7 2.3M15.4 3.8 17 2.3M7.6 6.6H5.4" />
        <path
          fill={B}
          d="M10 7.5a2 2 0 0 1 4 0V15h.5v-2a2 2 0 0 1 4 0v2h.5v-1a2 2 0 0 1 4 0v1.5h.3a2 2 0 0 1 3.7 1V22c0 4-2.5 7-6.5 7H15c-2.5 0-4-1-5.5-3L4.6 20c-1-1.6.8-3.4 2.4-2.3L10 20Z"
        />
      </g>
    ),
  },
  {
    name: "Press button",
    hot: [12, 7],
    art: (
      <g>
        <path {...line} strokeWidth={1.1} d="M7.5 6.5a4.5 4.5 0 0 1 9 0M4.5 6.5a7.5 7.5 0 0 1 15 0" />
        <path {...line} d="M10 7.5a2 2 0 0 1 4 0V16h.5v-2a2 2 0 0 1 4 0v2h.5v-1a2 2 0 0 1 4 0v1.5a2 2 0 0 1 3.5 1.2V23c0 4-2.5 6.5-6.5 6.5H15c-2.4 0-3.8-1-5.3-3l-4.4-6.4c-1-1.6.8-3.3 2.4-2.2L10 20Z" />
      </g>
    ),
  },
  {
    name: "Tap",
    hot: [12, 9],
    art: (
      <g>
        <rect x="4" y="2" width="16" height="10" rx="5" {...line} />
        <path {...line} d="M10 7.5a2 2 0 0 1 4 0V16h.5v-2a2 2 0 0 1 4 0v2h.5v-1a2 2 0 0 1 4 0v1.5a2 2 0 0 1 3.5 1.2V23c0 4-2.5 6.5-6.5 6.5H15c-2.4 0-3.8-1-5.3-3l-4.4-6.4c-1-1.6.8-3.3 2.4-2.2L10 20Z" />
      </g>
    ),
  },
];


/** Astrid's own cursor across the site: the outline arrow with click sparks. */
export const astridCursor = cursors.find((cursor) => cursor.name === "Arrow click")!;
