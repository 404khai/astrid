/** Dev only (vite, no native app): a scripted drawing in the same shape
 * format the runtime streams, so the canvas can be designed in a browser. */
const lines = [
  { shape: "title", text: "A tiny desk setup" },
  { shape: "box", x: 470, y: 470, w: 300, d: 180, h: 18, fill: "#8a6a4a", label: "desk" },
  { shape: "box", x: 600, y: 300, w: 150, d: 12, h: 100, fill: "paper", stroke: "blue", label: "monitor" },
  { shape: "box", x: 540, y: 410, w: 90, d: 40, h: 6, fill: "ink" },
  { shape: "ellipse", cx: 410, cy: 390, rx: 22, ry: 12, fill: "pink" },
  { shape: "path", d: "M392 390 C392 420 428 420 428 390", stroke: "pink", width: 3 },
  { shape: "path", d: "M300 330 C 280 260 330 230 320 180 M300 330 C 340 280 360 270 380 220", stroke: "green", width: 3, sketchy: true },
  { shape: "ellipse", cx: 300, cy: 345, rx: 30, ry: 14, fill: "orange" },
  { shape: "text", x: 80, y: 80, text: "Light from the left", size: 16, fill: "yellow" },
  { shape: "arrow", from: [150, 100], to: [300, 230], sketchy: true, stroke: "yellow" },
  { shape: "rect", x: 820, y: 120, w: 140, h: 80, r: 12, dashed: true, stroke: "teal" },
  { shape: "text", x: 890, y: 166, text: "cable tray", size: 14, align: "middle", fill: "teal" },
  { shape: "arrow", from: [880, 210], to: [760, 400], bend: -0.2, stroke: "teal" },
  { shape: "point", x: 680, y: 260, say: "Monitor at eye height" },
  { shape: "point", x: 330, y: 300, say: "A plant for the corner" },
];

let started = false;
export function demo(emit: (message: unknown) => void) {
  if (started) return;
  started = true;
  const at = (ms: number, message: unknown) => setTimeout(() => emit(message), ms);
  at(900, { type: "canvas", event: "begin", width: 1000, height: 640 });
  lines.forEach((shape, i) => at(1600 + i * 260, { type: "canvas", event: "shape", shape }));
  at(1600 + lines.length * 260, { type: "canvas", event: "end" });
  at(1700 + lines.length * 260, { type: "answer", text: "I sketched a compact desk: monitor at eye height, a plant in the corner and a cable tray." });
}
