// Builds Astrid's live canvas (web/canvas.html, ADR 0036) into one HTML file
// for the phone app's WebView (ADR 0039): web/vite.phone.config.ts makes one
// script and one stylesheet with fonts inlined, and this folds both into the
// page. Output: modules/astrid-overlay/android/src/main/assets/astrid-canvas.html.
import { execFileSync } from "node:child_process";
import { mkdirSync, readFileSync, readdirSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const web = join(here, "../../web");
const out = join(here, "../modules/astrid-overlay/android/src/main/assets/astrid-canvas.html");

execFileSync("npx", ["vite", "build", "-c", "vite.phone.config.ts", "--logLevel", "warn"], { cwd: web, stdio: "inherit" });

const dist = join(web, "dist-phone");
let html = readFileSync(join(dist, "canvas.html"), "utf8");
const assets = readdirSync(join(dist, "assets"));
const script = assets.filter((f) => f.endsWith(".js"));
const style = assets.filter((f) => f.endsWith(".css"));
if (script.length !== 1 || style.length > 1) throw new Error(`expected one script and at most one stylesheet, got ${assets.join(", ")}`);

// Function replacers: the bundle contains `$&`-like sequences.
html = html.replace(/<script type="module"[^>]*src="[^"]+"[^>]*><\/script>/, () => {
  const code = readFileSync(join(dist, "assets", script[0]), "utf8").replace(/<\/script/gi, "<\\/script");
  return `<script type="module">${code}</script>`;
});
html = html.replace(/<link rel="stylesheet"[^>]*href="[^"]+"[^>]*>/, () =>
  style.length ? `<style>${readFileSync(join(dist, "assets", style[0]), "utf8")}</style>` : "",
);
html = html.replace(/<link rel="icon"[^>]*>/, "");
if (/(src|href)="\.\/assets\//.test(html)) throw new Error("an asset was left out of the page");

mkdirSync(dirname(out), { recursive: true });
writeFileSync(out, html);
console.log(`canvas: ${(html.length / 1024).toFixed(0)} KiB -> ${out}`);
