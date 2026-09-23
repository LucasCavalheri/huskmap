use std::io;
use std::path::PathBuf;

/// Core error. Views map this at their edges.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("io failed at {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("git: {0}")]
    Git(String),
    #[error("path is outside scan roots: {0}")]
    OutsideRoots(PathBuf),
    #[error("plan is stale: {0}")]
    StalePlan(String),
    #[error("refusing to mutate: {0}")]
    Safety(String),
    #[error("forbidden path: {0}")]
    Forbidden(PathBuf),
    #[error("unsupported {kind} version {found}, want {want}")]
    Version {
        kind: &'static str,
        found: u32,
        want: u32,
    },
    #[error("json: {0}")]
    Json(#[from] serde_json::Error),
    #[error("trash failed at {path}: {message}")]
    Trash { path: PathBuf, message: String },
    #[error("update: {0}")]
    Update(String),
}

impl Error {
    pub fn io(path: impl Into<PathBuf>, source: io::Error) -> Self {
        Self::Io {
            path: path.into(),
            source,
        }
    }

    pub fn git(err: impl ToString) -> Self {
        Self::Git(err.to_string())
    }

    pub fn safety(msg: impl Into<String>) -> Self {
        Self::Safety(msg.into())
    }

    pub fn update(msg: impl Into<String>) -> Self {
        Self::Update(msg.into())
    }

    pub fn stale(msg: impl Into<String>) -> Self {
        Self::StalePlan(msg.into())
    }
}

impl From<git2::Error> for Error {
    fn from(value: git2::Error) -> Self {
        Self::Git(value.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::ErrorKind;

    #[test]
    fn io_display_includes_path() {
        let err = Error::io("/tmp/x", io::Error::from(ErrorKind::NotFound));
        let text = err.to_string();
        assert!(text.contains("/tmp/x"), "{text}");
        assert!(text.contains("io failed"), "{text}");
    }

    #[test]
    fn git_from_string() {
        assert_eq!(Error::git("boom").to_string(), "git: boom");
    }

    #[test]
    fn safety_and_stale() {
        assert!(Error::safety("nope").to_string().contains("nope"));
        assert!(Error::stale("drift").to_string().contains("drift"));
    }

    #[test]
    fn version_mismatch() {
        let err = Error::Version {
            kind: "plan",
            found: 9,
            want: 1,
        };
        assert_eq!(err.to_string(), "unsupported plan version 9, want 1");
    }

    #[test]
    fn forbidden_and_outside() {
        assert!(
            Error::Forbidden(PathBuf::from("/.env"))
                .to_string()
                .contains(".env")
        );
        assert!(
            Error::OutsideRoots(PathBuf::from("/etc"))
                .to_string()
                .contains("/etc")
        );
    }

    #[test]
    fn update_display() {
        assert_eq!(Error::update("no net").to_string(), "update: no net");
    }

    #[test]
    fn trash_display() {
        let err = Error::Trash {
            path: PathBuf::from("/tmp/h"),
            message: "no bin".into(),
        };
        assert!(err.to_string().contains("no bin"));
    }
}
