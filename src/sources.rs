use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
    process::Command,
    sync::OnceLock,
};

use serde_json::Value;

use crate::{
    config::Config,
    herdr::herdr_json,
    model::{Entry, EntryAction, Source, WorkspaceRef},
    navigator_state::NavigatorSnapshot,
    paths::{basename, canonical_str, expand_path, home},
};

pub(crate) fn collect_workspaces(
    snapshot: &mut NavigatorSnapshot,
) -> (Vec<Entry>, HashMap<String, Vec<WorkspaceRef>>, bool, bool) {
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
    workspaces_from_json(&ws_json, &cwd_by_ws, snapshot)
}

fn workspaces_from_json(
    ws_json: &Value,
    cwd_by_ws: &HashMap<String, String>,
    snapshot: &mut NavigatorSnapshot,
) -> (Vec<Entry>, HashMap<String, Vec<WorkspaceRef>>, bool, bool) {
    let has_live_workspace_list = ws_json
        .pointer("/result/workspaces")
        .and_then(|value| value.as_array())
        .is_some();
    let mut entries = Vec::new();
    let mut map = HashMap::new();
    let mut snapshot_changed = false;
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
            let tab_count = w.get("tab_count").and_then(|v| v.as_i64()).unwrap_or(0);
            let pane_count = w.get("pane_count").and_then(|v| v.as_i64()).unwrap_or(0);
            let agent_status = w
                .get("agent_status")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown");
            let focused = w.get("focused").and_then(|v| v.as_bool()).unwrap_or(false);
            if !id.is_empty() {
                snapshot_changed |= snapshot.migrate_legacy_kind(id, label);
            }
            if let Some(key) = canonical_str(&path) {
                map.entry(key).or_insert_with(Vec::new).push(WorkspaceRef {
                    id: id.into(),
                    label: label.into(),
                    kind: snapshot.workspace_kind(id, label),
                    path: path.clone(),
                    tab_count,
                    pane_count,
                });
            }
            let subtitle = format!(
                "agent:{agent_status} · {} tabs:{} panes:{}",
                id, tab_count, pane_count
            );
            let mut search_terms = vec![id.into(), label.into(), agent_status.into()];
            if focused {
                search_terms.push("focused".into());
            }
            entries.push(Entry {
                source: Source::Workspace,
                title: label.into(),
                subtitle,
                path,
                workspace_id: Some(id.into()),
                workspace_label: Some(label.into()),
                agent_target: None,
                project: None,
                action: EntryAction::FocusWorkspace { id: id.into() },
                source_label: None,
                search_terms,
                canonical: OnceLock::new(),
            });
        }
    }
    (entries, map, snapshot_changed, has_live_workspace_list)
}

pub(crate) fn collect_agents(
    workspaces: &[Entry],
    aliases: &[crate::config::AgentAliasConfig],
) -> Vec<Entry> {
    let agent_json = herdr_json(["agent", "list"]).unwrap_or(Value::Null);
    let tab_json = herdr_json(["tab", "list"]).unwrap_or(Value::Null);
    agents_from_json(
        &agent_json,
        &tab_labels_from_json(&tab_json),
        workspaces,
        aliases,
    )
}

/// Tab labels as `tab_id → label`. Herdr reports an unnamed tab with its number
/// as the label, which adds nothing to a row, so those are skipped.
fn tab_labels_from_json(tab_json: &Value) -> HashMap<String, String> {
    let mut labels = HashMap::new();
    if let Some(tabs) = tab_json.pointer("/result/tabs").and_then(|v| v.as_array()) {
        for t in tabs {
            let Some(id) = t.get("tab_id").and_then(|v| v.as_str()) else {
                continue;
            };
            let label = t.get("label").and_then(|v| v.as_str()).unwrap_or("").trim();
            let number = t
                .get("number")
                .and_then(|v| v.as_i64())
                .map(|n| n.to_string());
            if label.is_empty() || number.as_deref() == Some(label) {
                continue;
            }
            labels.insert(id.to_string(), label.to_string());
        }
    }
    labels
}

fn agents_from_json(
    agent_json: &Value,
    tab_labels: &HashMap<String, String>,
    workspaces: &[Entry],
    aliases: &[crate::config::AgentAliasConfig],
) -> Vec<Entry> {
    let workspace_labels: HashMap<&str, &str> = workspaces
        .iter()
        .filter_map(|entry| Some((entry.workspace_id.as_deref()?, entry.title.as_str())))
        .collect();
    let mut entries = Vec::new();
    if let Some(agents) = agent_json
        .pointer("/result/agents")
        .and_then(|v| v.as_array())
    {
        for p in agents {
            let Some(agent) = p.get("agent").and_then(|v| v.as_str()) else {
                continue;
            };
            let pane = p.get("pane_id").and_then(|v| v.as_str()).unwrap_or("");
            let tab = p.get("tab_id").and_then(|v| v.as_str()).unwrap_or("");
            let term = p.get("terminal_id").and_then(|v| v.as_str()).unwrap_or("");
            // Herdr's `agent focus` accepts pane IDs, not terminal IDs.
            let target = pane;
            let cwd = p.get("cwd").and_then(|v| v.as_str()).unwrap_or("/");
            let foreground_cwd = p
                .get("foreground_cwd")
                .and_then(|v| v.as_str())
                .unwrap_or(cwd);
            let status = p
                .get("agent_status")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown");
            let workspace_id = p.get("workspace_id").and_then(|v| v.as_str()).unwrap_or("");
            let workspace_label = workspace_labels
                .get(workspace_id)
                .copied()
                .unwrap_or(workspace_id);
            let path = PathBuf::from(cwd);
            let dir = basename(&path);
            let alias_terms: Vec<String> = aliases
                .iter()
                .filter(|alias| alias.matches(agent, workspace_label, cwd))
                .map(|alias| alias.alias.clone())
                .collect();
            let tab_label = tab_labels.get(tab).map(String::as_str).unwrap_or("");
            // Coding agents keep rewriting the terminal title with the task they
            // are on, which makes it the liveliest search term a pane has.
            let terminal_title = p
                .get("terminal_title_stripped")
                .or_else(|| p.get("terminal_title"))
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .trim();
            // A tab label separates panes of one project better than its
            // directory: inside a monorepo every pane shares workspace and cwd.
            let title = if tab_label.is_empty() {
                format!("{agent} · {workspace_label} · {dir}")
            } else {
                format!("{agent} · {workspace_label} · {tab_label}")
            };
            let subtitle = format!("{status} · {pane} · {tab}");
            let mut search_terms = vec![
                agent.into(),
                status.into(),
                pane.into(),
                tab.into(),
                term.into(),
                workspace_id.into(),
                workspace_label.into(),
                dir,
                basename(&PathBuf::from(foreground_cwd)),
                foreground_cwd.into(),
            ];
            if let Some(session) = p.pointer("/agent_session/value").and_then(|v| v.as_str()) {
                search_terms.push(session.into());
            }
            if !tab_label.is_empty() {
                search_terms.push(tab_label.into());
            }
            if !terminal_title.is_empty() {
                search_terms.push(terminal_title.into());
            }
            search_terms.extend(alias_terms);
            entries.push(Entry {
                source: Source::Agent,
                title,
                subtitle,
                path,
                workspace_id: (!workspace_id.is_empty()).then(|| workspace_id.into()),
                workspace_label: Some(workspace_label.into()),
                agent_target: Some(target.into()),
                project: None,
                action: EntryAction::FocusAgent {
                    target: target.into(),
                },
                source_label: None,
                search_terms,
                canonical: OnceLock::new(),
            });
        }
    }
    entries
}

const AGENT_SPINNER: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

// Mirrors Herdr's workspace `state_dot` and agent `agent_icon` mappings.
pub(crate) fn status_icon_at(source: &Source, status: &str, tick: u32) -> &'static str {
    let workspace = *source == Source::Workspace;
    let status = status.to_lowercase();
    if status.contains("block")
        || status.contains("error")
        || status.contains("fail")
        || status.contains("attention")
        || status.contains("request")
        || status.contains("wait")
    {
        if workspace {
            "●"
        } else {
            "◉"
        }
    } else if status.contains("work") || status.contains("run") {
        if workspace {
            "●"
        } else {
            AGENT_SPINNER[tick as usize % AGENT_SPINNER.len()]
        }
    } else if status.contains("done") || status.contains("complete") {
        "●"
    } else if status.contains("idle") {
        if workspace {
            "○"
        } else {
            "✓"
        }
    } else if workspace {
        "·"
    } else {
        "○"
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
                workspace_label: None,
                agent_target: None,
                project: None,
                action: EntryAction::FocusOrCreateDir,
                source_label: None,
                search_terms: vec![],
                canonical: OnceLock::new(),
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
            workspace_label: None,
            agent_target: None,
            project: None,
            action: EntryAction::FocusOrCreateDir,
            source_label: None,
            search_terms: vec![],
            canonical: OnceLock::new(),
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

#[cfg(test)]
mod tests {
    use super::*;

    // ponytail: fixtures captured verbatim from `herdr workspace list` / `herdr agent list` on 0.7.3
    #[test]
    fn parses_herdr_workspace_and_agent_list_json() {
        let ws_json = serde_json::json!({"id":"cli:workspace:list","result":{"type":"workspace_list","workspaces":[
            {"active_tab_id":"w41:t1","agent_status":"unknown","focused":false,"label":"~","number":1,"pane_count":1,"tab_count":1,"workspace_id":"w41"},
            {"active_tab_id":"w43:t1","agent_status":"working","focused":true,"label":"dir: picker","number":3,"pane_count":1,"tab_count":1,"workspace_id":"w43"}]}});
        let (entries, _, _, _) =
            workspaces_from_json(&ws_json, &HashMap::new(), &mut NavigatorSnapshot::default());
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].subtitle, "agent:unknown · w41 tabs:1 panes:1");
        assert!(entries[0].search_terms.contains(&"unknown".to_string()));
        assert_eq!(entries[1].subtitle, "agent:working · w43 tabs:1 panes:1");
        assert!(entries[1].search_terms.contains(&"working".to_string()));
        assert!(entries[1].search_terms.contains(&"focused".to_string()));

        let agent_json = serde_json::json!({"id":"cli:agent:list","result":{"type":"agent_list","agents":[
            {"agent":"claude","agent_session":{"agent":"claude","kind":"id","source":"herdr:claude","value":"58f4-session"},
             "agent_status":"working","cwd":"/tmp","focused":true,"foreground_cwd":"/tmp","pane_id":"w43:p1",
             "revision":0,"tab_id":"w43:t1","terminal_id":"term_1","workspace_id":"w43"}]}});
        let tab_json = serde_json::json!({"id":"cli:tab:list","result":{"type":"tab_list","tabs":[
            {"agent_status":"working","focused":true,"label":"primary","number":1,"pane_count":1,"tab_id":"w43:t1","workspace_id":"w43"},
            {"agent_status":"idle","focused":false,"label":"2","number":2,"pane_count":1,"tab_id":"w43:t2","workspace_id":"w43"}]}});
        let tab_labels = tab_labels_from_json(&tab_json);
        assert_eq!(
            tab_labels.get("w43:t1").map(String::as_str),
            Some("primary")
        );
        assert!(!tab_labels.contains_key("w43:t2")); // unnamed tab: label equals its number

        let agents = agents_from_json(&agent_json, &tab_labels, &entries, &[]);
        assert_eq!(agents.len(), 1);
        assert_eq!(agents[0].agent_target.as_deref(), Some("w43:p1"));
        assert_eq!(agents[0].title, "claude · dir: picker · primary");
        assert!(agents[0].search_terms.contains(&"primary".to_string()));
        assert!(matches!(
            &agents[0].action,
            EntryAction::FocusAgent { target } if target == "w43:p1"
        ));
        assert!(agents[0].search_terms.contains(&"term_1".to_string()));
        assert!(agents[0].search_terms.contains(&"58f4-session".to_string()));
        assert!(agents[0].subtitle.starts_with("working"));
    }

    #[test]
    fn persisted_workspace_kind_survives_label_changes_and_legacy_labels_migrate() {
        let ws_json = serde_json::json!({"result":{"workspaces":[
            {"workspace_id":"w1","label":"renamed","tab_count":1,"pane_count":1},
            {"workspace_id":"w2","label":"project: legacy","tab_count":1,"pane_count":1}
        ]}});
        let cwd_by_ws = HashMap::from([("w1".into(), "/tmp".into()), ("w2".into(), "/tmp".into())]);
        let mut snapshot = NavigatorSnapshot::default();
        snapshot.record("w1", crate::model::WorkspaceKind::Dir);

        let (_, workspaces, migrated, live) =
            workspaces_from_json(&ws_json, &cwd_by_ws, &mut snapshot);

        assert!(live);

        assert!(migrated);
        let workspaces = workspaces.get("/tmp").unwrap();
        assert!(matches!(
            workspaces[0].kind,
            crate::model::WorkspaceKind::Dir
        ));
        assert!(matches!(
            workspaces[1].kind,
            crate::model::WorkspaceKind::Project
        ));
        assert!(matches!(
            snapshot.workspace_kind("w2", "renamed"),
            crate::model::WorkspaceKind::Project
        ));
    }

    #[test]
    fn status_icons_match_herdr() {
        assert_eq!(status_icon_at(&Source::Workspace, "blocked", 0), "●");
        assert_eq!(status_icon_at(&Source::Workspace, "working", 0), "●");
        assert_eq!(status_icon_at(&Source::Workspace, "done", 0), "●");
        assert_eq!(status_icon_at(&Source::Workspace, "idle", 0), "○");
        assert_eq!(status_icon_at(&Source::Workspace, "unknown", 0), "·");

        assert_eq!(status_icon_at(&Source::Agent, "blocked", 0), "◉");
        assert_eq!(status_icon_at(&Source::Agent, "working", 0), "⠋");
        assert_eq!(status_icon_at(&Source::Agent, "working", 1), "⠙");
        assert_eq!(status_icon_at(&Source::Agent, "done", 0), "●");
        assert_eq!(status_icon_at(&Source::Agent, "idle", 0), "✓");
        assert_eq!(status_icon_at(&Source::Agent, "unknown", 0), "○");
    }
}
