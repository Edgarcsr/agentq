//! The user's settings, kept between runs in `~/.agentq/config.toml`.
//!
//! The folder can be moved with `AGENTQ_HOME`. A missing or broken file is
//! the same as no settings at all: the program must always start.

use crate::i18n::Lang;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// What the user chose in the settings screen. Anything unset falls back to
/// the default (the language of the system, the default logo).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    /// A language tag, as in `--lang`.
    pub language: Option<String>,
    /// The name of a logo, such as `lock`.
    pub logo: Option<String>,
}

impl Settings {
    /// The folder that holds the settings: `$AGENTQ_HOME`, else `~/.agentq`.
    pub fn dir() -> Option<PathBuf> {
        if let Some(dir) = std::env::var_os("AGENTQ_HOME").filter(|v| !v.is_empty()) {
            return Some(PathBuf::from(dir));
        }
        let home = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE"))?;
        Some(PathBuf::from(home).join(".agentq"))
    }

    /// The settings file.
    pub fn path() -> Option<PathBuf> {
        Settings::dir().map(|dir| dir.join("config.toml"))
    }

    /// Reads the saved settings; empty when there are none or they are broken.
    pub fn load() -> Settings {
        Settings::path()
            .and_then(|path| std::fs::read_to_string(path).ok())
            .and_then(|text| toml::from_str(&text).ok())
            .unwrap_or_default()
    }

    /// Writes the settings, creating the folder when needed.
    pub fn save(&self) -> std::io::Result<()> {
        let path = Settings::path().ok_or_else(|| std::io::Error::other("no home folder"))?;
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let text = toml::to_string_pretty(self).map_err(std::io::Error::other)?;
        std::fs::write(path, text)
    }

    /// The saved language, when there is one we know.
    pub fn lang(&self) -> Option<Lang> {
        self.language.as_deref().and_then(Lang::parse)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_round_trip_through_toml() {
        let settings = Settings {
            language: Some("pt-BR".into()),
            logo: Some("key".into()),
        };
        let text = toml::to_string_pretty(&settings).unwrap();
        assert_eq!(toml::from_str::<Settings>(&text).unwrap(), settings);
        assert_eq!(settings.lang(), Some(Lang::PtBr));
    }

    #[test]
    fn missing_or_unknown_fields_are_fine() {
        assert_eq!(toml::from_str::<Settings>("").unwrap(), Settings::default());
        let extra: Settings = toml::from_str("logo = \"lock\"\nfuture = 1").unwrap();
        assert_eq!(extra.logo.as_deref(), Some("lock"));
        assert_eq!(extra.lang(), None);
    }
}
