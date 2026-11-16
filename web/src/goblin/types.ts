// Vendored from github.com/404khai/goblins (9e63cf1, MIT, Copyright (c) 2026 Khai; see web/src/goblin/LICENSE).
export type ExpressionId =
  | "idle" | "happy" | "laughing" | "love" | "wink" | "surprised" | "thinking" | "searching"
  | "confused" | "nervous" | "sad" | "angry" | "talking" | "sleeping" | "celebrate";

import type { MorphId } from "./morph";

export interface Goblin {
  id: string;
  name: string;
  expression: ExpressionId;
  skin: string;
  head: "round" | "egg" | "wide" | "bean";
  ears: "pointy" | "round" | "droopy" | "long" | "tiny";
  nose: "bulb" | "button" | "long" | "hook";
  eyes: "round" | "oval" | "wonky" | "beady" | "visor";
  hat: "gnome" | "wizard" | "beret" | "beanie" | "mushroom" | "sprout" | "headband" | "none";
  hatColor: string;
  beard: "full" | "pointy" | "forked" | "braid" | "none";
  beardColor: string;
  eyewear: "none" | "round" | "shades" | "monocle";
  scarf: "none" | "wrap" | "long";
  scarfColor: string;
  extras: string[];
  /** Optional spell: the goblin transforms into what it is doing. 3D only. */
  morph?: MorphId | null;
}

type Trait = Exclude<keyof Goblin, "id" | "name" | "expression" | "morph">;
export interface Option { label: string; values: string[]; color?: boolean; multi?: boolean; dependsOn?: [Trait, string] }

export const OPTIONS: Record<Trait, Option> = {
  skin: { label: "Skin", color: true, values: ["#6FA35A", "#9AAE4C", "#4E9C8E", "#4A7FD6", "#9C86C9", "#E2508C", "#E8613C", "#F2B630", "#C07A55", "#7C8A99"] },
  head: { label: "Head", values: ["round", "egg", "wide", "bean"] },
  ears: { label: "Ears", values: ["pointy", "round", "droopy", "long", "tiny"] },
  nose: { label: "Nose", values: ["bulb", "button", "long", "hook"] },
  eyes: { label: "Eyes", values: ["round", "oval", "wonky", "beady", "visor"] },
  hat: { label: "Hat", values: ["gnome", "wizard", "beret", "beanie", "mushroom", "sprout", "headband", "none"] },
  hatColor: { label: "Hat color", color: true, dependsOn: ["hat", "none"], values: ["#D8402F", "#2F4AA8", "#E0A92B", "#2E6B46", "#25221F", "#E35D9C", "#5B3FA0", "#ECE6D6"] },
  beard: { label: "Beard", values: ["full", "pointy", "forked", "braid", "none"] },
  beardColor: { label: "Beard color", color: true, dependsOn: ["beard", "none"], values: ["#EDE8DF", "#C9652B", "#9A948C", "#5B3B28", "#2A2420", "#E9C46A"] },
  eyewear: { label: "Eyewear", values: ["none", "round", "shades", "monocle"] },
  scarf: { label: "Scarf", values: ["none", "wrap", "long"] },
  scarfColor: { label: "Scarf color", color: true, dependsOn: ["scarf", "none"], values: ["#E0A92B", "#2F4AA8", "#D8402F", "#ECE6D6", "#2E6B46", "#E35D9C"] },
  extras: { label: "Extras", multi: true, values: ["headphones", "earring", "pipe", "tusks", "freckles"] },
};

export const NAMES = ["Grub", "Nettle", "Bramble", "Pip", "Mossbeard", "Wort", "Snig", "Tansy", "Burdock", "Fennick", "Gristle", "Hob", "Sorrel", "Knuckle", "Toadflax", "Mugwump", "Dabble", "Crumb"];

const pick = <T,>(a: readonly T[]) => a[Math.floor(Math.random() * a.length)];

export function randomGoblin(base: Pick<Goblin, "id" | "name" | "expression">): Goblin {
  const g = { ...base, extras: OPTIONS.extras.values.filter(() => Math.random() < 0.25) } as Goblin;
  for (const k of Object.keys(OPTIONS) as Trait[]) if (!OPTIONS[k].multi) (g as any)[k] = pick(OPTIONS[k].values);
  return g;
}

const G = (id: number, name: string, expression: ExpressionId, t: Omit<Goblin, "id" | "name" | "expression">): Goblin => ({ id: `g${id}`, name, expression, ...t });

export const DEFAULT_CLAN: Goblin[] = [
  G(1, "Grub", "idle", { skin: "#6FA35A", head: "round", ears: "pointy", nose: "bulb", eyes: "round", hat: "gnome", hatColor: "#D8402F", beard: "full", beardColor: "#EDE8DF", eyewear: "round", scarf: "none", scarfColor: "#E0A92B", extras: [] }),
  G(2, "Nettle", "thinking", { skin: "#9C86C9", head: "egg", ears: "round", nose: "long", eyes: "oval", hat: "beret", hatColor: "#25221F", beard: "pointy", beardColor: "#2A2420", eyewear: "none", scarf: "wrap", scarfColor: "#D8402F", extras: ["pipe"] }),
  G(3, "Bramble", "searching", { skin: "#4E9C8E", head: "wide", ears: "droopy", nose: "bulb", eyes: "visor", hat: "gnome", hatColor: "#2F4AA8", beard: "braid", beardColor: "#9A948C", eyewear: "none", scarf: "none", scarfColor: "#E0A92B", extras: ["headphones"] }),
  G(4, "Pip", "happy", { skin: "#F2B630", head: "bean", ears: "tiny", nose: "button", eyes: "round", hat: "sprout", hatColor: "#E0A92B", beard: "none", beardColor: "#C9652B", eyewear: "none", scarf: "long", scarfColor: "#2F4AA8", extras: ["freckles"] }),
  G(5, "Mossbeard", "sleeping", { skin: "#9AAE4C", head: "round", ears: "long", nose: "hook", eyes: "round", hat: "wizard", hatColor: "#5B3FA0", beard: "full", beardColor: "#9A948C", eyewear: "monocle", scarf: "none", scarfColor: "#E0A92B", extras: [] }),
  G(6, "Wort", "angry", { skin: "#7C8A99", head: "wide", ears: "pointy", nose: "hook", eyes: "beady", hat: "headband", hatColor: "#D8402F", beard: "none", beardColor: "#EDE8DF", eyewear: "none", scarf: "none", scarfColor: "#ECE6D6", extras: ["tusks", "earring"] }),
  G(7, "Tansy", "love", { skin: "#E2508C", head: "egg", ears: "round", nose: "button", eyes: "oval", hat: "mushroom", hatColor: "#D8402F", beard: "none", beardColor: "#EDE8DF", eyewear: "none", scarf: "wrap", scarfColor: "#ECE6D6", extras: ["freckles"] }),
  G(8, "Knuckle", "laughing", { skin: "#C07A55", head: "round", ears: "droopy", nose: "bulb", eyes: "wonky", hat: "beanie", hatColor: "#2E6B46", beard: "forked", beardColor: "#C9652B", eyewear: "none", scarf: "long", scarfColor: "#E0A92B", extras: ["earring"] }),
  G(9, "Fennick", "confused", { skin: "#4A7FD6", head: "bean", ears: "long", nose: "long", eyes: "round", hat: "none", hatColor: "#D8402F", beard: "pointy", beardColor: "#EDE8DF", eyewear: "round", scarf: "none", scarfColor: "#E0A92B", extras: [] }),
  G(10, "Sorrel", "sad", { skin: "#6FA35A", head: "egg", ears: "droopy", nose: "button", eyes: "oval", hat: "beanie", hatColor: "#2F4AA8", beard: "none", beardColor: "#9A948C", eyewear: "none", scarf: "long", scarfColor: "#D8402F", extras: ["headphones"] }),
  G(11, "Dabble", "surprised", { skin: "#E8613C", head: "round", ears: "round", nose: "bulb", eyes: "round", hat: "headband", hatColor: "#2F4AA8", beard: "none", beardColor: "#5B3B28", eyewear: "none", scarf: "none", scarfColor: "#E0A92B", extras: ["tusks"] }),
  G(12, "Hob", "celebrate", { skin: "#9AAE4C", head: "wide", ears: "pointy", nose: "bulb", eyes: "wonky", hat: "gnome", hatColor: "#E0A92B", beard: "full", beardColor: "#5B3B28", eyewear: "shades", scarf: "wrap", scarfColor: "#2F4AA8", extras: [] }),
];
