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
                agent_kind: None,
                agent_task: None,
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
    agents_from_json(&agent_json, workspaces, aliases)
}

fn agents_from_json(
    agent_json: &Value,
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
            // A blank name must fall back to the kind; it would otherwise
            // leave the title leading with an empty ` · ` segment.
            let agent_name = p
                .get("name")
                .and_then(|v| v.as_str())
                .filter(|s| !s.trim().is_empty());
            let task_title = p
                .get("terminal_title_stripped")
                .and_then(|v| v.as_str())
                .filter(|s| !s.trim().is_empty());
            let alias_terms: Vec<String> = aliases
                .iter()
                .filter(|alias| alias.matches(agent, workspace_label, cwd))
                .map(|alias| alias.alias.clone())
                .collect();
            let display_agent = agent_name.unwrap_or(agent);
            let title = format!("{display_agent} · {workspace_label} · {dir}");
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
            if let Some(name) = agent_name {
                search_terms.push(name.into());
            }
            if let Some(task) = task_title {
                search_terms.push(task.into());
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
                agent_kind: Some(agent.into()),
                agent_task: task_title.map(String::from),
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
                agent_kind: None,
                agent_task: None,
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
            agent_kind: None,
            agent_task: None,
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
             "agent_status":"working","cwd":"/tmp","focused":true,"foreground_cwd":"/tmp","name":"reviewer","pane_id":"w43:p1",
             "revision":0,"tab_id":"w43:t1","terminal_id":"term_1",
             "terminal_title_stripped":"◐ Fix buildSrc consumer surface","workspace_id":"w43"}]}});
        let agents = agents_from_json(&agent_json, &entries, &[]);
        assert_eq!(agents.len(), 1);
        assert_eq!(agents[0].agent_target.as_deref(), Some("w43:p1"));
        assert!(matches!(
            &agents[0].action,
            EntryAction::FocusAgent { target } if target == "w43:p1"
        ));
        assert!(agents[0].search_terms.contains(&"term_1".to_string()));
        assert!(agents[0].search_terms.contains(&"58f4-session".to_string()));
        assert!(agents[0].search_terms.contains(&"reviewer".to_string()));
        // "stripped" means ANSI-stripped, not glyph-stripped.
        assert!(agents[0]
            .search_terms
            .contains(&"◐ Fix buildSrc consumer surface".to_string()));
        assert_eq!(
            agents[0].agent_task.as_deref(),
            Some("◐ Fix buildSrc consumer surface")
        );
        assert_eq!(agents[0].agent_kind.as_deref(), Some("claude"));
        assert!(agents[0].title.starts_with("reviewer · "));
        assert!(agents[0].subtitle.starts_with("working"));
    }

    // Herdr omits `name` for unnamed agents; the kind has to carry the title.
    #[test]
    fn unnamed_and_blank_named_agents_fall_back_to_the_agent_kind() {
        let agent_json = serde_json::json!({"id":"cli:agent:list","result":{"type":"agent_list","agents":[
            {"agent":"opencode","agent_status":"idle","cwd":"/tmp","pane_id":"w1:p1","tab_id":"w1:t1",
             "terminal_id":"term_1","workspace_id":"w1"},
            {"agent":"codex","agent_status":"idle","cwd":"/tmp","name":"   ","pane_id":"w2:p1","tab_id":"w2:t1",
             "terminal_id":"term_2","terminal_title_stripped":"  ","workspace_id":"w2"}]}});
        let agents = agents_from_json(&agent_json, &[], &[]);

        assert_eq!(agents.len(), 2);
        assert!(agents[0].title.starts_with("opencode · "));
        assert!(agents[1].title.starts_with("codex · "));
        assert_eq!(agents[1].agent_task, None);
        assert!(!agents[1].search_terms.iter().any(|t| t.trim().is_empty()));
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
