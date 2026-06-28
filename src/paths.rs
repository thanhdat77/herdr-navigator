use std::{
    env, fs,
    path::{Path, PathBuf},
};

pub(crate) fn plugin_config_dir() -> PathBuf {
    env::var("HERDR_PLUGIN_CONFIG_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| home().join(".config/herdr/plugins/config/herdr-picker-plus"))
}
pub(crate) fn herdr_plus_projects_dir() -> PathBuf {
    home().join(".config/herdr/plugins/config/cloudmanic.herdr-plus/projects")
}
pub(crate) fn herdr_plus_quick_actions_dir() -> PathBuf {
    home().join(".config/herdr/plugins/config/cloudmanic.herdr-plus/quick-actions")
}
pub(crate) fn home() -> PathBuf {
    env::var("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/"))
}
pub(crate) fn expand_path(s: &str) -> PathBuf {
    if let Some(rest) = s.strip_prefix("~/") {
        home().join(rest)
    } else if s == "~" {
        home()
    } else {
        PathBuf::from(s.replace("$HOME", &home().display().to_string()))
    }
}
pub(crate) fn basename(path: &Path) -> String {
    path.file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("workspace")
        .to_string()
}
pub(crate) fn canonical_str(path: &Path) -> Option<String> {
    fs::canonicalize(path).ok().map(|p| p.display().to_string())
}
