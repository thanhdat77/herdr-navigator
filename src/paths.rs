use std::{
    env, fs, io,
    path::{Path, PathBuf},
};

pub(crate) fn plugin_config_dir() -> PathBuf {
    env::var("HERDR_PLUGIN_CONFIG_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| home().join(".config/herdr/plugins/config/herdr-navigator"))
}

pub(crate) fn migrate_legacy_plugin_config() {
    let current = plugin_config_dir();
    let legacy = home().join(".config/herdr/plugins/config/herdr-picker-plus");
    if current != legacy {
        let _ = copy_missing_files(&legacy, &current);
    }
}

fn copy_missing_files(from: &Path, to: &Path) -> io::Result<()> {
    if !from.is_dir() {
        return Ok(());
    }
    fs::create_dir_all(to)?;
    for entry in fs::read_dir(from)? {
        let entry = entry?;
        let destination = to.join(entry.file_name());
        if entry.file_type()?.is_file() && !destination.exists() {
            fs::copy(entry.path(), destination)?;
        }
    }
    Ok(())
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn legacy_config_files_copy_without_overwriting_new_values() {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let old = env::temp_dir().join(format!("herdr-navigator-old-{suffix}"));
        let new = env::temp_dir().join(format!("herdr-navigator-new-{suffix}"));
        fs::create_dir_all(&old).unwrap();
        fs::create_dir_all(&new).unwrap();
        fs::write(old.join("config.toml"), "old").unwrap();
        fs::write(old.join("jump-back-workspace"), "w1").unwrap();
        fs::write(new.join("config.toml"), "new").unwrap();

        copy_missing_files(&old, &new).unwrap();

        assert_eq!(fs::read_to_string(new.join("config.toml")).unwrap(), "new");
        assert_eq!(
            fs::read_to_string(new.join("jump-back-workspace")).unwrap(),
            "w1"
        );
        let _ = fs::remove_dir_all(old);
        let _ = fs::remove_dir_all(new);
    }
}
