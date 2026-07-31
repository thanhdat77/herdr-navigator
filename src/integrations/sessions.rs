use std::{env, path::PathBuf, process::Command, sync::OnceLock};

use serde::Deserialize;
use serde_json::Value;

use crate::{
    config::{Config, SessionEntryConfig},
    herdr::{herdr_bin, herdr_json},
    model::{Entry, EntryAction, Source},
    paths::home,
};

#[derive(Debug, Deserialize)]
struct SessionList {
    #[serde(default)]
    sessions: Vec<ListedSession>,
}

#[derive(Debug, Deserialize)]
struct ListedSession {
    name: String,
    #[serde(default)]
    running: bool,
    #[serde(default)]
    default: bool,
    session_dir: Option<String>,
}

pub(crate) fn collect_sessions(config: &Config) -> Vec<Entry> {
    let mut entries = Vec::new();
    if config.sessions.local {
        entries.extend(collect_local_sessions());
    }
    entries.extend(
        config
            .sessions
            .entries
            .iter()
            .filter(|entry| entry.remote.is_none())
            .map(manual_session_entry),
    );
    entries
}

pub(crate) fn collect_remotes(config: &Config) -> Vec<Entry> {
    config
        .sessions
        .entries
        .iter()
        .filter_map(remote_entry)
        .collect()
}

fn collect_local_sessions() -> Vec<Entry> {
    let json = herdr_json(["session", "list", "--json"]).unwrap_or(Value::Null);
    let list: SessionList = serde_json::from_value(json.clone())
        .or_else(|_| {
            serde_json::from_value(json.pointer("/result").cloned().unwrap_or(Value::Null))
        })
        .unwrap_or(SessionList { sessions: vec![] });
    list.sessions.into_iter().map(local_session_entry).collect()
}

fn local_session_entry(session: ListedSession) -> Entry {
    let mut flags = Vec::new();
    if session.default {
        flags.push("default");
    }
    if session.running {
        flags.push("running");
    }
    let subtitle = if flags.is_empty() {
        "local session".into()
    } else {
        format!("local session · {}", flags.join(" · "))
    };
    let path = session
        .session_dir
        .as_ref()
        .map(PathBuf::from)
        .unwrap_or_else(|| home().join(format!(".config/herdr/sessions/{}", session.name)));
    Entry {
        source: Source::Session,
        title: session.name.clone(),
        subtitle,
        path,
        workspace_id: None,
        workspace_label: None,
        agent_target: None,
        project: None,
        action: EntryAction::AttachSession {
            name: session.name,
            remote: None,
        },
        source_label: None,
        search_terms: vec!["local".into(), "session".into()],
        canonical: OnceLock::new(),
    }
}

fn manual_session_entry(config: &SessionEntryConfig) -> Entry {
    let session = config.session.as_deref().unwrap_or(&config.name);
    let path = home().join(format!(".config/herdr/sessions/{session}"));
    let mut search_terms = vec!["session".into(), session.into()];
    search_terms.extend(config.tags.iter().cloned());
    Entry {
        source: Source::Session,
        title: config.name.clone(),
        subtitle: format!("local session · {session}"),
        path,
        workspace_id: None,
        workspace_label: None,
        agent_target: None,
        project: None,
        action: EntryAction::AttachSession {
            name: session.into(),
            remote: None,
        },
        source_label: None,
        search_terms,
        canonical: OnceLock::new(),
    }
}

fn remote_entry(config: &SessionEntryConfig) -> Option<Entry> {
    let target = config.remote.clone()?;
    let mut search_terms = vec!["server".into(), "remote".into(), target.clone()];
    search_terms.extend(config.tags.iter().cloned());
    Some(Entry {
        source: Source::Server,
        title: config.name.clone(),
        subtitle: format!("remote Herdr · {target}"),
        path: PathBuf::from(format!("remote:{target}")),
        workspace_id: None,
        workspace_label: None,
        agent_target: None,
        project: None,
        action: EntryAction::OpenRemote { target },
        source_label: None,
        search_terms,
        canonical: OnceLock::new(),
    })
}

pub(crate) fn attach_session(name: &str) -> Result<(), String> {
    let status = herdr_attach_command()
        .args(["session", "attach", name])
        .status()
        .map_err(|err| err.to_string())?;

    if status.success() {
        Ok(())
    } else {
        Err(format!("herdr exited with {status}"))
    }
}

pub(crate) fn open_remote(target: &str) -> Result<(), String> {
    let status = Command::new(herdr_bin())
        .args(["--remote", target, "--handoff"])
        .status()
        .map_err(|err| err.to_string())?;

    if status.success() {
        Ok(())
    } else {
        Err(format!("herdr exited with {status}"))
    }
}

fn herdr_attach_command() -> Command {
    let mut command = Command::new(herdr_bin());
    for key in env::vars()
        .map(|(key, _)| key)
        .filter(|key| key.starts_with("HERDR_"))
    {
        command.env_remove(key);
    }
    command
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_session_entry_attaches_by_name() {
        let entry = local_session_entry(ListedSession {
            name: "work".into(),
            running: true,
            default: false,
            session_dir: Some("/tmp/herdr-work".into()),
        });

        assert_eq!(entry.source, Source::Session);
        assert!(entry.subtitle.contains("running"));
        assert!(matches!(
            entry.action,
            EntryAction::AttachSession { ref name, remote: None } if name == "work"
        ));
    }

    #[test]
    fn manual_remote_entry_opens_remote_target() {
        let entry = remote_entry(&SessionEntryConfig {
            name: "prod".into(),
            remote: Some("prod-box".into()),
            session: Some("default".into()),
            tags: vec!["api".into()],
        })
        .unwrap();

        assert_eq!(entry.source, Source::Server);
        assert!(entry.haystack().contains("prod-box"));
        assert!(matches!(
            entry.action,
            EntryAction::OpenRemote { ref target } if target == "prod-box"
        ));
    }
}
