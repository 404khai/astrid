// Writes Astrid's built-in goblins as one astrid/mark@1 file each into the repo's goblins/ folder, which the
// desktop app reads. A goblin is a mark from the Goblins studio (web/src/studio, docs › Goblins). File names
// are kept from the old 3D clan so saved groups and sidebar picks still point at the same goblins.
// Also writes goblins/glyph.svg, the composer's goblin button, as a one-colour template image.
// Run: bun web/scripts/build-goblin-clan.ts
import { writeFileSync } from "node:fs";
import { geometry } from "../src/studio/forms";
import { FACES, PRESETS, eyePath } from "../src/studio/faces";

const FLUFFY = new Set(["Mossbeard", "Tansy"]);
const out = new URL("../../goblins/", import.meta.url).pathname;
const palette = (id: string) => PRESETS.find((p) => p.id === id)!;
const CLAN: [string, string, string, string, "arches" | "full" | "dual", "curved" | "sharp", string][] = [
  ["01-grub", "Grub", "peak", "cherry", "arches", "curved", "happy"],
  ["02-nettle", "Nettle", "lynx", "grape", "full", "sharp", "wink"],
  ["03-bramble", "Bramble", "crest", "ocean", "dual", "curved", "searching"],
  ["04-pip", "Pip", "round", "sun", "arches", "curved", "laughing"],
  ["05-mossbeard", "Mossbeard", "dome", "moss", "full", "curved", "sleeping"],
  ["06-wort", "Wort", "steps", "ink", "arches", "sharp", "serious"],
  ["07-tansy", "Tansy", "capsule", "bubblegum", "dual", "curved", "love"],
  ["08-knuckle", "Knuckle", "block", "ember", "full", "sharp", "angry"],
  ["09-fennick", "Fennick", "ticket", "mint", "arches", "curved", "surprised"],
  ["10-sorrel", "Sorrel", "keystone", "lilac", "dual", "sharp", "thinking"],
  ["11-dabble", "Dabble", "valley", "gold", "arches", "curved", "celebrate"],
  ["12-hob", "Hob", "sprocket", "slate", "full", "sharp", "idle"],
];
for (const [file, name, form, pal, shape, corners, mood] of CLAN) {
  const p = palette(pal);
  const mark = { $schema: "astrid/mark@1", name, mood, form, corners, fluffy: FLUFFY.has(name), shape, top: p.arch, bottom: p.arch, eyes: p.eyes, middle: "121212", ground: null };
  writeFileSync(`${out}${file}.goblin.json`, JSON.stringify(mark, null, 2) + "\n");
}
// The glyph: the lynx form's full plate with the eyes cut out, in black for a template image.
const g = geometry("lynx", "curved");
const eyes = eyePath("L", FACES.idle.L) + eyePath("R", FACES.idle.R);
writeFileSync(`${out}glyph.svg`, `<svg xmlns="http://www.w3.org/2000/svg" viewBox="40 60 432 380" width="64" height="64"><path fill="#000" fill-rule="evenodd" d="${g.full}${eyes}"/></svg>\n`);
console.log(`wrote ${CLAN.length} goblins and the glyph`);
