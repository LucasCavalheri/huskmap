# huskmap website

Static landing page, built with [Astro](https://astro.build). English lives at `/`,
Brazilian Portuguese at `/pt-br/`.

```bash
npm install
npm run dev      # local preview
npm test         # copy rules and screenshot checks
npm run build    # astro check, then the static site in dist/
```

- **Zero framework JS.** Every section is HTML and CSS. One inline script (under 2 KB)
  reveals sections on scroll, counts numbers up, copies the install command and types the
  filter examples. With `prefers-reduced-motion`, all motion stops.
- **One source of truth.** Glyphs and brand marks are read from `../huskmap-gui/assets`,
  colors match `huskmap-gui/src/theme.rs`, fonts are the app's (subset to woff2, OFL).
- **Copy** lives in `src/i18n/en.ts` and `src/i18n/pt-BR.ts` and follows the rules in
  `AGENTS.md` ("Copy: plain words"). `astro check` fails if a key is missing.
- **Screenshots** in `public/shots/` come from the synthetic report only, never a real home:

  ```bash
  HUSKMAP_SNAPSHOT_SCALE=2 cargo test -p huskmap-gui --features desktop \
    --test snapshots site_frames -- --ignored
  ```

  then convert `target/tmp/snapshots/site-*.png` to 1480 and 2960 px WebP.
- **SEO:** canonical and hreflang links, Open Graph and Twitter cards (`public/og.png`),
  `SoftwareApplication` and `FAQPage` JSON-LD, `sitemap.xml` and `robots.txt`.
  The domain is set in `astro.config.mjs` (`site`).
