# huskmap website

Static landing page, built with [Astro](https://astro.build). English lives at `/`,
Brazilian Portuguese at `/pt-br/`.

```bash
npm install
npm run dev      # local preview
npm test         # copy rules and screenshot checks
npm run build    # astro check, then the static site in dist/
```

- **Minimal and delicate.** Paper, ink and one accent; the app screenshots carry the color.
  Hero, a light/dark comparison of the app, three points, the agents, install. Nothing else.
- **Zero framework JS.** HTML and CSS. One small inline script reveals sections on scroll,
  drives the light/dark slider and copies the install command. With
  `prefers-reduced-motion`, all motion stops.
- **One source of truth.** Glyphs and brand marks are read from `../huskmap-gui/assets`,
  colors follow the app's Daylight palette, fonts are the app's (subset to woff2, OFL).
- **Copy** lives in `src/i18n/en.ts` and `src/i18n/pt-BR.ts` and follows the rules in
  `AGENTS.md` ("Copy: plain words"). `astro check` fails if a key is missing.
- **Screenshots** in `public/shots/` come from the synthetic report only, never a real home:

  ```bash
  HUSKMAP_SNAPSHOT_SCALE=2 cargo test -p huskmap-gui --features desktop \
    --test snapshots site_frames -- --ignored
  ```

  then convert `target/tmp/snapshots/site-map-{en,pt}-{dark,light}.png` to 1480 and
  2960 px WebP in `public/shots/`.
- **SEO:** canonical and hreflang links, Open Graph and Twitter cards (`public/og.png`),
  `SoftwareApplication` JSON-LD, `sitemap.xml` and `robots.txt`. Lighthouse: 100 on
  performance, accessibility, best practices and SEO.
  The domain is set in `astro.config.mjs` (`site`).
