# Changelog

All notable changes to huskmap. Dates are UTC. Versions follow SemVer.

## 0.1.0 - 2026-09-23

First release. Linux only, every Linux.

- **A map of what AI coding agents left on your disk**: worktrees, dependency folders (`node_modules`, `target`, `.venv`, `.next`, `.turbo` and friends, only when their project file is there), package caches (npm, pnpm, yarn, bun, pip, uv, poetry, cargo, go, Playwright), agent sessions matched to their project, agent caches and logs. Claude Code, Codex, Cursor, Grok, Gemini CLI, OpenCode and Aider.
- **Your work is protected**: folders in use (read from `/proc`) are never removed, not even by force. Uncommitted changes, commits that exist nowhere else, git locks, the main checkout and secret files keep a folder where it is.
- **Nothing moves until you confirm**: scanning only reads. On confirm, huskmap checks everything again, moves the items to the system trash (so you can restore them) and removes worktrees from git's list. One removal runs at a time.
- **Mark anyway** for protected worktrees you are sure about, with its own warning. Branches always stay in the repo.
- **Filters**: chips above the list, or type them (`kind:deps size:>500mb age:>30d -is:blocked`), in English or Portuguese. Sort by size, age or name, and mark everything visible that can go.
- **Light and dark themes**, or follow the desktop. Switch with `t`.
- **How it works**: a built-in guide on first launch and on `?`.
- **English and Brazilian Portuguese**, picked from your settings, timezone or locale, and switchable any time.
- **Marks survive** restarts and updates.
- **Updates**: a daily check against GitHub Releases, verified with SHA256SUMS and installed the same way huskmap was installed. `huskmap update`, or turn it off with `HUSKMAP_NO_UPDATE_CHECK=1`.
- **Install anywhere**: `.deb`, `.rpm`, `.pkg.tar.zst`, `.apk`, a portable `.tar.gz`, or `curl … | bash`, which waits for a running removal and reopens the map with your marks.
- **CLI** for scripts and headless machines: `scan` (with `--json`), `doctor`, `plan`, `apply`, `map` (terminal UI), `update`, `completions` and a man page.
