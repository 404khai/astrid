# Anatomy kit (vendored)

`iso-kit.mjs` is copied unchanged from the Anatomy skill
([404khai/anatomy](https://github.com/404khai/anatomy), fork of
[wheresryan22/anatomy](https://github.com/wheresryan22/anatomy)) at commit
`1482e846` (2026-10-18). MIT licensed; see `LICENSE`.

It is used only at build time by `scripts/build-figure.mjs`, which writes the
homepage's isometric engine to `src/generated/engine-figure.ts`. Nothing here
ships to the browser except the generated SVG strings.
