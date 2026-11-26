/** Dev only (vite, no native app): scripted drawings in the same shape
 * format the runtime streams, so the canvas can be designed in a browser.
 * `?demo=dam` or `?demo=paper` picks another scene; the turbofan is default. */
const turbofan = [
  { shape: "title", text: "Turbofan, cutaway" },
  { shape: "slab", x: -260, y: -90, z: -40, w: 520, d: 180, h: 16, r: 10 },
  { shape: "slab", x: -40, y: -14, z: -24, w: 80, d: 28, h: 50, r: 4 },
  { shape: "cylinder", x: -230, y: 0, z: 80, radius: 98, h: 30, axis: "x" },
  { shape: "rotor", x: -205, y: 0, z: 80, radius: 90, blades: 14, hub: 22, axis: "x", pitch: 30, motion: "spin", speed: 2.4, label: "fan" },
  { shape: "cylinder", x: -180, y: 0, z: 80, radius: 66, r2: 54, h: 120, axis: "x", label: "compressor" },
  { shape: "cylinder", x: -60, y: 0, z: 80, radius: 58, h: 90, axis: "x", fill: "orange", glow: true, motion: "pulse", label: "combustor" },
  { shape: "cylinder", x: 30, y: 0, z: 80, radius: 56, r2: 64, h: 90, axis: "x", label: "turbine" },
  { shape: "cylinder", x: 120, y: 0, z: 80, radius: 60, r2: 36, h: 110, axis: "x", label: "nozzle" },
  { shape: "line", points: [[-420, 0, 150], [-240, 0, 150], [-60, 0, 150], [240, 0, 120], [400, 0, 110]], stroke: "blue", width: 2, motion: "flow", speed: 1.6 },
  { shape: "line", points: [[-420, 0, 30], [-240, 0, 20], [240, 0, 40], [400, 0, 50]], stroke: "blue", width: 2, motion: "flow", speed: 1.6 },
  { shape: "arrow", from: [240, 0, 80], to: [400, 0, 80], stroke: "red", motion: "flow", speed: 2 },
  { shape: "callout", x: -15, y: 0, z: 140, at: [650, 70], title: "Combustor", tag: "CB-2", rows: [["Role", "Burns fuel in air"], ["Temp", "≈ 1,700 °C"], ["Out", "Hot gas to turbine"]] },
  { shape: "point", x: -205, y: 0, z: 172, say: "The fan pushes most of the thrust" },
];

const dam = [
  { shape: "title", text: "Hydroelectric dam" },
  { shape: "slab", x: -320, y: -200, z: -20, w: 640, d: 400, h: 20, r: 14 },
  { shape: "slab", x: -320, y: -180, z: 0, w: 250, d: 360, h: 170, fill: "#2a5bd7", label: "reservoir" },
  { shape: "slab", x: -70, y: -200, z: 0, w: 90, d: 400, h: 90 },
  { shape: "slab", x: -60, y: -200, z: 90, w: 60, d: 400, h: 140, label: "dam" },
  { shape: "slab", x: -64, y: -200, z: 230, w: 68, d: 400, h: 8 },
  { shape: "line", points: [[-140, 70, 120], [-60, 70, 50], [40, 70, 30], [110, 70, 46]], stroke: "blue", width: 3, motion: "flow", speed: 1.8, label: "penstock" },
  { shape: "slab", x: 60, y: 10, z: 0, w: 120, d: 120, h: 36, r: 6 },
  { shape: "rotor", x: 120, y: 70, z: 50, radius: 42, blades: 9, hub: 12, axis: "z", pitch: 35, motion: "spin", speed: 1.4, fill: "orange", glow: true, label: "turbine" },
  { shape: "cylinder", x: 120, y: 70, z: 70, radius: 30, h: 60, label: "generator" },
  { shape: "slab", x: 180, y: 20, z: 0, w: 140, d: 100, h: 8, fill: "#2a5bd7" },
  { shape: "line", points: [[150, 70, 10], [320, 70, 10]], stroke: "blue", width: 2, motion: "flow", speed: 1.2 },
  { shape: "line", points: [[-4, -140, 236], [30, -140, 120], [70, -140, 10], [320, -140, 10]], stroke: "teal", width: 3, motion: "flow", speed: 2.4, label: "spillway" },
  { shape: "cylinder", x: 250, y: -40, z: 0, radius: 6, r2: 3, h: 180 },
  { shape: "line", points: [[120, 70, 130], [250, -40, 180], [320, -120, 170]], stroke: "yellow", width: 1.5, motion: "flow", speed: 3, dashed: true },
  { shape: "callout", x: 120, y: 70, z: 50, at: [660, 80], title: "Turbine", tag: "U-3", rows: [["Role", "Turns falling water"], ["Head", "160 m"], ["Output", "120 MW"]] },
  { shape: "point", x: -100, y: 70, z: 100, say: "Water falls through the penstock" },
  { shape: "point", x: 120, y: 70, z: 130, say: "The spinning turbine drives the generator" },
];

const paper = [
  { shape: "canvas", background: "#f4efe6", grid: false },
  ...dam.slice(0, -2).map((s) => (s.shape === "title" ? { ...s, text: "Hydroelectric dam, on paper" } : s)),
];

const scenes: Record<string, unknown[]> = { turbofan, dam, paper };

let started = false;
export function demo(emit: (message: unknown) => void) {
  if (started) return;
  started = true;
  const lines = scenes[new URLSearchParams(location.search).get("demo") ?? ""] ?? turbofan;
  const at = (ms: number, message: unknown) => setTimeout(() => emit(message), ms);
  at(900, { type: "canvas", event: "begin", width: 1000, height: 640 });
  lines.forEach((shape, i) => at(1600 + i * 260, { type: "canvas", event: "shape", shape }));
  at(1600 + lines.length * 260, { type: "canvas", event: "end" });
}
