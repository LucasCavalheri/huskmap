# AGENTS.md — huskmap

Read this file before writing any code. These rules are binding.

## What huskmap is

huskmap is a **Linux-only desktop + CLI** tool for people who live an agentic workflow: Grok, Claude Code, Codex, Cursor, OpenCode, Aider, Gemini CLI and similar.

It is **not** a generic disk cleaner. It understands agent-specific paths, session metadata, and git worktree topology.

It must show:

- Heavy git worktrees left by agents (`agent-*` dirs, `~/.codex/worktrees`, `.claude/worktrees`, Cursor/OpenCode equivalents)
- Bloated `node_modules` / `.venv` / `target` / `.next` / `.turbo` duplicated across those worktrees
- Agent sessions, conversation stores, caches, leftover artifacts
- A map of what agents did to this machine: disk weight, stale sessions, orphan worktrees, reclaimable space

Tone of the product: sharp, slightly dark, mischievous. The machine has a secret afterlife of agents. The UI must feel like a tool you `pacman -S`, not a SaaS dashboard.

Never collide with `agent-gc` in naming, flags, or UX copy. We are huskmap.

## Platform: Linux only, every Linux

huskmap targets Linux and nothing else. No macOS or Windows code paths, `cfg` arms, path tables, packages or CI jobs. Do not add them "for later".

Inside Linux, compatibility is total:

- Any distro family: Debian/Ubuntu, Fedora/RHEL, openSUSE, Arch, Alpine (musl), Gentoo, Void, NixOS. No distro-specific assumptions in core.
- glibc and musl. x86_64 and aarch64.
- X11 and Wayland for the GUI (`DISPLAY` or `WAYLAND_DISPLAY`). Headless machines get the CLI/TUI.
- Paths follow the XDG Base Directory spec: honor `XDG_CACHE_HOME`, `XDG_DATA_HOME`, `XDG_CONFIG_HOME`, `XDG_STATE_HOME`, falling back to `~/.cache`, `~/.local/share`, `~/.config`, `~/.local/state`. Never hardcode `~/.cache` where the spec applies.
- Processes are read from `/proc`. Trash is the freedesktop.org trash spec.
- Ship like the rest of the family: `.deb`, `.rpm`, `.pkg.tar.zst`, `.apk` and a portable `tar.gz`, plus a `.desktop` file (`packaging/`), built by `packaging/build-linux-packages.sh` and checked by `scripts/package.test.sh`.
- `install.sh` (curl | bash) and the in-app updater never interrupt an apply (`apply.lock`), keep marks (`session.json`), and verify `SHA256SUMS`. Their state interface is `huskmap status`; keep its `key=value` lines stable.

## Network

The only network huskmap does is the update check against GitHub Releases: at most once a day, off with `HUSKMAP_NO_UPDATE_CHECK=1` or `check_updates: false`, behind the `Fetch` trait. Nothing else leaves the machine: no telemetry, no geolocation (language comes from local settings, timezone and locale).

## Stack (do not change without an explicit human decision)

| Layer            | Choice                                                                       | Why                                                   |
| ---------------- | ---------------------------------------------------------------------------- | ----------------------------------------------------- |
| Language         | **Rust** (edition 2024), one workspace                                       | one binary family, no JS runtime in the GUI           |
| CLI              | `clap` + `ratatui`                                                           | `huskmap scan`, `huskmap doctor`, scriptable `--json` |
| Desktop GUI      | **Freya** (Skia)                                                             | native, animated, componentized, MIT, pure Rust       |
| Charts / map viz | Freya + `plotters` (Freya integration) and custom Skia draw for the husk map | the map is the product                                |
| Async / FS       | `tokio` + `walkdir` / `jwalk` + `ignore`                                     | fast scans, respect `.gitignore` only when relevant   |
| Git              | `git2` and/or invoking `git worktree` when safer                             | worktree topology is first-class                      |
| Serde            | `serde` + `serde_json`                                                       | scan reports, session metadata                        |
| Processes        | `/proc` directly                                                             | who is inside a worktree, no extra dependency         |
| Updates          | `ureq` (rustls) + `sha2`                                                     | GitHub Releases check, verified downloads             |
| Tests            | `cargo test`, `cargo llvm-cov` / `cargo tarpaulin`                           | coverage gate                                         |
| Package          | workspace: `huskmap-core`, `huskmap-cli`, `huskmap-gui`                      | core has zero UI deps                                 |

Forbidden:

- Tauri, Electron, Dioxus-WebView, any HTML/CSS/JS UI
- Slint (DSL + license)
- egui as the product UI (debug aesthetic)
- Iced / GPUI unless Freya is proven blocked — ask first
- `unsafe` except documented FFI / platform shims with tests
- `unwrap()` / `expect()` on fallible I/O, git, or parse paths in library code
- Calling this tool “cleaner”, “gc”, or “agent-gc”

## Architecture

```
huskmap-core   scan, classify, size, risk, plan, apply
huskmap-cli    clap commands + ratatui + JSON
huskmap-gui    Freya app that only talks to core
```

Core owns the domain. GUI and CLI are views. If a rule exists in the GUI, it exists in core and is testable without opening a window.

Domain objects (names are stable — use them):

- `Husk` — one reclaimable or notable artifact (worktree, session, cache, dep tree, lockfile debris)
- `Grove` — a repo + its worktree set
- `Afterimage` — a dead agent session / conversation store with no live process
- `Ballast` — duplicated heavy dirs (`node_modules`, `target`, `.venv`, `.next`, `.turbo`)
- `ScanReport` — the map: totals, groups, risk, reclaimable bytes
- `Plan` — dry-run of what would be removed / quarantined
- `ApplyResult` — what actually happened

Safety invariant (non-negotiable):

- Default is **scan + plan**. Mutation requires an explicit apply.
- Never delete a dirty worktree or a worktree with unpushed-only work without a force flag the user opted into.
- Never touch the primary checkout, paths outside scan roots, or files marked dangerous (secrets, `.env`, ssh keys).
- Prefer `git worktree remove` / unlock+prune over raw `rm`. Fall back to trash/recycle, not silent unlink, when the OS allows it.
- Re-validate facts immediately before apply. A plan is stale if mtime/size/git state drifted.

Known scan roots (extend in one table in core, not scattered `if`s):

- `~/.codex/worktrees`
- `~/.claude` and `**/.claude/worktrees`
- `~/.cursor`, `~/.opencode`, `~/.gemini`, `~/.aider`
- common project homes: `~/dev`, `~/src`, `~/code`, `~/projects`, `~/Developer`, `~/workspace`, `~/repos`, `~/git`, `~/work`, `~/Documents`, `~/Documentos`
- package-manager caches under the XDG dirs (npm, pnpm, yarn, bun, pip, uv, poetry, cargo, go, playwright)

Classifier must use markers (e.g. `target` only under `Cargo.toml`; venv only with venv markers). Do not delete a folder just because it is large.

## Coverage

Target: **100% line + branch coverage on `huskmap-core`.**

If 100% is genuinely unreachable (OS-only branches, FFI, GUI event glue):

1. Isolate the untestable edge behind a tiny trait.
2. Cover the trait with fakes in core tests.
3. Document the residual in `COVERAGE.md` with the exact missed lines and why.
4. Core still ships at ≥ 95% lines and ≥ 90% branches. Below that is a failed PR.

Rules:

- Every public core function has a unit or table-driven test.
- Scan, classify, plan, apply each have fixtures under `huskmap-core/tests/fixtures/`.
- Apply tests run against temp dirs only. Never a developer home path.
- GUI/CLI get integration tests for command parsing and for mapping `ScanReport` → view model. Widget pixels are not an excuse to skip view-model tests.
- `cargo test --workspace` must pass. Coverage job is part of CI.
- Do not write tests that only assert the mock was called. Assert observable state.

## GUI — beauty is a functional requirement

The desktop app must be **absurdly beautiful**. Not “clean”. Not “fine for a utility”. It should look like a title sequence: dark grove, insect husks, sonar over a disk.

This is a merge-blocking rule. A feature that works but looks like a settings panel is incomplete.

Freya rules:

- Use Freya built-ins first (`Button`, `Slider`, `Switch`, `VirtualScrollView`, `ScrollView`, layout `rect`s).
- Build a huskmap design system in `huskmap-gui/src/theme.rs` + `components/`. No one-off colors in screens.
- One theme: **Afterlife** — near-black carbon, bone/ash text, oxidized copper accents, warning amber, danger oxblood. No purple SaaS, no default blue, no white dashboard.
- Typography: one display face for titles (tight, slightly strange), one mono face for paths and bytes. Numbers are tabular.
- Motion is required: scan pulse, map reveal, reclaim count-up, row hover, panel slide. Short, physical, no bounce spam.
- The **Husk Map** is the home screen, not a list dumped on boot. List/table is a drill-in.
- Empty states and errors are illustrated, copy is dry and sharp. No “Oops!”.
- Density of a pro tool (DaisyDisk / ncdu energy) with the drama of a creature that eats leftovers.
- 60fps on a normal laptop during scan visualization. Virtualize long lists.
- Keyboard-first: `j/k`, `/` search, `enter` inspect, `x` mark, `a` apply-with-confirm.

Icons:

- UI glyphs (actions, categories, wards, chrome) come from **Hugeicons** (stroke style), vendored as SVG under `huskmap-gui/assets/icons/`. One set, one stroke weight. No mixing icon families.
- Agents and toolchains are identified by their **real brand mark**, never a generic glyph. Claude is the Claude mark, not a Hugeicons "code" or "sparkle" icon; Codex is the OpenAI mark; Cursor, Gemini, OpenCode, Aider, Grok, npm, pnpm, bun, yarn, Python, uv, Rust/Cargo, Go and Playwright each use their own official SVG.
- Brand marks live under `huskmap-gui/assets/brands/`, each with its source and license noted in `huskmap-gui/assets/brands/SOURCES.md`. Prefer the vendor's press kit; Simple Icons (CC0) is the fallback.
- Keep brand marks unmodified in shape. Monochrome tinting to the theme (bone/ash/copper) is allowed; recoloring into a fake brand color, redrawing, or cropping is not.
- Map every agent/ecosystem to its mark in one table in `huskmap-gui/src/icons.rs`. `Unknown` gets a neutral Hugeicons glyph, never a borrowed brand.

Do not ship:

- Unstyled Freya defaults as the product look
- Rainbow charts
- Emoji soup in the chrome
- Light theme in v1 (token the theme anyway so it can exist later)

## CLI

Binary name: `huskmap`.

Minimum commands:

```
huskmap scan [PATH]... [--json] [--min-size] [--category]
huskmap doctor
huskmap plan [--preset safe|agent-only|older]
huskmap apply --plan <file>   # refuses without a plan
huskmap map                   # optional TUI of the last report
```

`huskmap` with no args opens the GUI when a display exists; otherwise runs `scan` in the TUI.

Help text and splash: short, sharp, same voice as the GUI.

## Code style

- `cargo fmt --check` and `clippy --all-targets -- -D warnings` are required.
- Errors: `thiserror` in core, `anyhow` only at CLI/GUI edges.
- Logging: `tracing`. No `println!` in core.
- Paths: `camino` or `std::path::PathBuf` consistently; never string-concatenate OS paths.
- No `TODO` left in main without an issue number.
- Commits and PRs in English. UI strings live in `huskmap-core/src/copy.rs`, English and PT-BR side by side.

## Copy: plain words

People must understand every label on first read, in both languages.

- Buttons, column names, filters, statuses and messages use words people already know: "Scan", "List", "Move to trash", "Safe to remove" / "Escanear", "Lista", "Mandar para a lixeira", "Pode apagar". Never the domain names (`husk`, `grove`, `ballast`, `afterimage`, `ledger`) and never metaphors ("sound the grove", "cascas", "bosque", "lastro"). `copy_uses_plain_words` enforces a list.
- The dark, mischievous voice lives only in taglines, empty states and errors, and even there it must be clear.
- PT-BR is written the way a Brazilian developer talks, not translated word by word. Keep English tech terms Brazilians use as-is (worktree, branch, commit, push, cache, log).
- Every screen is explained in the in-app guide (`Guide` in `copy.rs`). A new feature adds its line there in both languages.
- Do not add dependencies for one helper function.

## What to build, in order

1. `huskmap-core` scan + classify + size + `ScanReport` (tests first).
2. CLI `scan` / `doctor` / `--json`.
3. Plan + apply with dry-run and fixtures.
4. Freya shell: theme, map view, detail drawer, apply confirm.
5. Wire scan progress into the map (live husks appearing).
6. Packagers last (deb, rpm, pacman, apk, tar.gz, `cargo install`).

Do not start on installers, marketing pages, or extra agent adapters before the map + safe apply loop works.

## How an agent should work in this repo

1. Read this file and the crate `README`s.
2. Change core first if behavior changes.
3. Add or update tests in the same change.
4. Run `cargo test --workspace` and clippy.
5. Run coverage on core. If it dropped, fix tests before pushing.
6. If you touch GUI, describe the visual result in the PR (layout, motion, tokens used). A screenshot or frame dump when possible.
7. Never commit scan results from a real home directory.

If a requirement here conflicts with a comment in code, this file wins until a human edits it.
