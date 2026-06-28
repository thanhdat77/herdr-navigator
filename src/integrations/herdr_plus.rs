use std::{env, fs, path::Path};

use serde_json::Value;

use crate::{
    herdr::{herdr_json, run_herdr},
    model::{Entry, EntryAction, Project, Source},
    paths::{expand_path, herdr_plus_projects_dir, home},
};

pub(crate) fn collect_projects() -> Vec<Entry> {
    let mut out = Vec::new();
    let dir = herdr_plus_projects_dir();
    let Ok(read) = fs::read_dir(dir) else {
        return out;
    };
    for file in read.flatten() {
        let path = file.path();
        if path.extension().and_then(|s| s.to_str()) != Some("toml") {
            continue;
        }
        let Ok(s) = fs::read_to_string(&path) else {
            continue;
        };
        let Ok(project) = toml::from_str::<Project>(&s) else {
            continue;
        };
        let p = expand_path(&project.working_dir);
        out.push(Entry {
            source: Source::Project,
            title: project.name.clone(),
            subtitle: project.description.clone(),
            path: p,
            workspace_id: None,
            agent_target: None,
            project: Some(project),
            action: EntryAction::OpenProject,
            source_label: None,
        });
    }
    out
}

pub(crate) fn quick_actions_entry() -> Entry {
    Entry {
        source: Source::QuickAction,
        title: "Herdr Plus Quick Actions".into(),
        subtitle: "open the Herdr Plus quick-action picker".into(),
        path: env::current_dir().unwrap_or_else(|_| home()),
        workspace_id: None,
        agent_target: None,
        project: None,
        action: EntryAction::InvokePluginAction {
            action: "cloudmanic.herdr-plus.quick-actions".into(),
        },
        source_label: None,
    }
}

pub(crate) fn bootstrap_project_tabs(
    project: &Project,
    create_json: &Value,
    cwd: &Path,
) -> Result<(), String> {
    let workspace_id = create_json
        .pointer("/result/workspace/workspace_id")
        .and_then(|v| v.as_str())
        .ok_or("workspace create did not return workspace_id")?;
    let root_pane = create_json
        .pointer("/result/root_pane/pane_id")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    if let Some(first) = project.tabs.first() {
        let _ = run_herdr(["tab", "rename", &format!("{workspace_id}:t1"), &first.name]);
        if let Some(cmd) = &first.command {
            if !root_pane.is_empty() {
                let _ = run_herdr(["pane", "run", root_pane, cmd]);
            }
        }
    }
    for tab in project.tabs.iter().skip(1) {
        let json = herdr_json([
            "tab",
            "create",
            "--workspace",
            workspace_id,
            "--cwd",
            &cwd.display().to_string(),
            "--label",
            &tab.name,
            "--no-focus",
        ])?;
        if let Some(cmd) = &tab.command {
            if let Some(pane) = json
                .pointer("/result/root_pane/pane_id")
                .and_then(|v| v.as_str())
            {
                let _ = run_herdr(["pane", "run", pane, cmd]);
            }
        }
    }
    Ok(())
}
