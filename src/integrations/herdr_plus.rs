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
            workspace_label: None,
            agent_target: None,
            project: Some(project),
            action: EntryAction::OpenProject,
            source_label: None,
            search_terms: vec![],
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
        workspace_label: None,
        agent_target: None,
        project: None,
        action: EntryAction::InvokePluginAction {
            action: "cloudmanic.herdr-plus.quick-actions".into(),
        },
        source_label: None,
        search_terms: vec![],
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
    if project.tabs.is_empty() {
        return Ok(());
    }
    let root_pane = create_json
        .pointer("/result/root_pane/pane_id")
        .and_then(|v| v.as_str())
        .ok_or("workspace create did not return root pane")?;
    let mut runs = Vec::new();

    for (tab_index, tab) in project.tabs.iter().enumerate() {
        let tab_root = if tab_index == 0 {
            let _ = run_herdr(["tab", "rename", &format!("{workspace_id}:t1"), &tab.name]);
            root_pane.to_string()
        } else {
            herdr_json([
                "tab",
                "create",
                "--workspace",
                workspace_id,
                "--cwd",
                &cwd.display().to_string(),
                "--label",
                &tab.name,
                "--no-focus",
            ])?
            .pointer("/result/root_pane/pane_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| format!("tab create did not return root pane for {}", tab.name))?
            .to_string()
        };

        let mut previous_pane = tab_root.clone();
        for (pane_index, pane) in tab.effective_panes().iter().enumerate() {
            let pane_id = if pane_index == 0 {
                tab_root.clone()
            } else {
                let direction = pane.split.as_deref().unwrap_or("down");
                herdr_json([
                    "pane",
                    "split",
                    &previous_pane,
                    "--direction",
                    direction,
                    "--no-focus",
                ])?
                .pointer("/result/pane/pane_id")
                .and_then(|v| v.as_str())
                .ok_or_else(|| {
                    format!(
                        "pane split did not return pane {} for {}",
                        pane_index + 1,
                        tab.name
                    )
                })?
                .to_string()
            };
            if let Some(label) = pane
                .label
                .as_deref()
                .filter(|label| !label.trim().is_empty())
            {
                let _ = run_herdr(["pane", "rename", &pane_id, label.trim()]);
            }
            if let Some(command) = pane
                .command
                .as_deref()
                .filter(|command| !command.trim().is_empty())
            {
                runs.push((pane_id.clone(), command.to_string()));
            }
            previous_pane = pane_id;
        }
    }

    for (pane, command) in runs {
        let _ = run_herdr(["pane", "run", &pane, &command]);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_split_pane_project_tabs() {
        let project: Project = toml::from_str(
            r#"
name = "demo"
working_dir = "~/"

[[tabs]]
name = "server"

[[tabs.panes]]
label = "Server"
command = "serve"

[[tabs.panes]]
label = "Terminal"
split = "right"
"#,
        )
        .unwrap();

        let panes = &project.tabs[0].panes;
        assert_eq!(panes.len(), 2);
        assert_eq!(panes[0].label.as_deref(), Some("Server"));
        assert_eq!(panes[0].command.as_deref(), Some("serve"));
        assert_eq!(panes[1].label.as_deref(), Some("Terminal"));
        assert_eq!(panes[1].split.as_deref(), Some("right"));
    }

    #[cfg(unix)]
    #[test]
    fn bootstraps_split_panes_before_running_commands() {
        use std::{
            env,
            os::unix::fs::PermissionsExt,
            sync::{Mutex, OnceLock},
            time::{SystemTime, UNIX_EPOCH},
        };

        static ENV_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        let _lock = ENV_LOCK.get_or_init(|| Mutex::new(())).lock().unwrap();
        let dir = env::temp_dir().join(format!(
            "herdr-navigator-split-test-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&dir).unwrap();
        let log = dir.join("calls");
        let fake = dir.join("herdr");
        fs::write(
            &fake,
            format!(
                r#"#!/bin/sh
printf '%s\n' "$*" >> '{}'
case "$1 $2 $3" in
  "tab create --workspace") printf '%s\n' '{{"result":{{"root_pane":{{"pane_id":"w1:p2"}}}}}}' ;;
  "pane split w1:p2") printf '%s\n' '{{"result":{{"pane":{{"pane_id":"w1:p3"}}}}}}' ;;
  "pane split w1:p3") printf '%s\n' '{{"result":{{"pane":{{"pane_id":"w1:p4"}}}}}}' ;;
esac
"#,
                log.display()
            ),
        )
        .unwrap();
        fs::set_permissions(&fake, fs::Permissions::from_mode(0o755)).unwrap();

        let project: Project = toml::from_str(
            r#"
name = "demo"
working_dir = "/tmp/project"

[[tabs]]
name = "agent"
command = "claude"

[[tabs]]
name = "server"

[[tabs.panes]]
label = "Server"
command = "serve"

[[tabs.panes]]
label = "Terminal"
split = "right"

[[tabs.panes]]
label = "Logs"
command = "tail"
"#,
        )
        .unwrap();
        let create_json = serde_json::json!({
            "result": {
                "workspace": { "workspace_id": "w1" },
                "root_pane": { "pane_id": "w1:p1" }
            }
        });
        let previous_bin = env::var_os("HERDR_BIN_PATH");
        env::set_var("HERDR_BIN_PATH", &fake);
        let result = bootstrap_project_tabs(&project, &create_json, Path::new("/tmp/project"));
        match previous_bin {
            Some(path) => env::set_var("HERDR_BIN_PATH", path),
            None => env::remove_var("HERDR_BIN_PATH"),
        }
        let calls = fs::read_to_string(&log).unwrap();
        fs::remove_dir_all(&dir).unwrap();

        result.unwrap();
        assert_eq!(
            calls.lines().collect::<Vec<_>>(),
            vec![
                "tab rename w1:t1 agent",
                "tab create --workspace w1 --cwd /tmp/project --label server --no-focus",
                "pane rename w1:p2 Server",
                "pane split w1:p2 --direction right --no-focus",
                "pane rename w1:p3 Terminal",
                "pane split w1:p3 --direction down --no-focus",
                "pane rename w1:p4 Logs",
                "pane run w1:p1 claude",
                "pane run w1:p2 serve",
                "pane run w1:p4 tail",
            ]
        );
    }
}
