// Vendored from github.com/404khai/goblins (9e63cf1, MIT, Copyright (c) 2026 Khai; see web/src/goblin/LICENSE).
import { EXPRESSION_IDS } from "./expressions";
import { goblinSvg } from "./svg";
import type { Goblin } from "./types";

export const SCHEMA_VERSION = 1;

export function toJSON(g: Goblin | Goblin[]) {
  return JSON.stringify({ $schema: "goblins/v1", version: SCHEMA_VERSION, goblins: Array.isArray(g) ? g : [g] }, null, 2);
}

/** Accepts a goblins/v1 file, a bare goblin object, or an array of goblins. */
export function fromJSON(text: string): Goblin[] {
  const v = JSON.parse(text);
  const list = Array.isArray(v) ? v : Array.isArray(v?.goblins) ? v.goblins : [v];
  if (!list.length || !list.every((g: Goblin) => g && typeof g.skin === "string" && typeof g.hat === "string"))
    throw new Error("That file has no goblins in it. Export one from the studio and try again.");
  return list.map((g: Goblin, i: number) => ({ ...g, extras: g.extras ?? [], expression: g.expression ?? "idle", id: `g${Date.now()}${i}` }));
}

export const toSVG = (g: Goblin, size = 512) => goblinSvg(g, { size, idPrefix: slug(g.name) });

const slug = (s: string) => s.toLowerCase().replace(/[^a-z0-9]+/g, "-").replace(/^-|-$/g, "") || "goblin";
const pascal = (s: string) => s.replace(/(^|[^a-zA-Z0-9]+)([a-zA-Z0-9])/g, (_, __, c: string) => c.toUpperCase()).replace(/^[^a-zA-Z]+/, "") || "Goblin";

/** A dependency-free React component with every expression baked in as SVG. */
export function toReact(g: Goblin) {
  const name = pascal(g.name);
  const frames = EXPRESSION_IDS.map((id) => `  ${id}: ${JSON.stringify(goblinSvg(g, { expression: id, idPrefix: `${slug(g.name)}-${id}` }).replace(/<svg /, '<svg width="100%" height="100%" '))},`).join("\n");
  return `// ${g.name}, exported from Goblins. Drop this file into any React project.
import type { CSSProperties } from "react";

const FRAMES = {
${frames}
} as const;

export type ${name}Expression = keyof typeof FRAMES;

export function ${name}({ expression = ${JSON.stringify(g.expression)}, size = 96, className, style }: {
  expression?: ${name}Expression;
  size?: number | string;
  className?: string;
  style?: CSSProperties;
}) {
  return (
    <span
      className={className}
      style={{ display: "inline-block", width: size, height: size, lineHeight: 0, ...style }}
      dangerouslySetInnerHTML={{ __html: FRAMES[expression] }}
    />
  );
}

export const ${name}Config = ${JSON.stringify(g, null, 2)} as const;
`;
}

export async function toPNG(g: Goblin, size = 1024): Promise<Blob> {
  const svg = goblinSvg(g, { size, idPrefix: slug(g.name) });
  const url = URL.createObjectURL(new Blob([svg], { type: "image/svg+xml" }));
  try {
    const img = new Image();
    await new Promise((ok, err) => { img.onload = ok; img.onerror = err; img.src = url; });
    const c = document.createElement("canvas");
    c.width = c.height = size;
    c.getContext("2d")!.drawImage(img, 0, 0, size, size);
    return await new Promise<Blob>((ok, err) => c.toBlob((b) => (b ? ok(b) : err(new Error("PNG encode failed"))), "image/png"));
  } finally {
    URL.revokeObjectURL(url);
  }
}

export function download(name: string, data: string | Blob, type = "text/plain") {
  const blob = typeof data === "string" ? new Blob([data], { type }) : data;
  const a = document.createElement("a");
  a.href = URL.createObjectURL(blob);
  a.download = name;
  a.click();
  setTimeout(() => URL.revokeObjectURL(a.href), 1000);
}

export const fileBase = slug;
