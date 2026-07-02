use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
    process::Command,
};

use serde_json::Value;

use crate::{
    config::Config,
    herdr::herdr_json,
    model::{Entry, EntryAction, Source, WorkspaceKind, WorkspaceRef},
    paths::{basename, canonical_str, expand_path, home},
};

pub(crate) fn collect_workspaces() -> (Vec<Entry>, HashMap<String, Vec<WorkspaceRef>>) {
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
            let tab_count = w.get("tab_count").and_then(|v| v.as_i64()).unwrap_or(0);
            let pane_count = w.get("pane_count").and_then(|v| v.as_i64()).unwrap_or(0);
            if let Some(key) = canonical_str(&path) {
                map.entry(key).or_insert_with(Vec::new).push(WorkspaceRef {
                    id: id.into(),
                    label: label.into(),
                    kind: workspace_kind(label),
                    path: path.clone(),
                    tab_count,
                    pane_count,
                });
            }
            entries.push(Entry {
                source: Source::Workspace,
                title: label.into(),
                subtitle: format!("{} tabs:{} panes:{}", id, tab_count, pane_count),
                path,
                workspace_id: Some(id.into()),
                workspace_label: Some(label.into()),
                agent_target: None,
                project: None,
                action: EntryAction::FocusWorkspace { id: id.into() },
                source_label: None,
                search_terms: vec![id.into(), label.into()],
            });
        }
    }
    (entries, map)
}

fn workspace_kind(label: &str) -> WorkspaceKind {
    let label = label.trim().to_ascii_lowercase();
    if label.starts_with("project:") {
        WorkspaceKind::Project
    } else if label.starts_with("dir:") {
        WorkspaceKind::Dir
    } else if label.starts_with("server:") {
        WorkspaceKind::Server
    } else {
        WorkspaceKind::Unknown
    }
}

pub(crate) fn collect_servers(config: &Config) -> Vec<Entry> {
    let mut entries = Vec::new();
    if config.servers.ssh_config {
        let path = home().join(".ssh/config");
        if let Ok(text) = fs::read_to_string(path) {
            entries.extend(ssh_config_hosts(&text).into_iter().map(|host| {
                server_entry(
                    &host.name,
                    host.hostname.as_deref(),
                    host.user.as_deref(),
                    Some(&host.name),
                    &[],
                    &config.servers.base_dir,
                )
            }));
        }
    }
    entries.extend(config.servers.entries.iter().map(|server| {
        server_entry(
            &server.name,
            server.host.as_deref(),
            server.user.as_deref(),
            server.target.as_deref(),
            &server.tags,
            &config.servers.base_dir,
        )
    }));
    entries
}

fn server_entry(
    name: &str,
    host: Option<&str>,
    user: Option<&str>,
    target_override: Option<&str>,
    tags: &[String],
    base_dir: &str,
) -> Entry {
    let target = target_override
        .map(str::to_string)
        .unwrap_or_else(|| match (user, host) {
            (Some(user), Some(host)) => format!("{user}@{host}"),
            (_, Some(host)) => host.to_string(),
            _ => name.to_string(),
        });
    let path = expand_path(base_dir);
    let subtitle = format!("autossh/ssh {target}");
    let mut search_terms = vec![name.into(), target.clone()];
    if let Some(host) = host {
        search_terms.push(host.into());
    }
    if let Some(user) = user {
        search_terms.push(user.into());
    }
    search_terms.extend(tags.iter().cloned());
    Entry {
        source: Source::Server,
        title: name.into(),
        subtitle,
        path,
        workspace_id: None,
        workspace_label: Some(format!("server: {name}")),
        agent_target: None,
        project: None,
        action: EntryAction::OpenServer { target },
        source_label: None,
        search_terms,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SshHost {
    name: String,
    hostname: Option<String>,
    user: Option<String>,
}

fn ssh_config_hosts(text: &str) -> Vec<SshHost> {
    let mut out = Vec::new();
    let mut names: Vec<String> = Vec::new();
    let mut hostname: Option<String> = None;
    let mut user: Option<String> = None;

    for line in text.lines().map(clean_ssh_config_line) {
        let Some((key, value)) = line.split_once(char::is_whitespace) else {
            continue;
        };
        let key = key.to_ascii_lowercase();
        let value = value.trim();
        if key == "host" {
            flush_ssh_hosts(&mut out, &names, hostname.take(), user.take());
            names = value
                .split_whitespace()
                .filter(|name| !name.contains(['*', '?', '!']))
                .map(str::to_string)
                .collect();
        } else if key == "hostname" {
            hostname = Some(value.into());
        } else if key == "user" {
            user = Some(value.into());
        }
    }
    flush_ssh_hosts(&mut out, &names, hostname, user);
    out
}

fn clean_ssh_config_line(line: &str) -> &str {
    line.split('#').next().unwrap_or("").trim()
}

fn flush_ssh_hosts(
    out: &mut Vec<SshHost>,
    names: &[String],
    hostname: Option<String>,
    user: Option<String>,
) {
    for name in names {
        out.push(SshHost {
            name: name.clone(),
            hostname: hostname.clone(),
            user: user.clone(),
        });
    }
}

pub(crate) fn collect_agents(
    workspaces: &[Entry],
    aliases: &[crate::config::AgentAliasConfig],
) -> Vec<Entry> {
    let workspace_labels: HashMap<&str, &str> = workspaces
        .iter()
        .filter_map(|entry| Some((entry.workspace_id.as_deref()?, entry.title.as_str())))
        .collect();
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
            let tab = p.get("tab_id").and_then(|v| v.as_str()).unwrap_or("");
            let term = p
                .get("terminal_id")
                .and_then(|v| v.as_str())
                .unwrap_or(pane);
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
            let title = format!("{} {agent} · {workspace_label} · {dir}", agent_status_icon(status));
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
            search_terms.extend(alias_terms);
            entries.push(Entry {
                source: Source::Agent,
                title,
                subtitle,
                path,
                workspace_id: (!workspace_id.is_empty()).then(|| workspace_id.into()),
                workspace_label: Some(workspace_label.into()),
                agent_target: Some(term.into()),
                project: None,
                action: EntryAction::FocusAgent {
                    target: term.into(),
                },
                source_label: None,
                search_terms,
            });
        }
    }
    entries
}

fn agent_status_icon(status: &str) -> &'static str {
    let status = status.to_lowercase();
    if status.contains("block") || status.contains("error") || status.contains("fail") {
        "!"
    } else if status.contains("done") || status.contains("complete") {
        "✓"
    } else if status.contains("work") || status.contains("run") {
        "●"
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

    #[test]
    fn agent_status_icons_are_plain_unicode() {
        assert_eq!(agent_status_icon("working"), "●");
        assert_eq!(agent_status_icon("done"), "✓");
        assert_eq!(agent_status_icon("blocked"), "!");
        assert_eq!(agent_status_icon("idle"), "○");
    }

    #[test]
    fn parses_simple_ssh_config_hosts() {
        let hosts = ssh_config_hosts(
            r#"
            Host prod-api prod-api-short
              HostName 10.0.0.5
              User ubuntu

            Host *
              User ignored

            Host staging-db
              HostName staging.internal
            "#,
        );

        assert_eq!(hosts.len(), 3);
        assert_eq!(hosts[0].name, "prod-api");
        assert_eq!(hosts[0].hostname.as_deref(), Some("10.0.0.5"));
        assert_eq!(hosts[0].user.as_deref(), Some("ubuntu"));
        assert_eq!(hosts[2].name, "staging-db");
        assert_eq!(hosts[2].hostname.as_deref(), Some("staging.internal"));
    }

    #[test]
    fn manual_server_entry_uses_remote_target_when_present() {
        let entry = server_entry(
            "logs-prod",
            Some("prod-api"),
            Some("ubuntu"),
            Some("prod-api"),
            &["logs".into(), "prod".into()],
            "~/workspace/server",
        );

        assert_eq!(entry.source, Source::Server);
        assert_eq!(entry.title, "logs-prod");
        assert_eq!(entry.path, home().join("workspace/server"));
        assert!(entry.haystack().contains("logs"));
        assert!(matches!(
            entry.action,
            EntryAction::OpenServer { ref target } if target == "prod-api"
        ));
    }
}
