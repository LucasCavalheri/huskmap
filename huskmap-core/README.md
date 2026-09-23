# huskmap-core

Domain crate. Scan, classify, size, judge, plan, apply. No UI; the CLI and the desktop map are views over it.

Public nouns: `Husk`, `Grove`, `Afterimage`, `Ballast`, `Ward`, `ScanReport`, `Plan`, `ApplyResult`.

- `roots.rs`: the one table of scan roots, resolved against the XDG base dirs (`Dirs`).
- `classify.rs`: marker rules. Ballast needs a marker; agent-home rules never apply inside a worktree.
- `git.rs`: checkout facts, stranded commits (reachable from HEAD and no other ref), trash-then-prune.
- `process.rs`: who works where, read from `/proc`.
- `secrets.rs`: a secret guards a husk only when it is the only copy.
- `copy.rs`: every string, English and PT-BR, plus locale choice (explicit, saved, then Brazil or not).

`cargo cov` runs coverage for this crate. Residuals live in `COVERAGE.md` at the repo root.
