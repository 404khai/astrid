// Hand-drawn stroke helpers: deterministic jitter so sketches look drawn
// but render identically on every load.
const noise = (seed: number) => {
  const value = Math.sin(seed * 127.1) * 43758.5453;
  return value - Math.floor(value);
};

export function sketchEllipse(cx: number, cy: number, rx: number, ry: number, seed = 1) {
  const points: string[] = [];
  const turns = 1.12;
  const steps = 44;
  for (let index = 0; index <= steps; index++) {
    const t = index / steps;
    const angle = -2.2 + t * turns * Math.PI * 2;
    const wobble = 1 + (noise(seed + index) - 0.5) * 0.08 + t * 0.06;
    const x = cx + Math.cos(angle) * rx * wobble;
    const y = cy + Math.sin(angle) * ry * wobble;
    points.push(`${index ? "L" : "M"}${x.toFixed(1)} ${y.toFixed(1)}`);
  }
  return points.join("");
}

export function sketchArrow(
  [x1, y1]: [number, number],
  [x2, y2]: [number, number],
  bend = 0.25,
) {
  const mx = (x1 + x2) / 2 - (y2 - y1) * bend;
  const my = (y1 + y2) / 2 + (x2 - x1) * bend;
  const angle = Math.atan2(y2 - my, x2 - mx);
  const head = (offset: number) =>
    `M${x2.toFixed(1)} ${y2.toFixed(1)}L${(x2 - Math.cos(angle + offset) * 11).toFixed(1)} ${(y2 - Math.sin(angle + offset) * 11).toFixed(1)}`;
  return {
    line: `M${x1} ${y1}Q${mx.toFixed(1)} ${my.toFixed(1)} ${x2} ${y2}`,
    head: head(0.45) + head(-0.45),
  };
}
