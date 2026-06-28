use std::{
    collections::HashMap,
    env, fs,
    path::{Path, PathBuf},
    process::Command,
};

use serde_json::Value;

use crate::{
    config::Config,
    herdr::{herdr_json, run_herdr},
    model::{Entry, Project, Source},
    paths::{basename, canonical_str, expand_path, herdr_plus_projects_dir, home},
};

pub(crate) fn collect_workspaces() -> (Vec<Entry>, HashMap<String, String>) {
    let ws_json = herdr_json(["workspace", "list"]).unwrap_or(Value::Null);
    let pane_json = herdr_json(["pane", "list"]).unwrap_or(Value::Null);
    let mut cwd_by_ws: HashMap<String, String> = HashMap::new();
    if let Some(panes) = pane_json
        .pointer("/result/panes")
        .and_then(|v| v.as_array())
    {
        for p in panes {
            let Some(ws) = p.get("workspace_id").and_then(|v| v.as_str()) else {
                continue;
            };
            let cwd = p
                .get("foreground_cwd")
                .or_else(|| p.get("cwd"))
                .and_then(|v| v.as_str())
                .unwrap_or("");
            if !cwd.is_empty() {
                cwd_by_ws.entry(ws.into()).or_insert(cwd.into());
            }
        }
    }
    let mut entries = Vec::new();
    let mut map = HashMap::new();
    if let Some(workspaces) = ws_json
        .pointer("/result/workspaces")
        .and_then(|v| v.as_array())
    {
        for w in workspaces {
            let id = w.get("workspace_id").and_then(|v| v.as_str()).unwrap_or("");
            let label = w.get("label").and_then(|v| v.as_str()).unwrap_or(id);
            let cwd = cwd_by_ws
                .get(id)
                .cloned()
                .unwrap_or_else(|| home().display().to_string());
            let path = PathBuf::from(&cwd);
            if let Some(key) = canonical_str(&path) {
                map.insert(key, id.into());
            }
            entries.push(Entry {
                source: Source::Workspace,
                title: label.into(),
                subtitle: format!(
                    "{} tabs:{} panes:{}",
                    id,
                    w.get("tab_count").and_then(|v| v.as_i64()).unwrap_or(0),
                    w.get("pane_count").and_then(|v| v.as_i64()).unwrap_or(0)
                ),
                path,
                workspace_id: Some(id.into()),
                agent_target: None,
                project: None,
            });
        }
    }
    (entries, map)
}

pub(crate) fn collect_agents() -> Vec<Entry> {
    let pane_json = herdr_json(["pane", "list"]).unwrap_or(Value::Null);
    let mut entries = Vec::new();
    if let Some(panes) = pane_json
        .pointer("/result/panes")
        .and_then(|v| v.as_array())
    {
        for p in panes {
            let Some(agent) = p.get("agent").and_then(|v| v.as_str()) else {
                continue;
            };
            let pane = p.get("pane_id").and_then(|v| v.as_str()).unwrap_or("");
            let term = p
                .get("terminal_id")
                .and_then(|v| v.as_str())
                .unwrap_or(pane);
            let cwd = p
                .get("foreground_cwd")
                .or_else(|| p.get("cwd"))
                .and_then(|v| v.as_str())
                .unwrap_or("/");
            let status = p
                .get("agent_status")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown");
            entries.push(Entry {
                source: Source::Agent,
                title: agent.into(),
                subtitle: format!("{status} {pane}"),
                path: PathBuf::from(cwd),
                workspace_id: p
                    .get("workspace_id")
                    .and_then(|v| v.as_str())
                    .map(|s| s.into()),
                agent_target: Some(term.into()),
                project: None,
            });
        }
    }
    entries
}

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
    }
}

pub(crate) fn collect_zoxide() -> Vec<Entry> {
    let Ok(out) = Command::new("zoxide").args(["query", "-l"]).output() else {
        return vec![];
    };
    if !out.status.success() {
        return vec![];
    }
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|line| {
            let path = PathBuf::from(line);
            Entry {
                source: Source::Zoxide,
                title: basename(&path),
                subtitle: line.into(),
                path,
                workspace_id: None,
                agent_target: None,
                project: None,
            }
        })
        .collect()
}

pub(crate) fn collect_roots(config: &Config) -> Vec<Entry> {
    let mut out = Vec::new();
    for root in &config.roots {
        walk_dirs(&expand_path(&root.path), root.max_depth, &mut out);
    }
    out
}
fn walk_dirs(path: &Path, depth: usize, out: &mut Vec<Entry>) {
    if depth == 0 || !path.is_dir() {
        return;
    }
    if path.join(".git").exists()
        || path.join("package.json").exists()
        || path.join("Cargo.toml").exists()
    {
        out.push(Entry {
            source: Source::Root,
            title: basename(path),
            subtitle: path.display().to_string(),
            path: path.to_path_buf(),
            workspace_id: None,
            agent_target: None,
            project: None,
        });
    }
    if let Ok(read) = fs::read_dir(path) {
        for e in read.flatten() {
            let p = e.path();
            if p.is_dir() && !basename(&p).starts_with('.') {
                walk_dirs(&p, depth - 1, out);
            }
        }
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
