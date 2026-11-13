/** Dev only (vite, no native app): a scripted drawing in the same shape
 * format the runtime streams, so the canvas can be designed in a browser. */
const lines = [
  { shape: "title", text: "Turbofan, cutaway" },
  { shape: "slab", x: -260, y: -90, z: -40, w: 520, d: 180, h: 16, r: 10 },
  { shape: "slab", x: -40, y: -14, z: -24, w: 80, d: 28, h: 50, r: 4 },
  { shape: "cylinder", x: -220, y: 0, z: 80, radius: 92, h: 40, axis: "x", label: "fan" },
  { shape: "cylinder", x: -180, y: 0, z: 80, radius: 66, r2: 54, h: 120, axis: "x", label: "compressor" },
  { shape: "cylinder", x: -60, y: 0, z: 80, radius: 58, h: 90, axis: "x", fill: "orange", label: "combustor" },
  { shape: "cylinder", x: 30, y: 0, z: 80, radius: 56, r2: 64, h: 90, axis: "x", fill: "violet", label: "turbine" },
  { shape: "cylinder", x: 120, y: 0, z: 80, radius: 60, r2: 36, h: 110, axis: "x", label: "nozzle" },
  { shape: "cylinder", x: -250, y: 0, z: 80, radius: 22, r2: 6, h: 30, axis: "x" },
  { shape: "arrow", from: [-420, 0, 140], to: [-260, 0, 120], stroke: "blue" },
  { shape: "arrow", from: [240, 0, 80], to: [380, 0, 80], stroke: "red" },
  { shape: "text", x: -400, y: 0, z: 110, text: "Cold air in", size: 15, fill: "blue" },
  { shape: "point", x: -15, y: 0, z: 140, say: "Fuel burns here and heats the air" },
  { shape: "point", x: -200, y: 0, z: 172, say: "The fan pushes most of the thrust" },
];

let started = false;
export function demo(emit: (message: unknown) => void) {
  if (started) return;
  started = true;
  const at = (ms: number, message: unknown) => setTimeout(() => emit(message), ms);
  at(900, { type: "canvas", event: "begin", width: 1000, height: 640 });
  lines.forEach((shape, i) => at(1600 + i * 260, { type: "canvas", event: "shape", shape }));
  at(1600 + lines.length * 260, { type: "canvas", event: "end" });
  at(1700 + lines.length * 260, { type: "answer", text: "A turbofan in cutaway: the fan and compressor squeeze the air, fuel burns in the combustor, the turbine spins and the nozzle pushes the exhaust out." });
}
