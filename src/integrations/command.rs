use std::process::Command;

use serde::Deserialize;

use crate::{
    config::IntegrationConfig,
    model::{Entry, EntryAction, Source},
    paths::{expand_path, home},
};

#[derive(Debug, Deserialize)]
struct IntegrationItem {
    id: String,
    title: String,
    #[serde(default)]
    subtitle: String,
    path: Option<String>,
    #[serde(default)]
    kind: String,
}

pub(crate) fn collect(integrations: &[IntegrationConfig]) -> Vec<Entry> {
    integrations
        .iter()
        .filter(|integration| integration.enabled)
        .flat_map(collect_one)
        .collect()
}

fn collect_one(integration: &IntegrationConfig) -> Vec<Entry> {
    if integration.id.trim().is_empty() {
        return vec![];
    }
    let Ok(output) = Command::new("sh")
        .arg("-c")
        .arg(&integration.collect)
        .output()
    else {
        return vec![];
    };
    if !output.status.success() {
        return vec![];
    }
    parse_items(&output.stdout)
        .unwrap_or_default()
        .into_iter()
        .map(|item| entry_from_item(integration, item))
        .collect()
}

fn parse_items(bytes: &[u8]) -> Result<Vec<IntegrationItem>, serde_json::Error> {
    serde_json::from_slice(bytes)
}

fn entry_from_item(integration: &IntegrationConfig, item: IntegrationItem) -> Entry {
    let path = item.path.as_deref().map(expand_path).unwrap_or_else(home);
    let subtitle = subtitle(integration, &item);
    let command = render_template(&integration.open, &item);
    Entry {
        source: Source::Integration,
        title: item.title,
        subtitle,
        path,
        workspace_id: None,
        agent_target: None,
        project: None,
        action: EntryAction::RunCommand {
            command,
            notify_success: integration.notify_success,
            notify_error: integration.notify_error,
        },
        source_label: Some(integration.label.clone()),
    }
}

fn subtitle(integration: &IntegrationConfig, item: &IntegrationItem) -> String {
    match (item.kind.is_empty(), item.subtitle.is_empty()) {
        (true, true) => integration.label.clone(),
        (true, false) => format!("{} · {}", integration.label, item.subtitle),
        (false, true) => format!("{} · {}", integration.label, item.kind),
        (false, false) => format!("{} · {} · {}", integration.label, item.kind, item.subtitle),
    }
}

fn render_template(template: &str, item: &IntegrationItem) -> String {
    let path = item.path.as_deref().unwrap_or("");
    template
        .replace("{{id}}", &shell_quote(&item.id))
        .replace("{{title}}", &shell_quote(&item.title))
        .replace("{{subtitle}}", &shell_quote(&item.subtitle))
        .replace("{{path}}", &shell_quote(path))
        .replace("{{kind}}", &shell_quote(&item.kind))
}

fn shell_quote(value: &str) -> String {
    if value.is_empty() {
        "''".into()
    } else {
        format!("'{}'", value.replace('\'', "'\\''"))
    }
}

pub(crate) fn run_command(command: &str) -> Result<(), String> {
    let status = Command::new("sh")
        .arg("-c")
        .arg(command)
        .status()
        .map_err(|e| e.to_string())?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("command exited with {status}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config() -> IntegrationConfig {
        IntegrationConfig {
            id: "demo".into(),
            label: "Demo".into(),
            enabled: true,
            collect: "demo list --json".into(),
            open: "demo open {{id}} --path {{path}}".into(),
            notify_success: true,
            notify_error: true,
        }
    }

    #[test]
    fn parses_collect_json() {
        let items = parse_items(
            br#"[{"id":"abc","title":"Item","subtitle":"Info","path":"/tmp","kind":"action"}]"#,
        )
        .unwrap();

        assert_eq!(items.len(), 1);
        assert_eq!(items[0].id, "abc");
        assert_eq!(items[0].title, "Item");
    }

    #[test]
    fn renders_shell_safe_open_command() {
        let item = IntegrationItem {
            id: "a b".into(),
            title: "It's fine".into(),
            subtitle: String::new(),
            path: Some("/tmp/a b".into()),
            kind: "action".into(),
        };

        assert_eq!(
            render_template("demo open {{id}} --title {{title}} --path {{path}}", &item),
            "demo open 'a b' --title 'It'\\''s fine' --path '/tmp/a b'"
        );
    }

    #[test]
    fn failed_collect_is_optional() {
        let mut cfg = config();
        cfg.collect = "exit 7".into();

        assert!(collect(&[cfg]).is_empty());
    }

    #[test]
    fn builds_entry_with_run_command_action() {
        let cfg = config();
        let item = IntegrationItem {
            id: "abc".into(),
            title: "Item".into(),
            subtitle: "Info".into(),
            path: Some("/tmp".into()),
            kind: "action".into(),
        };
        let entry = entry_from_item(&cfg, item);

        assert_eq!(entry.source, Source::Integration);
        assert_eq!(entry.title, "Item");
        assert!(matches!(entry.action, EntryAction::RunCommand { .. }));
        assert_eq!(entry.path, std::path::PathBuf::from("/tmp"));
    }
}
