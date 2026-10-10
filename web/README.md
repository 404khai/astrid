# Astrid homepage

Responsive React/TypeScript homepage based on Paper’s **Astrid / Homepage / Desktop** in the [Astrid design file](https://app.paper.design/file/01M44J87RWEKW0TNRSDQAYZXM5/p-1-0). Layout and typography values came from Paper’s JSX export. Product copy reflects the personal runtime direction and verified Phase 4 foundations in AGENTS.md; the terminal run is explicitly illustrative.

```sh
cd web
pnpm install --frozen-lockfile
pnpm dev
pnpm build
pnpm test
```

Preview: http://127.0.0.1:5173. Tests use locally installed Google Chrome. The site is a standalone client-side marketing page; it adds no runtime capabilities or desktop dependencies. External project links open GitHub documentation.

## Components and sources

- `AstridLogo`: original `references/mobile/svgs/logo.svg`, with isolated eye motion. Nine-second cycle: forward → up → left → right → forward → blink. Reduced-motion preferences disable the animation. Gradient IDs are unique per instance.
- `LaptopMockupCard`: [Nexvyn laptop mockup](https://ui.nexvyn.dev/components/laptop-mockup), adapted from its [component registry](https://ui.nexvyn.dev/r/laptop-mockup.json) to scoped CSS and the Paper layout’s larger scale. Keeps its screen, bezel, chassis, base, notch, forwarded ref, and variant API. Nexvyn permits personal and commercial component use; the blueprint illustration is not included.
- Icons use the actual upstream `RuneIcon` component and lookup helpers, vendored because `runeicons-react` is private and absent from npm. See `vendor/runeicons-react/README.md` for the pinned source and subset.
- Space Grotesk and IBM Plex Mono are self-hosted through Fontsource.

The More dropdown supports Tab navigation, Escape dismissal with focus restoration, and outside-click dismissal. No tracking, hosted forms, or remote font requests are added.
