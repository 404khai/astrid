import { useEffect, useRef } from "react";

/** 8×8 Bayer matrix, normalised to thresholds in (0, 1). */
const BAYER = [
  0, 32, 8, 40, 2, 34, 10, 42, 48, 16, 56, 24, 50, 18, 58, 26, 12, 44, 4, 36,
  14, 46, 6, 38, 60, 28, 52, 20, 62, 30, 54, 22, 3, 35, 11, 43, 1, 33, 9, 41,
  51, 19, 59, 27, 49, 17, 57, 25, 15, 47, 7, 39, 13, 45, 5, 37, 63, 31, 55, 23,
  61, 29, 53, 21,
].map((value) => (value + 0.5) / 64);

const CELL = 3;
const FPS = 18;

function hash(x: number, y: number) {
  const h = Math.sin(x * 127.1 + y * 311.7) * 43758.5453;
  return h - Math.floor(h);
}
function noise(x: number, y: number) {
  const ix = Math.floor(x);
  const iy = Math.floor(y);
  const fx = x - ix;
  const fy = y - iy;
  const ux = fx * fx * (3 - 2 * fx);
  const uy = fy * fy * (3 - 2 * fy);
  const a = hash(ix, iy);
  const b = hash(ix + 1, iy);
  const c = hash(ix, iy + 1);
  const d = hash(ix + 1, iy + 1);
  return a + (b - a) * ux + (c - a) * uy + (a - b - c + d) * ux * uy;
}
function fbm(x: number, y: number) {
  return noise(x, y) * 0.55 + noise(x * 2.1, y * 2.1) * 0.3 + noise(x * 4.3, y * 4.3) * 0.15;
}

/** A slowly drifting landscape — ridges, a spire and fog — ordered-dithered
 * into a 1-bit grid, like an old Mac screen that is still breathing. */
export function DitherField({ className = "" }: { className?: string }) {
  const canvas = useRef<HTMLCanvasElement>(null);
  useEffect(() => {
    const element = canvas.current;
    const context = element?.getContext("2d");
    if (!element || !context) return;
    const buffer = document.createElement("canvas");
    const small = buffer.getContext("2d");
    if (!small) return;
    const reduced = matchMedia("(prefers-reduced-motion: reduce)");
    let width = 0;
    let height = 0;
    let image: ImageData | null = null;
    let frame = 0;
    let last = 0;
    let visible = false;

    const resize = () => {
      const box = element.getBoundingClientRect();
      width = Math.max(1, Math.round(box.width / CELL));
      height = Math.max(1, Math.round(box.height / CELL));
      buffer.width = width;
      buffer.height = height;
      element.width = width * CELL;
      element.height = height * CELL;
      image = small.createImageData(width, height);
    };

    const draw = (time: number) => {
      if (!image) return;
      const t = time / 1000;
      const data = image.data;
      for (let y = 0; y < height; y++) {
        const v = y / height;
        for (let x = 0; x < width; x++) {
          const u = x / width;
          // sky: brighter toward the horizon, clouds drifting left
          let light = 0.03 + Math.max(0, v - 0.15) * 0.26;
          light += (fbm(u * 3 + t * 0.05, v * 5) - 0.45) * 0.28 * (1 - v);
          // far ridge, near ridge: parallax at different speeds
          const far = 0.55 - fbm(u * 2.4 + t * 0.012, 3.1) * 0.22;
          const near = 0.74 - fbm(u * 3.6 + t * 0.03, 7.7) * 0.2;
          if (v > far) light = 0.34 + (v - far) * 0.25 - fbm(u * 9, v * 9) * 0.12;
          if (v > near) light = 0.16 + fbm(u * 14 + t * 0.03, v * 10) * 0.1;
          // a spire and its mast, slightly right of centre
          const spire = Math.abs(u - 0.66);
          if (v > 0.22 && spire < 0.012 + (v - 0.22) * 0.05 && v < far + 0.08) light = 0.42;
          if (spire < 0.002 && v > 0.08 && v <= 0.22) light = 0.5;
          // fog band rolling across the valley
          const fog = Math.exp(-((v - 0.68) ** 2) / 0.004) * fbm(u * 4 - t * 0.08, v * 3 + t * 0.02);
          light += fog * 0.3;
          // soft vignette
          light *= 1 - Math.hypot(u - 0.5, v - 0.55) * 0.55;
          const on = light > BAYER[(y & 7) * 8 + (x & 7)];
          const index = (y * width + x) * 4;
          const tint = on ? (fog > 0.25 ? 1 : 0) : 0;
          data[index] = on ? 64 - tint * 12 : 22;
          data[index + 1] = on ? 64 - tint * 4 : 22;
          data[index + 2] = on ? 68 + tint * 52 : 23;
          data[index + 3] = 255;
        }
      }
      small.putImageData(image, 0, 0);
      context.imageSmoothingEnabled = false;
      context.drawImage(buffer, 0, 0, element.width, element.height);
    };

    const tick = (now: number) => {
      if (now - last >= 1000 / FPS) {
        last = now;
        draw(now);
      }
      frame = visible ? requestAnimationFrame(tick) : 0;
    };
    const run = () => {
      if (!frame && visible && !reduced.matches) frame = requestAnimationFrame(tick);
    };
    const sizes = new ResizeObserver(() => {
      resize();
      draw(performance.now());
    });
    sizes.observe(element);
    const views = new IntersectionObserver(([entry]) => {
      visible = entry.isIntersecting;
      run();
    });
    views.observe(element);
    return () => {
      sizes.disconnect();
      views.disconnect();
      cancelAnimationFrame(frame);
    };
  }, []);
  return <canvas ref={canvas} className={`dither-field ${className}`} aria-hidden="true" />;
}
