//! Per-user settings in `$XDG_CONFIG_HOME/huskmap/settings.json`. Small on purpose.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::clock::Millis;
use crate::error::Error;

/// Once a day is plenty for a disk map.
pub const UPDATE_INTERVAL_MS: Millis = 24 * 60 * 60 * 1000;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    /// `en` or `pt-br`, picked in the app. `None` lets geography decide.
    pub lang: Option<String>,
    /// Ask GitHub Releases for a newer huskmap, at most once a day.
    pub check_updates: bool,
    pub last_update_check_ms: Option<Millis>,
    /// A release the person chose to skip.
    pub skipped_version: Option<String>,
    /// The "How it works" guide was closed once; it no longer opens by itself.
    pub guide_seen: bool,
    /// `system`, `light` or `dark`. `None` follows the desktop.
    pub theme: Option<String>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            lang: None,
            check_updates: true,
            last_update_check_ms: None,
            skipped_version: None,
            guide_seen: false,
            theme: None,
        }
    }
}

impl Settings {
    pub fn path(config_dir: &Path) -> PathBuf {
        config_dir.join("huskmap").join("settings.json")
    }

    /// Missing or unreadable settings are defaults, never an error: a broken file must not
    /// keep the map from opening.
    pub fn load(config_dir: &Path) -> Self {
        std::fs::read_to_string(Self::path(config_dir))
            .ok()
            .and_then(|t| serde_json::from_str(&t).ok())
            .unwrap_or_default()
    }

    pub fn save(&self, config_dir: &Path) -> Result<(), Error> {
        let path = Self::path(config_dir);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| Error::io(parent, e))?;
        }
        let text = serde_json::to_string_pretty(self)?;
        let tmp = path.with_extension("json.tmp");
        std::fs::write(&tmp, text).map_err(|e| Error::io(&tmp, e))?;
        std::fs::rename(&tmp, &path).map_err(|e| Error::io(&path, e))
    }

    /// Load, change, save.
    pub fn update(config_dir: &Path, f: impl FnOnce(&mut Settings)) -> Result<Settings, Error> {
        let mut s = Self::load(config_dir);
        f(&mut s);
        s.save(config_dir)?;
        Ok(s)
    }

    /// `HUSKMAP_NO_UPDATE_CHECK=1` wins over the file; then the switch; then the daily cadence.
    pub fn update_check_due(&self, now_ms: Millis, env_off: bool) -> bool {
        if env_off || !self.check_updates {
            return false;
        }
        self.last_update_check_ms
            .is_none_or(|last| now_ms.saturating_sub(last) >= UPDATE_INTERVAL_MS)
    }
}

pub fn update_check_disabled_by_env() -> bool {
    std::env::var_os("HUSKMAP_NO_UPDATE_CHECK").is_some_and(|v| !v.is_empty() && v != "0")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_defaults_and_broken_file() {
        let tmp = tempfile::tempdir().unwrap();
        assert_eq!(Settings::load(tmp.path()), Settings::default());
        let saved = Settings::update(tmp.path(), |s| s.lang = Some("pt-br".into())).unwrap();
        assert_eq!(Settings::load(tmp.path()), saved);
        assert!(Settings::path(tmp.path()).ends_with("huskmap/settings.json"));
        std::fs::write(Settings::path(tmp.path()), "{ not json").unwrap();
        assert_eq!(Settings::load(tmp.path()), Settings::default());
        std::fs::write(Settings::path(tmp.path()), r#"{"lang":"en"}"#).unwrap();
        let partial = Settings::load(tmp.path());
        assert!(partial.check_updates, "missing keys take defaults");
        assert!(!partial.guide_seen, "people upgrading see the guide once");
        assert_eq!(
            partial.theme, None,
            "the desktop decides until someone picks"
        );
        let blocked = tmp.path().join("file");
        std::fs::write(&blocked, "x").unwrap();
        assert!(Settings::default().save(&blocked).is_err());
    }

    #[test]
    fn update_cadence() {
        let mut s = Settings::default();
        assert!(s.update_check_due(0, false));
        assert!(!s.update_check_due(0, true), "env turns it off");
        s.last_update_check_ms = Some(1_000);
        assert!(!s.update_check_due(1_000 + UPDATE_INTERVAL_MS - 1, false));
        assert!(s.update_check_due(1_000 + UPDATE_INTERVAL_MS, false));
        s.check_updates = false;
        assert!(!s.update_check_due(u64::MAX, false));
        let _ = update_check_disabled_by_env();
    }
}
