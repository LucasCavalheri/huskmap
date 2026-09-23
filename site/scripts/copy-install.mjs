// The site serves the repo's install.sh at /install.sh, so the one-liner is
// `curl -fsSL https://huskmap.lucascavalheri.com.br/install.sh | bash`. Copied at build time,
// never edited here: the script's source of truth is ../install.sh.
import { copyFileSync, chmodSync } from "node:fs";

const from = new URL("../../install.sh", import.meta.url);
const to = new URL("../public/install.sh", import.meta.url);
copyFileSync(from, to);
chmodSync(to, 0o644);
console.log("install.sh copied to public/");
