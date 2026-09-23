import { test } from "node:test";
import assert from "node:assert/strict";
import { ARCHES, FAMILIES, REPO_URL, board, fetchLatest, formatSize, matches, validate } from "../src/data/release.mjs";

const dl = (name) => ({ name, browser_download_url: `${REPO_URL}/releases/download/v0.2.0/${name}`, size: 3_145_728 });
const release = {
  tag_name: "v0.2.0",
  html_url: `${REPO_URL}/releases/tag/v0.2.0`,
  assets: [
    "huskmap_0.2.0_amd64.deb",
    "huskmap_0.2.0_arm64.deb",
    "huskmap-0.2.0-1.x86_64.rpm",
    "huskmap-0.2.0-1-x86_64.pkg.tar.zst",
    "huskmap-0.2.0-1-aarch64.pkg.tar.zst",
    "huskmap-0.2.0-r0-x86_64.apk",
    "huskmap-linux-x64.tar.gz",
    "huskmap-linux-arm64.tar.gz",
    "SHA256SUMS",
  ].map(dl),
};

test("every family and chip finds its own file, and only its own", () => {
  const b = board(release);
  assert.equal(b.version, "0.2.0");
  assert.equal(b.files.x86_64.deb.name, "huskmap_0.2.0_amd64.deb");
  assert.equal(b.files.aarch64.deb.name, "huskmap_0.2.0_arm64.deb");
  assert.equal(b.files.x86_64.rpm.name, "huskmap-0.2.0-1.x86_64.rpm");
  assert.equal(b.files.aarch64.rpm, null, "a missing build is unavailable, not guessed");
  assert.equal(b.files.aarch64.pacman.name, "huskmap-0.2.0-1-aarch64.pkg.tar.zst");
  assert.equal(b.files.x86_64.apk.name, "huskmap-0.2.0-r0-x86_64.apk");
  assert.equal(b.files.aarch64.tar.name, "huskmap-linux-arm64.tar.gz");
  assert.ok(b.sums.endsWith("/SHA256SUMS"));
  assert.ok(b.files.x86_64.deb.url.startsWith(`${REPO_URL}/releases/download/`));
  assert.equal(FAMILIES.length, 5);
  assert.deepEqual(ARCHES, ["x86_64", "aarch64"]);
  assert.ok(!matches("huskmap_0.2.0_amd64.deb", "deb", "aarch64"));
  assert.ok(!matches("x", "nope", "x86_64"));
});

test("no release yet means no board, other failures stop the build", async () => {
  const reply = (status, body = {}) => async () => ({ status, ok: status === 200, json: async () => body });
  assert.equal(await fetchLatest(reply(404)), null);
  await assert.rejects(fetchLatest(reply(500)), /HTTP 500/);
  assert.equal((await fetchLatest(reply(200, release))).tag_name, "v0.2.0");
  let auth;
  await fetchLatest(async (_url, opts) => {
    auth = opts.headers.Authorization;
    return { status: 404, ok: false };
  }, "t0k");
  assert.equal(auth, "Bearer t0k");
});

test("foreign URLs are refused", () => {
  assert.throws(() => validate({ ...release, html_url: "https://evil.example/x" }));
  assert.throws(() => validate({ ...release, assets: [{ name: "a", browser_download_url: "https://evil.example/a" }] }));
  assert.throws(() => validate(null));
});

test("sizes read naturally in both languages", () => {
  assert.equal(formatSize(3_145_728, "en"), "3.0 MB");
  assert.equal(formatSize(3_145_728, "pt-BR"), "3,0 MB");
  assert.equal(formatSize(0, "en"), "");
});

test("the build can preview a saved release or skip GitHub", async () => {
  const { writeFile, mkdtemp } = await import("node:fs/promises");
  const { join } = await import("node:path");
  const { tmpdir } = await import("node:os");
  const { resolveRelease } = await import("../src/data/release.mjs");
  const dir = await mkdtemp(join(tmpdir(), "huskmap-site-"));
  const file = join(dir, "release.json");
  await writeFile(file, JSON.stringify(release));
  assert.equal((await resolveRelease({ HUSKMAP_SITE_RELEASE_JSON: file })).tag_name, "v0.2.0");
  assert.equal(await resolveRelease({ HUSKMAP_SITE_OFFLINE: "1" }), null);
  let asked = false;
  await resolveRelease({}, async () => {
    asked = true;
    return { status: 404, ok: false };
  });
  assert.ok(asked, "by default it asks GitHub");
});
