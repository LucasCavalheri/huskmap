use std::path::PathBuf;

use huskmap_core::Dirs;

/// An explicit home (tests, `HUSKMAP_HOME`) gets pure XDG defaults so nothing leaks into the
/// developer's real `$XDG_*` dirs. Otherwise the real environment decides.
pub fn resolve_dirs(explicit: Option<PathBuf>) -> anyhow::Result<Dirs> {
    if let Some(home) = explicit {
        return Ok(Dirs::for_home(home));
    }
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
        .or_else(dirs::home_dir)
        .ok_or_else(|| anyhow::anyhow!("could not locate home directory"))?;
    Ok(Dirs::current(home))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn explicit_home_ignores_environment() {
        let d = resolve_dirs(Some(PathBuf::from("/tmp/husk-home"))).unwrap();
        assert_eq!(d.home, PathBuf::from("/tmp/husk-home"));
        assert_eq!(
            d.last_report(),
            PathBuf::from("/tmp/husk-home/.local/state/huskmap/last-report.json")
        );
    }

    #[test]
    fn real_home_resolves() {
        let d = resolve_dirs(None).unwrap();
        assert!(d.home.is_absolute());
    }
}
