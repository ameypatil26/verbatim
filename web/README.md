# Verbatim — Landing Page

Static single-page marketing site for Verbatim, built with **Svelte 5 + Tailwind CSS 3**.

## Develop

From the repository root:

```powershell
npm run web:dev
```

Open `http://127.0.0.1:5173`.

## Build

```powershell
npm run build:web
```

Output:

```text
web/dist/
├── index.html
├── favicon.png
└── assets/
    ├── index-*.css
    └── index-*.js
```

`vite.config.js` uses `base: './'`, so the same build works at a domain root or a GitHub Pages subpath.

## Deploy

### Netlify

- Base directory: repository root
- Build command: `npm run build:web`
- Publish directory: `web/dist`

### GitHub Pages

```powershell
npm run build:web
```

Publish the contents of `web/dist/` to the Pages branch (or use an Actions workflow that uploads `web/dist` as the artifact). The relative base means no `/repo-name/` configuration is needed.

### Tauri

The Tauri desktop app has its own Svelte UI in `src/`. This landing page is independent; it is not bundled into the desktop binary.

## Design

- Dark palette matching the app: `#1a1a2e`, `#16213e`, `#0f3460`, `#e94560`, `#4ecca3`.
- No external fonts, images, analytics, or network calls. Icons and diagrams are inline SVG/HTML/CSS.
- Sections: navbar, hero, features, how it works, architecture, comparison, FAQ, footer.
- Responsive from small mobile up to 1180 px content width.
- Motion respects `prefers-reduced-motion`.

## Notes

- The Vite build uses the repo's in-process TypeScript preprocessor and `esbuild: false` so it also builds in restricted Windows environments.
- The root `npm run check` command includes the web sources.
