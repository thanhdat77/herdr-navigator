use std::path::PathBuf;

use serde::Deserialize;

use crate::paths::canonical_str;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) enum Source {
    Workspace,
    Project,
    Zoxide,
    Root,
    Agent,
    QuickAction,
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
            _ => None,
        }
    }

    pub(crate) fn all() -> [Source; 6] {
        [
            Source::Workspace,
            Source::Project,
            Source::Zoxide,
            Source::Root,
            Source::Agent,
            Source::QuickAction,
        ]
    }
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
}

impl Entry {
    pub(crate) fn key(&self) -> String {
        canonical_str(&self.path).unwrap_or_else(|| self.path.display().to_string())
    }

    pub(crate) fn haystack(&self) -> String {
        format!(
            "{} {} {} {}",
            self.source.label(),
            self.title,
            self.subtitle,
            self.path.display()
        )
        .to_lowercase()
    }
}

#[derive(Clone, Debug, Deserialize)]
pub(crate) struct Project {
    pub(crate) name: String,
    #[serde(default)]
    pub(crate) description: String,
    pub(crate) working_dir: String,
    #[serde(default)]
    pub(crate) tabs: Vec<ProjectTab>,
}
#[derive(Clone, Debug, Deserialize)]
pub(crate) struct ProjectTab {
    pub(crate) name: String,
    pub(crate) command: Option<String>,
}
