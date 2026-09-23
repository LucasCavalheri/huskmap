# Coverage

Tool: `cargo-llvm-cov` (alias `cargo cov`). Target: 100% on `huskmap-core`; floor 95% lines.

Current: **98.4% lines**, 95.9% functions, 97.7% regions. Stable `llvm-cov` does not report branches; the branch floor is checked by review until the toolchain supports it.

## Residual

- `reclaim.rs` `TrashReclaimer`: moving a file into the freedesktop trash would write into the developer's `~/.local/share/Trash`, which tests must never touch. `FsReclaimer` covers the same `Reclaimer` contract against temp dirs.
- `git.rs` `stranded_commits`: the `revwalk()` / `push()` failure arms need a corrupt object database.
- `scan.rs`: the `WalkDir` entry error arm and the "size failed after classify" arm need a path that vanishes between two syscalls.
- `update.rs` `Https`: the real HTTPS client. Tests go through the `Fetch` trait with a fake; CI never phones GitHub.
- `update.rs` `SystemHost::run` pkexec 126/127 arm: needs a real polkit prompt to dismiss.
- `settings.rs` `update_check_disabled_by_env` reads the process environment; the cadence rule it feeds is covered.
- `#[cfg(test)]` `panic!("{other:?}")` arms in `classify.rs` tests never run on green tests.

OS edges stay behind `GitProbe`, `ProcessProbe`, `Reclaimer`, `Clock`, `Fetch` and `Host`, each with a fake used by the tests.
