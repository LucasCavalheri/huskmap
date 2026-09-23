// The site follows the app's copy rules: both languages cover the same keys, and no domain
// jargon or em dash reaches a visitor. Run with `npm test`.
import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

const load = (file) => readFileSync(new URL(`../src/i18n/${file}`, import.meta.url), "utf8");
const strings = (src) => [...src.matchAll(/"((?:[^"\\]|\\.)*)"/g)].map((m) => m[1]);
const keys = (src) => [...src.matchAll(/^\s*([a-zA-Z]+):/gm)].map((m) => m[1]);

test("both languages have the same keys, in the same order", () => {
  assert.deepEqual(keys(load("pt-BR.ts")), keys(load("en.ts")));
});

test("plain words only, no em dashes", () => {
  const jargon = [/\bhusks?\b/i, /\bgrove/i, /\bballast\b/i, /\bafterimage/i, /\bledger\b/i, /casca/i, /bosque/i, /lastro/i, /sondar/i];
  for (const file of ["en.ts", "pt-BR.ts"]) {
    for (const text of strings(load(file))) {
      if (/^(worktree|ballast|toolchain|afterimage|cache|debris)$/.test(text)) continue; // kind ids
      assert.ok(!text.includes("—"), `${file}: em dash in "${text}"`);
      for (const word of jargon) assert.ok(!word.test(text.replace(/huskmap/g, "")), `${file}: ${word} in "${text}"`);
    }
  }
});

test("every screenshot the page asks for exists in both languages", () => {
  for (const name of ["map", "list", "drawer", "confirm", "guide"]) {
    for (const tag of ["en", "pt"]) {
      for (const w of [1480, 2960]) {
        readFileSync(new URL(`../public/shots/${name}-${tag}-${w}.webp`, import.meta.url));
      }
    }
  }
});
