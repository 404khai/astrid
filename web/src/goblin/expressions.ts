// Vendored from github.com/404khai/goblins (9e63cf1, MIT, Copyright (c) 2026 Khai; see web/src/goblin/LICENSE).
import * as THREE from "three";
import type { MouthKind, PropKind, Rig } from "./build";
import type { ExpressionId } from "./types";

type EyeMode = "open" | "happy" | "closed";

interface Pose {
  eyes: [EyeMode, EyeMode]; // [left, right]
  lid: number; // 0 wide open … 1 half shut
  tilt: number; // lid slant, + angry / - sad
  pupil: number;
  brow: [number, number]; // [lift, slant] applied to both
  browL?: [number, number]; // optional asymmetric left brow
  mouth: MouthKind;
}

interface Out {
  pos: THREE.Vector3;
  rot: THREE.Vector3;
  look: THREE.Vector2 | null;
  hands: THREE.Vector3[];
  mouth: number;
  squash: number;
}

/** A single still frame of the expression, used by the SVG renderer. */
export interface Still {
  look?: [number, number];
  tilt?: number;
  hands?: [[number, number] | null, [number, number] | null];
  mouth?: number;
}

export interface Expression {
  label: string;
  blurb: string;
  pose: Pose;
  props?: PropKind[];
  /** Show the floating mitts. Off unless the pose needs them (holding, rubbing, scratching). */
  hands?: boolean;
  still?: Still;
  motion: (t: number, o: Out, r: Rig) => void;
}

export type { Pose, EyeMode };

const sin = Math.sin, abs = Math.abs, cos = Math.cos;
const pulse = (t: number, period: number, len: number) => (t % period) < len;
const P = (p: Partial<Pose>): Pose => ({ eyes: ["open", "open"], lid: 0, tilt: 0, pupil: 1, brow: [0, 0], mouth: "smile", ...p });

export const EXPRESSIONS: Record<ExpressionId, Expression> = {
  idle: {
    label: "Idle", blurb: "Breathes, follows your cursor, blinks.",
    pose: P({}),
    motion: (t, o) => {
      o.pos.y = sin(t * 1.6) * 0.06;
      o.squash = 1 + sin(t * 1.6) * 0.015;
      o.hands.forEach((h, i) => (h.y += sin(t * 1.6 + i) * 0.05));
    },
  },
  happy: {
    label: "Happy", blurb: "Bouncy hop with sparkles.",
    pose: P({ eyes: ["happy", "happy"], brow: [0.06, -0.05], mouth: "grin" }),
    props: ["sparkles"],
    motion: (t, o, r) => {
      const b = abs(sin(t * 4));
      o.pos.y = b * 0.18;
      o.squash = 0.96 + b * 0.06;
      o.rot.z = sin(t * 4) * 0.06;
      o.hands.forEach((h, i) => { h.y += 0.3 + b * 0.2; h.x *= 1.05; h.z += 0.1 * i; });
      orbit(r.props.sparkles, t, 1.6, r.top * 0.4);
    },
  },
  laughing: {
    label: "Laughing", blurb: "Head thrown back, shaking with it.",
    pose: P({ eyes: ["happy", "happy"], brow: [0.1, -0.15], mouth: "grin" }),
    motion: (t, o) => {
      o.pos.y = abs(sin(t * 14)) * 0.07 + 0.05;
      o.rot.x = -0.22 + sin(t * 14) * 0.03;
      o.mouth = 1.1 + abs(sin(t * 14)) * 0.35;
      o.hands[0].set(-0.55, -1.15 + abs(sin(t * 14)) * 0.05, 0.85);
      o.hands[1].set(0.55, -1.15 + abs(cos(t * 14)) * 0.05, 0.85);
    },
  },
  love: {
    label: "In love", blurb: "Swoons side to side, hearts drift up.",
    pose: P({ lid: 0.35, pupil: 1.2, brow: [0.08, -0.2], mouth: "smile" }),
    props: ["hearts"],
    motion: (t, o, r) => {
      o.rot.z = sin(t * 1.8) * 0.14;
      o.pos.x = sin(t * 1.8) * 0.08;
      o.hands[0].set(-0.75, -0.55, 0.85);
      o.hands[1].set(0.75, -0.55, 0.85);
      r.props.hearts.children.forEach((h, i) => {
        const k = (t * 0.45 + i / 3) % 1;
        h.position.set(sin(k * 9 + i * 2) * 0.5 + (i - 1) * 0.55, k * 1.6, 0);
        h.scale.setScalar((i === 0 ? 1 : i === 1 ? 0.8 : 0.65) * Math.min(1, (1 - k) * 4) * (1 + sin(t * 8) * 0.06));
      });
    },
  },
  wink: {
    label: "Wink", blurb: "Cheeky wink and a nod.",
    pose: P({ eyes: ["open", "happy"], brow: [0.04, 0], browL: [0.12, -0.15], mouth: "smirk" }),
    motion: (t, o) => {
      const k = (t % 2.4) / 2.4;
      o.rot.z = -0.12;
      o.rot.x = k < 0.15 ? sin((k / 0.15) * Math.PI) * 0.15 : 0;
      o.hands[1].set(1.05, -0.4 + sin(t * 3) * 0.04, 0.75);
    },
  },
  surprised: {
    label: "Surprised", blurb: "Jolts upright, hands fly up.",
    pose: P({ lid: -0.15, pupil: 0.55, brow: [0.17, -0.1], mouth: "o" }),
    props: ["exclaim"],
    motion: (t, o, r) => {
      const k = (t % 3) / 3;
      const jolt = k < 0.1 ? sin((k / 0.1) * Math.PI) : 0;
      o.pos.y = 0.12 + jolt * 0.3;
      o.squash = 1.04 - jolt * 0.08;
      o.mouth = 1 + jolt * 0.4;
      o.hands[0].set(-1.3, -0.05 + jolt * 0.2, 0.6);
      o.hands[1].set(1.3, -0.05 + jolt * 0.2, 0.6);
      r.props.exclaim.scale.setScalar(k < 0.1 ? k * 10 : 1);
      r.props.exclaim.rotation.z = sin(t * 10) * 0.08;
    },
  },
  thinking: {
    hands: true,
    label: "Thinking", blurb: "Chin rub, eyes up, thought bubbles.",
    pose: P({ lid: 0.3, brow: [0.02, 0.05], browL: [0.14, -0.2], mouth: "smirk" }),
    props: ["dots"],
    motion: (t, o, r) => {
      o.look = new THREE.Vector2(0.7, 0.85);
      o.rot.z = 0.12;
      o.rot.y = 0.18 + sin(t * 0.8) * 0.05;
      o.hands[1].set(0.35, -1.0 + abs(sin(t * 5)) * 0.06, 1.05);
      r.props.dots.children.forEach((d, i) => {
        const k = ((t * 0.9) % 3) - i * 0.5;
        d.scale.setScalar(THREE.MathUtils.clamp(k * 3, 0, 1) * (1 + sin(t * 3 + i) * 0.06));
      });
    },
  },
  searching: {
    hands: true,
    label: "Searching", blurb: "Scans the room with a magnifying glass.",
    pose: P({ lid: 0.15, brow: [0.06, 0.05], browL: [-0.02, 0.2], mouth: "flat" }),
    props: ["magnifier"],
    motion: (t, o, r) => {
      const s = sin(t * 1.3);
      const step = Math.round(s * 2) / 2; // pupils snap between spots
      o.look = new THREE.Vector2(step, -0.15);
      o.rot.y = s * 0.45;
      o.pos.z = 0.1;
      o.pos.x = s * 0.15;
      o.hands[1].set(0.4 + s * 0.85, -0.35, 1.25);
      o.hands[0].set(-0.9, -1.15, 0.55);
      r.props.magnifier.rotation.z = -s * 0.25;
    },
  },
  confused: {
    hands: true,
    label: "Confused", blurb: "Head tilt, scratching under the hat.",
    pose: P({ pupil: 0.85, brow: [0.02, 0.15], browL: [0.18, -0.25], mouth: "smirk" }),
    props: ["question"],
    motion: (t, o, r) => {
      o.rot.z = 0.2 + sin(t * 1.2) * 0.05;
      o.look = new THREE.Vector2(sin(t * 0.7) * 0.4, 0.3);
      o.hands[1].set(1.1, 0.55 + abs(sin(t * 12)) * 0.08, 0.3);
      r.props.question.rotation.z = sin(t * 2) * 0.2;
      r.props.question.position.y = r.top + 0.15 + sin(t * 2.4) * 0.08;
    },
  },
  nervous: {
    label: "Nervous", blurb: "Trembles, darting eyes, sweat drop.",
    pose: P({ lid: 0.05, pupil: 0.65, brow: [0.08, -0.3], mouth: "flat" }),
    props: ["sweat"],
    motion: (t, o, r) => {
      o.pos.x = sin(t * 38) * 0.015;
      o.look = new THREE.Vector2(pulse(t, 1.4, 0.7) ? -0.8 : 0.8, -0.1);
      o.mouth = 1 + sin(t * 20) * 0.15;
      o.hands[0].set(-0.3, -1.15, 0.95);
      o.hands[1].set(0.3, -1.15, 0.95);
      const k = (t * 0.5) % 1;
      r.props.sweat.position.y = 0.45 - k * 0.5;
      r.props.sweat.scale.setScalar(Math.min(1, (1 - k) * 4));
    },
  },
  sad: {
    label: "Sad", blurb: "Droops, looks down, a tear rolls.",
    pose: P({ lid: 0.45, tilt: -0.35, pupil: 1.15, brow: [0.02, -0.35], mouth: "frown" }),
    props: ["tear"],
    motion: (t, o, r) => {
      o.look = new THREE.Vector2(0, -0.8);
      o.rot.x = 0.2;
      o.pos.y = -0.12 + sin(t * 0.9) * 0.03;
      o.squash = 0.97;
      o.hands.forEach((h) => { h.y = -1.45; h.x *= 0.85; });
      const k = (t * 0.4) % 1;
      r.props.tear.position.y = 0.14 - 0.22 - k * 0.7;
      r.props.tear.scale.setScalar(Math.min(1, k * 6, (1 - k) * 4));
    },
  },
  angry: {
    label: "Angry", blurb: "Glares, fists shaking, steaming.",
    pose: P({ lid: 0.45, tilt: 0.5, pupil: 0.8, brow: [-0.08, 0.5], mouth: "frown" }),
    motion: (t, o) => {
      const fit = pulse(t, 2, 0.6);
      o.pos.x = fit ? sin(t * 45) * 0.04 : 0;
      o.rot.x = 0.1;
      o.squash = fit ? 0.97 : 1;
      o.hands[0].set(-1.25, -0.6 + (fit ? sin(t * 30) * 0.08 : 0), 0.55);
      o.hands[1].set(1.25, -0.6 + (fit ? cos(t * 30) * 0.08 : 0), 0.55);
    },
  },
  talking: {
    label: "Talking", blurb: "Chatters away with hand gestures.",
    pose: P({ lid: 0.1, brow: [0.04, 0], mouth: "o" }),
    motion: (t, o) => {
      o.mouth = 0.25 + abs(sin(t * 9) * sin(t * 3.1)) * 1.1;
      o.rot.z = sin(t * 1.7) * 0.06;
      o.rot.y = sin(t * 0.9) * 0.15;
      o.pos.y = abs(sin(t * 4.5)) * 0.03;
      o.hands[0].set(-1.15, -0.85 + sin(t * 3) * 0.2, 0.75);
      o.hands[1].set(1.15, -0.95 + cos(t * 2.4) * 0.25, 0.75);
    },
  },
  sleeping: {
    label: "Sleeping", blurb: "Nods off, snores, Z's float away.",
    pose: P({ eyes: ["closed", "closed"], brow: [-0.04, 0], mouth: "o" }),
    props: ["zzz"],
    motion: (t, o, r) => {
      const breath = sin(t * 1.2);
      o.rot.x = 0.25;
      o.rot.z = 0.14;
      o.pos.y = -0.15 + breath * 0.05;
      o.squash = 1 + breath * 0.03;
      o.mouth = 0.6 + (breath + 1) * 0.35;
      o.hands.forEach((h) => (h.y = -1.5));
      r.props.zzz.children.forEach((z, i) => {
        const k = (t * 0.35 + i / 3) % 1;
        z.position.set(k * 0.9 + sin(k * 6) * 0.15, k * 1.6, 0);
        z.scale.setScalar((0.6 + k * 0.6) * Math.min(1, (1 - k) * 4, k * 8));
      });
    },
  },
  celebrate: {
    label: "Celebrate", blurb: "Big jumps, arms up, confetti stars.",
    pose: P({ eyes: ["happy", "happy"], brow: [0.12, -0.1], mouth: "grin" }),
    props: ["sparkles"],
    motion: (t, o, r) => {
      const k = abs(sin(t * 4.5));
      o.pos.y = k * 0.55;
      o.squash = k < 0.15 ? 0.9 : 1.03;
      o.rot.y = sin(t * 2.25) * 0.3;
      o.hands[0].set(-1.15, 0.9 + k * 0.6, 0.3);
      o.hands[1].set(1.15, 0.9 + k * 0.6, 0.3);
      orbit(r.props.sparkles, t * 1.6, 1.9, 0.6 + k * 0.5);
    },
  },
};

function orbit(g: THREE.Group, t: number, radius: number, y: number) {
  g.children.forEach((c, i) => {
    const a = t * 0.8 + (i / g.children.length) * Math.PI * 2;
    c.position.set(cos(a) * radius, y + sin(a * 2) * 0.35, sin(a) * radius * 0.5 + 0.3);
    c.rotation.z = t * 2 + i;
    c.scale.setScalar((0.8 + (i % 2) * 0.4) * (0.75 + abs(sin(t * 3 + i)) * 0.4));
  });
}

const out: Out = { pos: new THREE.Vector3(), rot: new THREE.Vector3(), look: null, hands: [new THREE.Vector3(), new THREE.Vector3()], mouth: 1, squash: 1 };
const L = THREE.MathUtils.lerp;

/** Drive one goblin for a frame. `pointer` is the cursor direction relative to this goblin, roughly -1…1. */
export function animate(r: Rig, id: ExpressionId, t: number, pointer: THREE.Vector2, reduce: boolean) {
  const ex = EXPRESSIONS[id] ?? EXPRESSIONS.idle;
  const st = r.state;

  // discrete changes when the expression switches
  if (st.expr !== id) {
    st.expr = id;
    r.eyes.forEach((e, i) => {
      const m = ex.pose.eyes[i];
      e.open.visible = m === "open";
      e.happy.visible = m === "happy";
      e.closed.visible = m === "closed";
    });
    (Object.keys(r.mouths) as MouthKind[]).forEach((k) => (r.mouths[k].visible = k === ex.pose.mouth));
    (Object.keys(r.props) as PropKind[]).forEach((k) => (r.props[k].visible = !!ex.props?.includes(k)));
    r.hands.forEach((h) => (h.visible = !!ex.hands));
  }

  out.pos.set(0, 0, 0);
  out.rot.set(0, 0, 0);
  out.look = null;
  out.hands[0].copy(r.handHome[0]);
  out.hands[1].copy(r.handHome[1]);
  out.mouth = 1;
  out.squash = 1;
  ex.motion(reduce ? 0 : t, out, r);

  // body
  const k = 0.14;
  const h = r.head;
  h.position.set(L(h.position.x, out.pos.x, k * 2), L(h.position.y, out.pos.y, k * 2), L(h.position.z, out.pos.z, k));
  // idle goblins turn toward the cursor; scripted ones keep their own heading
  const yaw = out.rot.y !== 0 ? out.rot.y : out.look ? 0 : pointer.x * 0.35;
  const pitch = out.look ? 0 : -pointer.y * 0.15;
  h.rotation.x = L(h.rotation.x, out.rot.x + pitch, k);
  h.rotation.y = L(h.rotation.y, yaw, k);
  h.rotation.z = L(h.rotation.z, out.rot.z, k);
  const sq = L(h.scale.y, out.squash, 0.3);
  h.scale.set(1 / Math.sqrt(sq), sq, 1 / Math.sqrt(sq));
  r.hands.forEach((hand, i) => hand.position.lerp(out.hands[i], 0.18));

  // face
  st.lid = L(st.lid, ex.pose.lid, 0.2);
  st.tilt = L(st.tilt, ex.pose.tilt, 0.2);
  st.pupil = L(st.pupil, ex.pose.pupil, 0.2);
  const bl = ex.pose.browL ?? ex.pose.brow;
  const target = [bl[0], bl[1], ex.pose.brow[0], ex.pose.brow[1]];
  st.brow = st.brow.map((v, i) => L(v, target[i], 0.2));

  const look = out.look ?? pointer;
  if (!reduce && t > st.nextBlink) { st.blinkUntil = t + 0.13; st.nextBlink = t + 2.5 + Math.random() * 3.5; }
  const blinking = t < st.blinkUntil;

  r.eyes.forEach((e) => {
    e.g.scale.y = L(e.g.scale.y, blinking && e.open.visible ? 0.1 : 1, 0.5);
    e.look.obj.position.set(e.look.base.x + look.x * e.look.range, e.look.base.y + look.y * e.look.range, e.look.base.z);
    if (e.pupil) e.pupil.scale.set(st.pupil, st.pupil, 0.5);
    if (e.lid) {
      const lid = Math.max(0, st.lid);
      e.lid.rotation.x = -1.45 * (1 - lid);
      e.lid.rotation.z = e.side * st.tilt;
      e.lid.visible = lid > 0.02;
      e.white.scale.setScalar(1);
    }
    if (!e.pupil && e.white instanceof THREE.Mesh && e.white.geometry.type === "BoxGeometry") {
      e.white.scale.y = 1 - Math.max(0, st.lid) * 0.6; // visor squint
      e.white.rotation.z = e.side * st.tilt * 0.6;
    }
    // wide eyes for surprise
    const wide = 1 + Math.max(0, -st.lid) * 1.2;
    e.open.scale.set(wide, wide, 1);
  });
  r.brows.forEach((b, i) => {
    const lift = st.brow[i * 2], slant = st.brow[i * 2 + 1];
    b.obj.position.y = b.base.y + lift;
    b.obj.rotation.z = b.side * slant;
  });
  r.mouth.scale.set(1 + (out.mouth - 1) * 0.3, L(r.mouth.scale.y, out.mouth, 0.35), 1);
}

export const EXPRESSION_IDS = Object.keys(EXPRESSIONS) as ExpressionId[];

/** Key frames for the SVG renderer: where the eyes look, head tilt, and hand placement. */
export const STILLS: Partial<Record<ExpressionId, Still>> = {
  happy: { hands: [[-1.3, -0.85], [1.3, -0.85]] },
  laughing: { tilt: 0.05, hands: [[-0.55, -1.15], [0.55, -1.15]], mouth: 1.3 },
  love: { tilt: 0.1, hands: [[-0.75, -0.55], [0.75, -0.55]] },
  wink: { tilt: -0.12, hands: [null, [1.05, -0.4]] },
  surprised: { hands: [[-1.3, -0.05], [1.3, -0.05]], mouth: 1.2 },
  thinking: { look: [0.7, 0.85], tilt: 0.12, hands: [null, [0.35, -1.0]] },
  searching: { look: [0.5, -0.15], hands: [null, [0.75, -0.6]] },
  confused: { look: [0, 0.3], tilt: 0.2, hands: [null, [1.1, 0.55]] },
  nervous: { look: [-0.8, -0.1], hands: [[-0.3, -1.15], [0.3, -1.15]] },
  sad: { look: [0, -0.8], hands: [[-0.95, -1.45], [0.95, -1.45]] },
  angry: { hands: [[-1.25, -0.6], [1.25, -0.6]] },
  talking: { tilt: 0.05, hands: [[-1.15, -0.75], [1.15, -1.05]], mouth: 1.1 },
  sleeping: { tilt: 0.14, hands: [[-1.1, -1.5], [1.1, -1.5]], mouth: 1.1 },
  celebrate: { hands: [[-1.15, 1.1], [1.15, 1.1]] },
};
