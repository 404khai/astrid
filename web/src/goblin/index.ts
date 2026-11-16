// Vendored from github.com/404khai/goblins (9e63cf1, MIT, Copyright (c) 2026 Khai; see web/src/goblin/LICENSE).
export * from "./types";
export { buildGoblin, disposeRig, type Rig } from "./build";
export { animate, EXPRESSIONS, EXPRESSION_IDS } from "./expressions";
export { Morpher, MORPHS, MORPH_IDS, type MorphId } from "./morph";
export { goblinSvg, type SvgOptions } from "./svg";
export { toJSON, fromJSON, toSVG, toReact, toPNG } from "./export";
export { GoblinAvatar, Goblin3D } from "./react";
