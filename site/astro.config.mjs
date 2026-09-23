import { defineConfig } from "astro/config";
import { fileURLToPath } from "node:url";

export default defineConfig({
  // Change here if the landing page moves to another domain.
  site: "https://huskmap.lucascavalheri.com.br",
  output: "static",
  i18n: {
    defaultLocale: "en",
    locales: ["en", "pt-BR"],
    routing: { prefixDefaultLocale: false },
  },
  build: {
    format: "directory",
    assets: "assets",
    // One request for the page: CSS ships inside the HTML.
    inlineStylesheets: "always",
  },
  vite: {
    // Icons and brand marks are read straight from the app, one source of truth.
    server: { fs: { allow: [fileURLToPath(new URL("..", import.meta.url))] } },
  },
});
