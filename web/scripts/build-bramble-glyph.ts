// Writes the composer's goblin icon (goblins/bramble-glyph.svg): beardless
// Bramble as a one-colour outline glyph from the goblins library, used as a
// template image by the desktop app. Run: bun web/scripts/build-bramble-glyph.ts
import { writeFileSync } from "node:fs";
import { fromJSON, goblinGlyphSvg } from "@404khai/goblins/flat";

const bramble = fromJSON(JSON.stringify({
  $schema: "goblins/v1",
  version: 1,
  goblins: [{ id: "bramble", name: "Bramble", skin: "#4E9C8E", head: "wide", ears: "long", nose: "bulb", eyes: "beady", hat: { style: "gnome", color: "#2F4AA8" }, beard: "none", eyewear: "none", scarf: "none", extras: [] }],
}))[0];
const svg = goblinGlyphSvg(bramble, { color: "#000", size: 64, idPrefix: "bramble" });
writeFileSync(new URL("../../goblins/bramble-glyph.svg", import.meta.url), svg + "\n");
console.log(svg.length, "bytes");
