// Fluffy Astrid, drawn by open-bot-avatar (MIT, github.com/DominikScholz/open-bot-avatar): the body gets
// real volume and a coat of fur, breathes, hops and turns toward the pointer. Its fluffy material is
// wrapped so the eyes it seats in the fur are Astrid's own, re-cut every frame for the current mood.
import { useEffect, useMemo, useRef } from "react";
import { mountAvatar, type AvatarOptions, type MountedAvatar } from "open-bot-avatar/dom";
import { fluffy } from "open-bot-avatar/materials/fluffy";
import { validateAvatar, type AvatarDefinition } from "open-bot-avatar";
import { avatarDefinition, toAvatar, withStandIn, type Corners, type Form } from "./forms";
import { ANIMATED, FACES, eyePath, faceAt, type Mood } from "./faces";
import type { Shape } from "./Astrid";

type Pose = Pick<AvatarOptions, "state" | "mood">;
type Material = typeof fluffy;
const HAPPY = new Set<Mood>(["happy", "laughing", "love", "wink", "posting", "voice"]);
const WORKING = new Set<Mood>(["loading", "syncing", "scanning", "searching", "calculating", "catching"]);
const WAITING = new Set<Mood>(["waiting", "sleeping", "bored", "low-battery"]);

/** How the body moves for an Astrid mood. The eyes are always Astrid's, so no state swaps the body for a mark. */
export function poseFor(m: Mood): Pose {
  if (m === "celebrate") return { state: "done", mood: "happy" };
  if (HAPPY.has(m)) return { state: "idle", mood: "happy" };
  if (WORKING.has(m)) return { state: "working", mood: "neutral" };
  if (WAITING.has(m)) return { state: "waiting", mood: "neutral" };
  return { state: "idle", mood: "neutral" };
}

/** An eye outline from Astrid's 512 box into the avatar's 100 box (eyePath writes absolute M/L/Z pairs only). */
const toFace = (d: string) => d.replace(/(-?\d+(?:\.\d+)?) (-?\d+(?:\.\d+)?)/g, (_, x, y) => {
  const [ax, ay] = toAvatar([+x, +y]);
  return `${ax.toFixed(2)} ${ay.toFixed(2)}`;
});

/** The fluffy material with Astrid's eyes in place of the package's own. */
/**
 * Arches and dual have no fur behind the eyes, and the fur renderer only seats eyes in fur. There the
 * coat is rendered without eyes, copied, and Astrid's eyes are painted onto the copy as glossy beads,
 * so the package's hop, roll and squash carry body and eyes together.
 */
function paintEyes(img: ReturnType<Material["render"]> & object, paths: string[], ink: string, view: { cY: number; cP: number }, dx: number, dy: number) {
  const out = (paintEyes as { canvas?: HTMLCanvasElement }).canvas ??= document.createElement("canvas");
  if (out.width !== img.size) out.width = out.height = img.size;
  const ctx = out.getContext("2d")!;
  ctx.setTransform(1, 0, 0, 1, 0, 0);
  ctx.clearRect(0, 0, img.size, img.size);
  ctx.drawImage(img.canvas, img.x, img.y, img.size, img.size, 0, 0, img.size, img.size);
  // face plane to image pixels; the eyes sit on the mid-plane, so a turn only foreshortens them about the centre
  const px = img.size / (2 * img.radius);
  ctx.setTransform(px * view.cY, 0, 0, px * view.cP, (50 + dx - img.cx + img.radius) * px - 50 * px * view.cY, (50 + dy - img.cy + img.radius) * px - 50 * px * view.cP);
  for (const d of paths) {
    const eye = new Path2D(d);
    ctx.save();
    ctx.shadowColor = "rgba(0,0,0,0.45)";
    ctx.shadowBlur = 3 * px;
    ctx.shadowOffsetY = 1.2 * px;
    ctx.fillStyle = ink;
    ctx.fill(eye);
    ctx.restore();
    // gloss: a soft light from the upper left, clipped to the eye
    ctx.save();
    ctx.clip(eye);
    const [x0, y0, x1, y1] = bounds(d);
    const g = ctx.createRadialGradient(x0 + (x1 - x0) * 0.3, y0 + (y1 - y0) * 0.22, 0, x0 + (x1 - x0) * 0.3, y0 + (y1 - y0) * 0.22, Math.max(x1 - x0, y1 - y0) * 0.7);
    g.addColorStop(0, "rgba(255,255,255,0.75)");
    g.addColorStop(0.18, "rgba(255,255,255,0.28)");
    g.addColorStop(1, "rgba(255,255,255,0)");
    ctx.fillStyle = g;
    ctx.fillRect(x0, y0, x1 - x0, y1 - y0);
    const shade = ctx.createLinearGradient(0, y0, 0, y1);
    shade.addColorStop(0.55, "rgba(0,0,0,0)");
    shade.addColorStop(1, "rgba(0,0,0,0.35)");
    ctx.fillStyle = shade;
    ctx.fillRect(x0, y0, x1 - x0, y1 - y0);
    ctx.restore();
  }
  return { ...img, canvas: out, x: 0, y: 0 };
}

/** Bounding box of an absolute M/L path. */
function bounds(d: string): [number, number, number, number] {
  const n = d.match(/-?\d+(?:\.\d+)?/g)!.map(Number);
  let x0 = Infinity, y0 = Infinity, x1 = -Infinity, y1 = -Infinity;
  for (let i = 0; i < n.length - 1; i += 2) { x0 = Math.min(x0, n[i]); x1 = Math.max(x1, n[i]); y0 = Math.min(y0, n[i + 1]); y1 = Math.max(y1, n[i + 1]); }
  return [x0, y0, x1, y1];
}

function astridFur(mood: { current: Mood }, gaze: { current: number }, overlay: { current: boolean }): Material {
  const start = performance.now();
  return {
    ...fluffy,
    id: "astrid-fluffy",
    render(shape, view, look, scale) {
      // a state's mark has no eyes; leave it as it is
      if (!look.eyes.length) return fluffy.render(shape, view, look, scale);
      const m = mood.current, t = (performance.now() - start) / 1000;
      const face = ANIMATED.has(m) ? faceAt(m, t) : FACES[m];
      const paths = (["L", "R"] as const).map((side) => toFace(eyePath(side, face[side])));
      // keep the package's gaze, so the eyes still follow the pointer
      const gx = (look.eyes[0]?.dx ?? 0) * gaze.current, gy = (look.eyes[0]?.dy ?? 0) * gaze.current;
      if (overlay.current) {
        const img = fluffy.render(shape, view, { ...look, eyes: [] }, scale);
        return img && paintEyes(img, paths, look.ink, view as unknown as { cY: number; cP: number }, gx, gy);
      }
      const eyes = paths.map((d, i) => ({ mark: { shape: "path" as const, d }, dx: (look.eyes[i]?.dx ?? 0) * gaze.current, dy: (look.eyes[i]?.dy ?? 0) * gaze.current }));
      return fluffy.render(shape, view, { ...look, eyes }, scale);
    },
  };
}

interface Props { form: Form; corners: Corners; shape: Shape; color: string; eyes: string; mood: Mood; title: string }

export function FurBot({ form, corners, shape, color, eyes, mood, title }: Props) {
  const host = useRef<HTMLDivElement>(null);
  const bot = useRef<MountedAvatar | null>(null);
  const live = useRef(mood);
  live.current = mood;
  // the stand-in eyes in arches and dual are a third the size, so their gaze travels a third as far
  const gaze = useRef(1);
  const overlay = useRef(false);
  const material = useMemo(() => astridFur(live, gaze, overlay), []);
  // a few curved forms have no arch thick enough for even the stand-in eyes: those fall back to the full body
  const [definition, body] = useMemo(() => {
    const d = withStandIn(avatarDefinition(form, corners, shape), (x) => validateAvatar(x).ok);
    return validateAvatar(d).ok ? [d, shape] : [avatarDefinition(form, corners, "full"), "full" as Shape];
  }, [form, corners, shape]) as [AvatarDefinition, Shape];
  gaze.current = body === "full" ? 1 : 3;
  overlay.current = body !== "full";
  const options: Partial<AvatarOptions> = { definition, color, eyeColor: eyes, ...poseFor(mood), label: title };

  useEffect(() => {
    bot.current = mountAvatar(host.current!, { ...options, definition, material, followPointer: true, depth: 8 } as AvatarOptions);
    return () => { bot.current?.destroy(); bot.current = null; };
  }, []);
  useEffect(() => { bot.current?.update(options); }, [form, corners, shape, color, eyes, mood, title]);

  return <div ref={host} className="furbot" />;
}
