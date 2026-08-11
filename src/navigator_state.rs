use std::{
    collections::{HashMap, HashSet},
    fs,
    io::ErrorKind,
    path::Path,
};

use serde::{Deserialize, Serialize};

use crate::{model::WorkspaceKind, paths::plugin_state_dir};

const SNAPSHOT_FILE: &str = "navigator-state.json";
const SNAPSHOT_VERSION: u32 = 1;

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct NavigatorSnapshot {
    #[serde(default = "snapshot_version")]
    version: u32,
    #[serde(default)]
    workspaces: HashMap<String, NavigatorWorkspace>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
enum NavigatorWorkspaceKind {
    Dir,
    Project,
}

#[derive(Debug, Serialize, Deserialize)]
struct NavigatorWorkspace {
    kind: NavigatorWorkspaceKind,
}

impl Default for NavigatorSnapshot {
    fn default() -> Self {
        Self {
            version: SNAPSHOT_VERSION,
            workspaces: HashMap::new(),
        }
    }
}

fn snapshot_version() -> u32 {
    SNAPSHOT_VERSION
}

impl NavigatorSnapshot {
    pub(crate) fn load() -> Self {
        Self::load_from(&plugin_state_dir().join(SNAPSHOT_FILE))
    }

    fn load_from(path: &Path) -> Self {
        let content = match fs::read_to_string(path) {
            Ok(content) => content,
            Err(error) if error.kind() == ErrorKind::NotFound => return Self::default(),
            Err(error) => {
                eprintln!(
                    "warning: failed to read Navigator state {}: {error}",
                    path.display()
                );
                return Self::default();
            }
        };
        match serde_json::from_str::<Self>(&content) {
            Ok(snapshot) if snapshot.version == SNAPSHOT_VERSION => snapshot,
            Ok(snapshot) => {
                eprintln!(
                    "warning: ignoring unsupported Navigator state version {} (supports {})",
                    snapshot.version, SNAPSHOT_VERSION
                );
                Self::default()
            }
            Err(error) => {
                eprintln!("warning: ignoring malformed Navigator state: {error}");
                Self::default()
            }
        }
    }

    pub(crate) fn save(&self) -> Result<(), String> {
        self.save_to(&plugin_state_dir().join(SNAPSHOT_FILE))
    }

    fn save_to(&self, path: &Path) -> Result<(), String> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .map_err(|error| format!("failed to create Navigator state directory: {error}"))?;
        }
        let json = serde_json::to_string_pretty(self)
            .map_err(|error| format!("failed to encode Navigator state: {error}"))?;
        let tmp_path = path.with_extension("json.tmp");
        fs::write(&tmp_path, json)
            .map_err(|error| format!("failed to write Navigator state: {error}"))?;
        if let Err(error) = fs::rename(&tmp_path, path) {
            let _ = fs::remove_file(&tmp_path);
            return Err(format!("failed to replace Navigator state: {error}"));
        }
        Ok(())
    }

    pub(crate) fn workspace_kind(&self, id: &str, label: &str) -> WorkspaceKind {
        self.workspaces
            .get(id)
            .map(|workspace| match workspace.kind {
                NavigatorWorkspaceKind::Dir => WorkspaceKind::Dir,
                NavigatorWorkspaceKind::Project => WorkspaceKind::Project,
            })
            .unwrap_or_else(|| legacy_workspace_kind(label))
    }

    pub(crate) fn migrate_legacy_kind(&mut self, id: &str, label: &str) -> bool {
        if self.workspaces.contains_key(id) {
            return false;
        }
        let kind = legacy_workspace_kind(label);
        self.record(id, kind)
    }

    pub(crate) fn record(&mut self, id: &str, kind: WorkspaceKind) -> bool {
        let kind = match kind {
            WorkspaceKind::Dir => NavigatorWorkspaceKind::Dir,
            WorkspaceKind::Project => NavigatorWorkspaceKind::Project,
            WorkspaceKind::Unknown => return false,
        };
        let changed = !matches!(self.workspaces.get(id), Some(workspace) if workspace.kind == kind);
        self.workspaces
            .insert(id.into(), NavigatorWorkspace { kind });
        changed
    }

    pub(crate) fn reconcile<I>(&mut self, live_workspace_ids: I) -> bool
    where
        I: IntoIterator,
        I::Item: AsRef<str>,
    {
        let live: HashSet<String> = live_workspace_ids
            .into_iter()
            .map(|id| id.as_ref().to_owned())
            .collect();
        let old_len = self.workspaces.len();
        self.workspaces.retain(|id, _| live.contains(id));
        self.workspaces.len() != old_len
    }
}

fn legacy_workspace_kind(label: &str) -> WorkspaceKind {
    let label = label.trim().to_ascii_lowercase();
    if label.starts_with("project:") {
        WorkspaceKind::Project
    } else if label.starts_with("dir:") {
        WorkspaceKind::Dir
    } else {
        WorkspaceKind::Unknown
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        path::PathBuf,
        time::{SystemTime, UNIX_EPOCH},
    };

    fn state_path(name: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir()
            .join(format!("herdr-navigator-{name}-{unique}"))
            .join(SNAPSHOT_FILE)
    }

    #[test]
    fn saves_versioned_metadata_with_atomic_replacement() {
        let path = state_path("save");
        let mut snapshot = NavigatorSnapshot::default();
        snapshot.record("w1", WorkspaceKind::Dir);

        snapshot.save_to(&path).unwrap();

        assert!(!path.with_extension("json.tmp").exists());
        assert_eq!(
            fs::read_to_string(&path).unwrap(),
            "{\n  \"version\": 1,\n  \"workspaces\": {\n    \"w1\": {\n      \"kind\": \"dir\"\n    }\n  }\n}"
        );
        let _ = fs::remove_dir_all(path.parent().unwrap());
    }

    #[test]
    fn missing_malformed_or_unsupported_snapshots_are_ignored() {
        let path = state_path("invalid");
        assert!(NavigatorSnapshot::load_from(&path).workspaces.is_empty());
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, "not json").unwrap();
        assert!(NavigatorSnapshot::load_from(&path).workspaces.is_empty());

        fs::write(&path, r#"{"version":2,"workspaces":{}}"#).unwrap();
        assert!(NavigatorSnapshot::load_from(&path).workspaces.is_empty());
        let _ = fs::remove_dir_all(path.parent().unwrap());
    }

    #[test]
    fn persisted_kind_wins_and_legacy_labels_migrate() {
        let mut snapshot = NavigatorSnapshot::default();
        snapshot.record("w1", WorkspaceKind::Dir);
        assert_eq!(
            snapshot.workspace_kind("w1", "project: renamed"),
            WorkspaceKind::Dir
        );
        assert!(snapshot.migrate_legacy_kind("w2", "project: old"));
        assert_eq!(
            snapshot.workspace_kind("w2", "renamed"),
            WorkspaceKind::Project
        );
    }

    #[test]
    fn reconcile_drops_closed_workspaces() {
        let mut snapshot = NavigatorSnapshot::default();
        snapshot.record("w1", WorkspaceKind::Dir);
        snapshot.record("w2", WorkspaceKind::Project);

        assert!(snapshot.reconcile(["w2"]));
        assert!(!snapshot.workspaces.contains_key("w1"));
        assert!(snapshot.workspaces.contains_key("w2"));
    }
}
