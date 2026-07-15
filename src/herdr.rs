use std::{
    env,
    process::{Command, Stdio},
    thread,
};

use serde_json::Value;

use crate::{config::NotificationsConfig, paths::expand_path};

pub(crate) fn herdr_bin() -> String {
    env::var("HERDR_BIN_PATH").unwrap_or_else(|_| "herdr".into())
}
pub(crate) fn herdr_json<const N: usize>(args: [&str; N]) -> Result<Value, String> {
    let out = Command::new(herdr_bin())
        .args(args)
        .output()
        .map_err(|e| e.to_string())?;
    if !out.status.success() {
        return Err(String::from_utf8_lossy(&out.stderr).to_string());
    }
    serde_json::from_slice(&out.stdout).map_err(|e| e.to_string())
}
pub(crate) fn run_herdr<const N: usize>(args: [&str; N]) -> Result<(), String> {
    let status = Command::new(herdr_bin())
        .args(args)
        .status()
        .map_err(|e| e.to_string())?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("herdr exited with {status}"))
    }
}

pub(crate) fn notify_done(body: &str, config: &NotificationsConfig) {
    notify(body, "done", config);
}

pub(crate) fn notify_error(body: &str, config: &NotificationsConfig) {
    notify(body, "request", config);
}

fn notify(body: &str, default_sound: &'static str, config: &NotificationsConfig) {
    if !config.enabled {
        return;
    }
    let body = truncate(body, 180);
    let (sound, custom_sound) = notification_audio(config, default_sound);
    let _ = Command::new(herdr_bin())
        .args([
            "notification",
            "show",
            "Herdr Navigator",
            "--body",
            &body,
            "--position",
            "top-right",
            "--sound",
            sound,
        ])
        .status();
    if let Some(path) = custom_sound {
        play_custom_sound(path);
    }
}

fn notification_audio<'a>(
    config: &'a NotificationsConfig,
    default_sound: &'static str,
) -> (&'static str, Option<&'a str>) {
    match config.sound.trim().to_ascii_lowercase().as_str() {
        "default" => (default_sound, None),
        "custom" => (
            "none",
            config
                .custom_sound
                .as_deref()
                .filter(|path| !path.is_empty()),
        ),
        _ => ("none", None),
    }
}

fn play_custom_sound(path: &str) {
    let path = expand_path(path);
    if !path.is_file() {
        return;
    }
    thread::spawn(move || {
        #[cfg(target_os = "macos")]
        let players: &[(&str, &[&str])] = &[("afplay", &[])];
        #[cfg(not(target_os = "macos"))]
        let players: &[(&str, &[&str])] = &[("pw-play", &[]), ("paplay", &[]), ("aplay", &[])];

        for (player, args) in players {
            let status = Command::new(player)
                .args(*args)
                .arg(&path)
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status();
            if status.is_ok_and(|status| status.success()) {
                break;
            }
        }
    });
}

fn truncate(value: &str, max_chars: usize) -> String {
    let mut out: String = value.chars().take(max_chars).collect();
    if value.chars().count() > max_chars {
        out.push('…');
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::NotificationsConfig;

    #[test]
    fn notification_audio_resolves_default_none_and_custom() {
        let default = NotificationsConfig::default();
        assert_eq!(notification_audio(&default, "done"), ("done", None));

        let none = NotificationsConfig {
            enabled: true,
            sound: "none".into(),
            custom_sound: None,
        };
        assert_eq!(notification_audio(&none, "request"), ("none", None));

        let custom = NotificationsConfig {
            enabled: true,
            sound: "custom".into(),
            custom_sound: Some("~/sounds/navigator.wav".into()),
        };
        assert_eq!(
            notification_audio(&custom, "done"),
            ("none", Some("~/sounds/navigator.wav"))
        );
    }
}
