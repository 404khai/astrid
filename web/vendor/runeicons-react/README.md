# RuneIcons React source

Source: https://github.com/Runeicons/runeicons/tree/210c95e68213ff6cd521ea7364b96c8bdff26d40/packages/runeicons-react

Apache-2.0; see LICENSE. `src/index.tsx` and `src/icons.ts` are unchanged upstream sources. The package is connected through pnpm’s local `file:` dependency because the upstream package is private and is not currently published to npm.

`src/icons.generated.ts` contains only the three normal icons used by this homepage: `arrows-arrow-right`, `arrows-arrow-up-right`, and `arrows-chevron-down`. It was generated from the upstream `public/normal/arrows` SVGs at the same commit, preserving root paint attributes and converting black strokes/fills to currentColor as the upstream build script does. Other names/variants are not bundled. Expand the generated subset from the pinned upstream assets when adding icons.
