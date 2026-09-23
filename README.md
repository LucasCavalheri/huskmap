# huskmap

The grove remembers what the agents left.

huskmap is a Linux desktop + CLI map of the afterlife of agentic work: Claude Code, Codex, Cursor, OpenCode, Aider, Gemini CLI, Grok and kin. It is not a generic disk tool. It reads agent homes, session stores, package caches and git worktree topology, and it knows who is still working where.

Linux only, every Linux: any distro, glibc or musl, X11 or Wayland. Paths follow the XDG base directory spec.

## What it finds

| Kind | What | Examples |
| ---- | ---- | -------- |
| Worktrees | linked git worktrees and agent slots | `*-worktrees/*`, `proj/.claude/worktrees/*`, `~/.codex/worktrees/*/*`, `~/.cursor/worktrees/*/*` |
| Ballast | build and dependency dirs, only with a marker | `node_modules` + `package.json`, `target` + `Cargo.toml`, `.venv` + `pyvenv.cfg`, `.next`, `.turbo`, `.nuxt`, `.svelte-kit`, `.pytest_cache`, `.tox` |
| Toolchains | package-manager caches | npm, pnpm, yarn, bun, pip, uv, poetry, cargo, go build, playwright |
| Afterimages | agent session stores | `~/.claude/projects/*`, `~/.codex/sessions/Y/M/D`, `~/.grok/sessions/*` |
| Caches, Debris | agent caches, logs, prompt history | `~/.claude/debug`, `~/.codex/cache`, `*.log` |

## Worktrees are guarded

Before anything can go, huskmap asks:

- **Is someone in there?** Any process (agent, shell, editor, dev server) whose working directory is inside the worktree, read from `/proc`. Occupied worktrees never go, not even with `--force`.
- **Uncommitted changes?** Guarded; `--force` lifts it.
- **Commits that exist nowhere else?** Commits reachable from `HEAD` and from no other branch, remote or tag. A branch merged locally or pushed is fine; a worktree holding the only copy is guarded.
- **Locked by git, a primary checkout, outside the scan roots?** Guarded or untouchable.
- **Unique secrets?** A worktree `.env` that is a byte copy of the primary checkout's does not guard; one that exists only there does, absolutely.

Claude and Grok sessions are matched to their project: a session whose project directory is gone is marked orphaned; one whose agent is running there is occupied.

## Install

```
curl -fsSL https://raw.githubusercontent.com/LucasCavalheri/huskmap/main/install.sh | bash
```

The script reads `uname` and the package manager, downloads the matching release asset, checks it against the release's `SHA256SUMS`, and installs it. `--user` installs the portable build into `~/.local` without root; `--print-plan` shows what it would fetch.

If huskmap is open it says what is in flight before touching anything: a running apply is waited for and never interrupted, a scan (read-only) simply stops, and marked husks are kept. The window closes, updates, and reopens with its marks.

| File | Family |
| ---- | ------ |
| `huskmap_<ver>_amd64.deb` / `_arm64.deb` | Debian, Ubuntu, Mint, Pop!_OS, Kali, … |
| `huskmap-<ver>-1.x86_64.rpm` / `.aarch64.rpm` | Fedora, RHEL, Rocky, Alma, openSUSE, … |
| `huskmap-<ver>-1-x86_64.pkg.tar.zst` | Arch, Manjaro, EndeavourOS |
| `huskmap-<ver>-r0-x86_64.apk` | Alpine (with `gcompat`) |
| `huskmap-linux-x64.tar.gz` / `-arm64` | Gentoo, Void, NixOS, anywhere else |

Packages carry the `.desktop` entry, icons, man page and bash/zsh/fish completions.

## Updates

The desktop map asks GitHub Releases for a newer huskmap at most once a day. A chip appears in the status bar (`u`); the update modal shows the notes, then downloads, verifies the SHA256, installs the way this copy was installed (deb, rpm, pacman or apk through `pkexec`, or an in-place swap for the portable build) and restarts with your marks. Skip a version, or stop checking, from the same modal.

```
huskmap update --check     # say whether one exists
huskmap update             # install it (asks first; -y to not)
HUSKMAP_NO_UPDATE_CHECK=1  # never check
```

This is the only network huskmap does. Builds from `cargo` or a checkout are never overwritten.

## Commands

```
huskmap                  # desktop map on X11/Wayland; text scan otherwise
huskmap gui
huskmap scan [PATH]... [--json] [--min-size 500M] [--category worktree] [--all]
huskmap doctor
huskmap plan [--preset safe|agent-only|older] [--only PATH]... [--force] [-o plan.json]
huskmap apply --plan plan.json [--force]
huskmap map              # TUI of the last report
huskmap update [--check]
huskmap completions bash|zsh|fish
```

Default is scan + plan. Nothing moves until you apply. Apply re-reads size, mtime, git state and processes first; anything that drifted is kept. Husks go to the freedesktop trash; worktrees are then pruned from git. Only one apply runs at a time (`$XDG_STATE_HOME/huskmap/apply.lock`), and installers wait for it.

Guarded worktrees (dirty, stranded, locked) can be force-marked from the drawer (`X`) or with `plan --only PATH --force`. Occupied worktrees, primary checkouts and unique secrets never go.

## Language

English and Brazilian Portuguese. Order of precedence:

1. `--lang en|pt-br` or `HUSKMAP_LANG`
2. the EN | PT switch in the desktop map, remembered in `$XDG_CONFIG_HOME/huskmap/settings.json`
3. where the machine is: a Brazilian timezone (`TZ`, `/etc/localtime`, `/etc/timezone`) or a `*_BR` locale reads pt-BR; everywhere else, English

No network lookup for this; the machine's own settings decide.

## Desktop

A native Freya (Skia) window. The first launch opens **How it works** (`?` any time): what huskmap is for, the four steps, how to read the map, the six types, what protects an item, every filter and every key.

Keyboard first: `j/k` move, `enter` open, `x` mark, `X` mark anyway, `a` move to trash, `/` search and filter, `tab` map/list, `1`-`6` one type, `s` scan, `u` update, `?` guide. Right-click a dot on the map to mark it; click an agent in the rail to filter by it. The drawer opens folders (`xdg-open`) and copies paths.

The list has filter chips (type, status, agent, size, unchanged for), sortable columns and **Mark N removable**, which marks everything visible that may go. The chips write into the search box, and you can type the same filters by hand; they apply to the map too:

```
kind:deps size:>500mb age:>30d          # big, old dependency folders
is:uncommitted,unpushed                  # worktrees holding work
agent:claude -is:blocked "fix/"          # Claude's, not blocked, branch or path with fix/
tipo:sessoes status:orfao                # the same keys in Portuguese
```

Keys: `kind`/`tipo`, `is`/`status`, `agent`/`agente`, `size`/`peso`, `age`/`idade`, `branch`, `path`/`caminho`, `eco`. A leading `-` excludes; commas mean "any of".

Marks are remembered in `$XDG_STATE_HOME/huskmap/session.json` and come back on the next launch, re-checked against the fresh scan.

```
cargo run -p huskmap-cli            # opens the map when a display exists
cargo run -p huskmap-cli -- gui
```

Freya/Skia links against `libfreetype`, `fontconfig`, `EGL` and `GL`. If the distro has no unversioned `.so` stubs, keep them in `./link-libs` (gitignored); `.cargo/config.toml` points the linker there.

## Build

```
cargo test --workspace --all-features
bash scripts/install.test.sh               # installer flow, in a throwaway HOME
bash scripts/package.test.sh target/release/huskmap amd64
bash scripts/release-tools.test.sh
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo cov                            # coverage on huskmap-core (cargo-llvm-cov)
```

State lives in `$XDG_STATE_HOME/huskmap/`. Core owns the rules; the CLI and the map are views.

## Release

1. Bump `version` in `Cargo.toml` and add a `## <version>` section to `CHANGELOG.md`.
2. Tag `v<version>` and push. `.github/workflows/release.yml` checks the three agree, builds x86_64 and aarch64, packages every family, writes `SHA256SUMS`, attests provenance and publishes with the changelog section as notes.
