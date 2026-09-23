use std::path::{Path, PathBuf};

use crate::domain::HuskKind;

/// Stable identity for a husk: kind plus the path as scanned.
#[derive(
    Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
pub struct HuskId {
    pub kind: HuskKind,
    pub path: PathBuf,
}

impl HuskId {
    pub fn new(kind: HuskKind, path: impl Into<PathBuf>) -> Self {
        Self {
            kind,
            path: path.into(),
        }
    }

    pub fn as_path(&self) -> &Path {
        &self.path
    }
}

impl std::fmt::Display for HuskId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}", self.kind.as_str(), self.path.display())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stable_and_display() {
        let a = HuskId::new(HuskKind::Ballast, "/tmp/node_modules");
        let b = HuskId::new(HuskKind::Ballast, "/tmp/node_modules");
        assert_eq!(a, b);
        assert_eq!(a.to_string(), "ballast:/tmp/node_modules");
        assert_eq!(a.as_path(), Path::new("/tmp/node_modules"));
    }

    #[test]
    fn kind_distinguishes() {
        let a = HuskId::new(HuskKind::Cache, "/x");
        let b = HuskId::new(HuskKind::Ballast, "/x");
        assert_ne!(a, b);
    }
}
