// Downloads are resolved at build time from the latest GitHub release. Never invent asset
// URLs: a family without a matching file shows as unavailable, and no release at all
// (before v0.1.0) shows the whole board as "coming soon".

export const REPO = "LucasCavalheri/huskmap";
export const REPO_URL = `https://github.com/${REPO}`;
const API = `https://api.github.com/repos/${REPO}/releases/latest`;

/** Package families, in the order the page shows them. */
export const FAMILIES = [
  { id: "deb", ext: ".deb", marks: ["debian", "ubuntu", "linuxmint"] },
  { id: "rpm", ext: ".rpm", marks: ["fedora", "opensuse", "redhat"] },
  { id: "pacman", ext: ".pkg.tar.zst", marks: ["archlinux", "manjaro"] },
  { id: "apk", ext: ".apk", marks: ["alpinelinux"] },
  { id: "tar", ext: ".tar.gz", marks: ["gentoo", "voidlinux", "nixos"] },
];

export const ARCHES = ["x86_64", "aarch64"];

/** Does `name` belong to this family and chip? Mirrors packaging/build-linux-packages.sh. */
export function matches(name, family, arch) {
  const x64 = arch === "x86_64";
  switch (family) {
    case "deb":
      return name.endsWith(x64 ? "_amd64.deb" : "_arm64.deb");
    case "rpm":
      return name.endsWith(`.${arch}.rpm`);
    case "pacman":
      return name.endsWith(`-${arch}.pkg.tar.zst`);
    case "apk":
      return name.endsWith(`-${arch}.apk`);
    case "tar":
      return name === `huskmap-linux-${x64 ? "x64" : "arm64"}.tar.gz`;
    default:
      return false;
  }
}

/** { version, page, sums, files: { [arch]: { [family]: { name, url, size } | null } } } */
export function board(release) {
  const assets = release.assets ?? [];
  const files = {};
  for (const arch of ARCHES) {
    files[arch] = {};
    for (const f of FAMILIES) {
      const a = assets.find((x) => matches(x.name, f.id, arch));
      files[arch][f.id] = a ? { name: a.name, url: a.browser_download_url, size: a.size ?? 0 } : null;
    }
  }
  const sums = assets.find((x) => x.name === "SHA256SUMS");
  return {
    version: String(release.tag_name).replace(/^v/, ""),
    page: release.html_url,
    sums: sums ? sums.browser_download_url : null,
    files,
  };
}

export function validate(release) {
  if (!release?.tag_name || !Array.isArray(release.assets) || !String(release.html_url).startsWith(`${REPO_URL}/releases/`)) {
    throw new Error("unexpected GitHub release response");
  }
  for (const a of release.assets) {
    if (!String(a.browser_download_url).startsWith(`${REPO_URL}/releases/download/`)) {
      throw new Error(`unexpected asset URL: ${a.browser_download_url}`);
    }
  }
  return release;
}

/** The latest release, or `null` when none is published yet (HTTP 404). */
export async function fetchLatest(fetchImpl = fetch, token = undefined) {
  const headers = { Accept: "application/vnd.github+json" };
  if (token) headers.Authorization = `Bearer ${token}`;
  const res = await fetchImpl(API, { headers, signal: AbortSignal.timeout(15000) });
  if (res.status === 404) return null;
  if (!res.ok) throw new Error(`could not read the latest release: GitHub HTTP ${res.status}`);
  return validate(await res.json());
}

export function formatSize(bytes, locale) {
  if (!bytes) return "";
  const mb = bytes / (1024 * 1024);
  return `${mb.toLocaleString(locale, { maximumFractionDigits: 1, minimumFractionDigits: 1 })} MB`;
}

/**
 * What the build shows. `HUSKMAP_SITE_RELEASE_JSON=path` previews a saved release response,
 * `HUSKMAP_SITE_OFFLINE=1` skips GitHub (the board shows "coming soon").
 */
export async function resolveRelease(env, fetchImpl = fetch) {
  if (env.HUSKMAP_SITE_RELEASE_JSON) {
    const { readFile } = await import("node:fs/promises");
    return validate(JSON.parse(await readFile(env.HUSKMAP_SITE_RELEASE_JSON, "utf8")));
  }
  if (env.HUSKMAP_SITE_OFFLINE === "1") return null;
  return fetchLatest(fetchImpl, env.GITHUB_TOKEN);
}
