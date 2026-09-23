// Glyphs and brand marks come straight from the desktop app's assets: one set, one source.
// They ship painted white; here they take `currentColor` so CSS can tint them.
const glyphs = import.meta.glob("../../../huskmap-gui/assets/icons/*.svg", {
  query: "?raw",
  import: "default",
  eager: true,
}) as Record<string, string>;

const brands = import.meta.glob("../../../huskmap-gui/assets/brands/*.svg", {
  query: "?raw",
  import: "default",
  eager: true,
}) as Record<string, string>;

const distros = import.meta.glob("../assets/distros/*.svg", {
  query: "?raw",
  import: "default",
  eager: true,
}) as Record<string, string>;

function find(table: Record<string, string>, name: string): string {
  const hit = Object.entries(table).find(([path]) => path.endsWith(`/${name}.svg`));
  if (!hit) throw new Error(`icon not found: ${name}`);
  return hit[1];
}

function prepare(svg: string, label?: string): string {
  const cleaned = svg
    .replace(/<title>.*?<\/title>/s, "")
    .replace(/\swidth="[^"]*"/, "")
    .replace(/\sheight="[^"]*"/, "")
    .replace(/"white"/g, '"currentColor"')
    .replace(/'white'/g, "'currentColor'");
  const a11y = label ? ` role="img" aria-label="${label}"` : ` aria-hidden="true" focusable="false"`;
  return cleaned.replace(/<svg\b/, `<svg${a11y}`);
}

export const glyph = (name: string) => prepare(find(glyphs, name));
export const brand = (name: string, label: string) => prepare(find(brands, name), label);
export const distro = (name: string) => prepare(find(distros, name));

/** Agents huskmap knows, with the mark that identifies each one. */
export const AGENTS = [
  { mark: "claude", name: "Claude Code" },
  { mark: "openai", name: "Codex" },
  { mark: "cursor", name: "Cursor" },
  { mark: "grok", name: "Grok" },
  { mark: "gemini", name: "Gemini CLI" },
  { mark: "opencode", name: "OpenCode" },
  { mark: "aider", name: "Aider" },
];

/** Toolchains whose caches it weighs. */
export const TOOLS = [
  { mark: "npm", name: "npm" },
  { mark: "pnpm", name: "pnpm" },
  { mark: "yarn", name: "Yarn" },
  { mark: "bun", name: "Bun" },
  { mark: "python", name: "Python" },
  { mark: "uv", name: "uv" },
  { mark: "rust", name: "Rust" },
  { mark: "go", name: "Go" },
  { mark: "playwright", name: "Playwright" },
];
