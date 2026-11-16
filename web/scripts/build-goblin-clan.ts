// Writes the Goblins clan (web/src/goblin, MIT) as one goblins/v1 recipe file per goblin into
// the repo's goblins/ folder, which the desktop app reads. Run: bun web/scripts/build-goblin-clan.ts
import { writeFileSync } from "fs";
import { DEFAULT_CLAN } from "../src/goblin/types";
import { toJSON } from "../src/goblin/export";
const out = new URL("../../goblins", import.meta.url).pathname;
DEFAULT_CLAN.forEach((g, i) => {
  const slug = g.name.toLowerCase().replace(/[^a-z0-9]+/g, "-");
  writeFileSync(`${out}/${String(i + 1).padStart(2, "0")}-${slug}.goblin.json`, toJSON(g) + "\n");
});
console.log(`wrote ${DEFAULT_CLAN.length} goblins`);
