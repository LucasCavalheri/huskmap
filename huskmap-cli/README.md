# huskmap-cli

The `huskmap` binary.

```
huskmap                              # desktop map on X11/Wayland, text scan otherwise
huskmap gui
huskmap scan [PATH]... [--json] [--min-size 500M] [--category worktree] [--all]
huskmap doctor [--json]
huskmap plan [--preset safe|agent-only|older] [--only PATH]... [--force] [-o plan.json]
huskmap apply --plan plan.json [--force]
huskmap map                          # TUI of the last report
huskmap update [--check] [-y]
huskmap completions bash|zsh|fish
huskmap --lang pt-br ...             # or HUSKMAP_LANG
```

Text output leads with **Occupied & unsaved**: worktrees someone is inside, with uncommitted changes, or holding commits that exist nowhere else. Colors follow the Afterlife palette and respect `NO_COLOR`.

`apply` holds `$XDG_STATE_HOME/huskmap/apply.lock` while it runs; a second apply, the updater and `install.sh` wait for it.

Hidden, for tooling: `huskmap status` prints stable `key=value` lines (`window`, `phase`, `marked`, `marked_bytes`, `applying`, `apply_pid`, …) that `install.sh` reads; `huskmap man` prints the roff man page.

The last report is kept at `$XDG_STATE_HOME/huskmap/last-report.json`. `HUSKMAP_HOME` points everything at another home and ignores the real `XDG_*` variables (tests use it).
