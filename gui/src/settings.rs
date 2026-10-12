//! The remembered project choice, kept in a small file between runs.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Settings {
    /// Folder name of the project chosen last.
    #[serde(default)]
    pub project: Option<String>,
}

/// `$NOCODO_GUI_SETTINGS`, else `~/.config/nocodo/gui.json`.
pub fn default_path() -> Option<PathBuf> {
    if let Some(p) = std::env::var_os("NOCODO_GUI_SETTINGS") {
        return Some(p.into());
    }
    let home = std::env::var_os("HOME")?;
    Some(PathBuf::from(home).join(".config/nocodo/gui.json"))
}

/// Missing or unreadable files give the defaults: remembering is a convenience only.
pub fn load(path: &Path) -> Settings {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default()
}

pub fn save(path: &Path, settings: &Settings) -> std::io::Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let text = serde_json::to_string_pretty(settings).map_err(std::io::Error::other)?;
    std::fs::write(path, text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_and_defaults() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nested/gui.json");
        assert_eq!(load(&path), Settings::default());
        let s = Settings {
            project: Some("g-finance".into()),
        };
        save(&path, &s).unwrap();
        assert_eq!(load(&path), s);
    }

    #[test]
    fn broken_file_gives_defaults() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("gui.json");
        std::fs::write(&path, "not json").unwrap();
        assert_eq!(load(&path), Settings::default());
    }
}
