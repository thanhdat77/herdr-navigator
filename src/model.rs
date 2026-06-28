use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::paths::canonical_str;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) enum Source {
    Workspace,
    Project,
    Zoxide,
    Root,
    Agent,
    QuickAction,
    Integration,
}

impl Source {
    pub(crate) fn label(&self) -> &'static str {
        match self {
            Source::Workspace => "open",
            Source::Project => "project",
            Source::Zoxide => "zoxide",
            Source::Root => "root",
            Source::Agent => "agent",
            Source::QuickAction => "quick",
            Source::Integration => "plugin",
        }
    }

    pub(crate) fn from_config(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "workspace" | "workspaces" | "open" | "open_workspaces" => Some(Source::Workspace),
            "project" | "projects" | "herdr_plus_projects" => Some(Source::Project),
            "zoxide" | "z" => Some(Source::Zoxide),
            "root" | "roots" | "scan" => Some(Source::Root),
            "agent" | "agents" => Some(Source::Agent),
            "quick" | "quick_action" | "quick_actions" | "herdr_plus_quick_actions" => {
                Some(Source::QuickAction)
            }
            "plugin" | "integration" | "integrations" => Some(Source::Integration),
            _ => None,
        }
    }

    pub(crate) fn all() -> [Source; 7] {
        [
            Source::Workspace,
            Source::Project,
            Source::Zoxide,
            Source::Root,
            Source::Agent,
            Source::QuickAction,
            Source::Integration,
        ]
    }
}

#[derive(Clone, Debug)]
pub(crate) enum EntryAction {
    FocusWorkspace {
        id: String,
    },
    FocusAgent {
        target: String,
    },
    OpenProject,
    InvokePluginAction {
        action: String,
    },
    FocusOrCreateDir,
    RunCommand {
        command: String,
        notify_success: bool,
        notify_error: bool,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum WorkspaceKind {
    Project,
    Dir,
    Unknown,
}

#[derive(Clone, Debug)]
pub(crate) struct WorkspaceRef {
    pub(crate) id: String,
    pub(crate) label: String,
    pub(crate) kind: WorkspaceKind,
    pub(crate) path: PathBuf,
    pub(crate) tab_count: i64,
    pub(crate) pane_count: i64,
}

#[derive(Clone, Debug)]
pub(crate) struct Entry {
    pub(crate) source: Source,
    pub(crate) title: String,
    pub(crate) subtitle: String,
    pub(crate) path: PathBuf,
    pub(crate) workspace_id: Option<String>,
    pub(crate) agent_target: Option<String>,
    pub(crate) project: Option<Project>,
    pub(crate) action: EntryAction,
    pub(crate) source_label: Option<String>,
}

impl Entry {
    pub(crate) fn key(&self) -> String {
        canonical_str(&self.path).unwrap_or_else(|| self.path.display().to_string())
    }

    pub(crate) fn source_name(&self) -> &str {
        self.source_label
            .as_deref()
            .unwrap_or_else(|| self.source.label())
    }

    pub(crate) fn haystack(&self) -> String {
        format!(
            "{} {} {} {}",
            self.source_name(),
            self.title,
            self.subtitle,
            self.path.display()
        )
        .to_lowercase()
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct Project {
    pub(crate) name: String,
    #[serde(default)]
    pub(crate) description: String,
    pub(crate) working_dir: String,
    #[serde(default)]
    pub(crate) tabs: Vec<ProjectTab>,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct ProjectTab {
    pub(crate) name: String,
    pub(crate) command: Option<String>,
}
