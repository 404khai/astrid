// Vendored from github.com/404khai/goblins (9e63cf1, MIT, Copyright (c) 2026 Khai; see web/src/goblin/LICENSE).
import { useEffect, useMemo, useRef, type CSSProperties } from "react";
import * as THREE from "three";
import { buildGoblin, disposeRig } from "./build";
import { animate } from "./expressions";
import { Morpher, type MorphId } from "./morph";
import { goblinSvg, type SvgOptions } from "./svg";
import type { ExpressionId, Goblin } from "./types";

type Common = { goblin: Goblin; expression?: ExpressionId; size?: number | string; className?: string; style?: CSSProperties };

/** Flat SVG goblin. No WebGL, safe for lists, server rendering and avatars. */
export function GoblinAvatar({ goblin, expression, size = 96, className, style, ...svg }: Common & Omit<SvgOptions, "size" | "expression">) {
  const markup = useMemo(() => goblinSvg(goblin, { expression, ...svg }), [goblin, expression, svg.hands, svg.props, svg.background, svg.idPrefix]);
  return <span className={className} style={{ display: "inline-block", width: size, height: size, lineHeight: 0, ...style }} dangerouslySetInnerHTML={{ __html: markup }} />;
}

/** Live 3D goblin with the animated expression. One WebGL canvas per instance. */
export function Goblin3D({ goblin, expression, morph, size = 240, className, style, framing = "full" }: Common & { framing?: "full" | "head"; morph?: MorphId | null }) {
  const ref = useRef<HTMLCanvasElement>(null);
  const expr = useRef(expression ?? goblin.expression);
  expr.current = expression ?? goblin.expression;
  const spell = useRef<MorphId | null>(null);
  spell.current = morph !== undefined ? morph : goblin.morph ?? null;
  const key = JSON.stringify({ ...goblin, expression: undefined, name: undefined, morph: undefined });

  useEffect(() => {
    const canvas = ref.current!;
    const renderer = new THREE.WebGLRenderer({ canvas, antialias: true, alpha: true });
    renderer.setPixelRatio(Math.min(devicePixelRatio, 2));
    const scene = new THREE.Scene();
    scene.add(new THREE.AmbientLight(0xffffff, 1.7));
    const sun = new THREE.DirectionalLight(0xffffff, 2.4);
    sun.position.set(4, 6, 8);
    scene.add(sun);
    const camera = new THREE.PerspectiveCamera(30, 1, 0.1, 100);
    const head = framing === "head";
    camera.position.set(0, head ? 0.6 : 0.4, head ? 10.5 : 13);
    camera.lookAt(0, head ? 0.25 : 0.2, 0);
    const rig = buildGoblin(goblin);
    rig.ring.visible = false;
    scene.add(rig.root);
    const pointer = new THREE.Vector2();
    const move = (e: PointerEvent) => {
      const r = canvas.getBoundingClientRect();
      pointer.set(THREE.MathUtils.clamp((e.clientX - r.left) / r.width * 2 - 1, -1, 1), THREE.MathUtils.clamp(-((e.clientY - r.top) / r.height * 2 - 1), -1, 1));
    };
    window.addEventListener("pointermove", move);
    const ro = new ResizeObserver(() => {
      renderer.setSize(canvas.clientWidth, canvas.clientHeight, false);
      camera.aspect = canvas.clientWidth / canvas.clientHeight || 1;
      camera.updateProjectionMatrix();
    });
    ro.observe(canvas);
    const reduce = matchMedia("(prefers-reduced-motion: reduce)").matches;
    const clock = new THREE.Clock();
    let raf = 0;
    let morpher: Morpher | null = null;
    const tick = () => {
      const t = clock.getElapsedTime();
      animate(rig, expr.current, t, pointer, reduce);
      if (spell.current && !morpher) morpher = new Morpher(rig, goblin);
      if (morpher) { morpher.set(spell.current); morpher.update(t); }
      renderer.render(scene, camera);
      raf = requestAnimationFrame(tick);
    };
    tick();
    return () => { cancelAnimationFrame(raf); morpher?.dispose(); ro.disconnect(); window.removeEventListener("pointermove", move); disposeRig(rig); renderer.dispose(); };
  }, [key, framing]);

  return <canvas ref={ref} className={className} style={{ width: size, height: size, display: "block", ...style }} aria-label={`${goblin.name}, 3D goblin`} />;
}
