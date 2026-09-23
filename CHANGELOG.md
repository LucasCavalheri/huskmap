# Changelog

All notable changes to huskmap. Dates are UTC. Versions follow SemVer.

## 0.1.0 - 2026-09-23

First release. Linux only, every Linux.

- **Husk Map**: a sonar dial of what agents left on disk. Angle is kind, radius is age, size is weight, color is what you may do. Live husks stream in while a scan runs.
- **Worktrees are guarded**: occupied (a process works inside, read from `/proc`), uncommitted changes, commits that exist on no other ref, git locks, primary checkouts and unique secrets. Occupied ones never go, not even by force.
- **Finds** linked worktrees and agent slots (Claude, Codex, Cursor), marker-checked ballast (`node_modules`, `target`, `.venv`, `.next`, `.turbo`, `.nuxt`, `.svelte-kit`, Python caches), package caches (npm, pnpm, yarn, bun, pip, uv, poetry, cargo, go, playwright), agent sessions (Claude and Grok matched to their project), caches and logs.
- **Safe apply**: scan, plan, apply. Apply re-checks size, mtime, git and processes, sends husks to the freedesktop trash, then prunes worktrees from git. One apply at a time (`apply.lock`).
- **Force from the map** for guarded worktrees, with its own warning; the branch stays in the repo and files stay in the trash.
- **Marks survive** restarts and updates (`$XDG_STATE_HOME/huskmap/session.json`).
- **Updates**: a daily check against GitHub Releases, SHA256-verified, installed the way huskmap was installed (deb, rpm, pacman, apk, portable). `huskmap update`, or `HUSKMAP_NO_UPDATE_CHECK=1`.
- **Install anywhere**: `.deb`, `.rpm`, `.pkg.tar.zst`, `.apk`, portable `tar.gz`, and `install.sh`, which waits for a running apply and reopens the map with its marks.
- **Light and dark themes**: Daylight and Afterlife, or follow the desktop. Switch with `t` or from the status bar.
- **How it works**: an in-app guide that opens on first launch and on `?`.
- **Filters**: chips and a typed query language (`kind:deps size:>500mb age:>30d -is:blocked`), in English or Portuguese, shared by the map and the list. Sortable list columns and "Mark N removable".
- **English and Brazilian Portuguese**, in plain words, chosen by flag, saved pick, then timezone/locale (Brazil reads pt-BR).
- CLI: `scan`, `doctor`, `plan`, `apply`, `map` (TUI), `update`, `completions`; man page.
