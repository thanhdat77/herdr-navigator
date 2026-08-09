use std::{
    process::Command,
    sync::mpsc::{self, Receiver},
    thread,
};

use crate::herdr::run_herdr;

const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");
const PLUGIN_SOURCE: &str = "thanhdat77/herdr-navigator";
const RELEASE_REPO: &str = "https://github.com/thanhdat77/herdr-navigator.git";

pub(crate) fn check_in_background() -> Receiver<Option<String>> {
    let (sender, receiver) = mpsc::channel();
    thread::spawn(move || {
        let _ = sender.send(check_for_update());
    });
    receiver
}

fn check_for_update() -> Option<String> {
    let latest = fetch_latest_release()?;
    newer_version(CURRENT_VERSION, &latest)
}

pub(crate) fn install() -> Result<String, String> {
    let version = fetch_latest_release()
        .ok_or_else(|| "could not determine the latest Herdr Navigator release".to_string())?;
    let release =
        release_ref(&version).ok_or_else(|| format!("invalid release version: {version}"))?;
    run_herdr([
        "plugin",
        "install",
        PLUGIN_SOURCE,
        "--ref",
        &release,
        "--yes",
    ])?;
    Ok(version)
}

fn release_ref(version: &str) -> Option<String> {
    let [major, minor, patch] = parse_version(version)?;
    Some(format!("v{major}.{minor}.{patch}"))
}

fn fetch_latest_release() -> Option<String> {
    let output = Command::new("git")
        .args([
            "-c",
            "http.lowSpeedLimit=1",
            "-c",
            "http.lowSpeedTime=5",
            "ls-remote",
            "--tags",
            "--refs",
            RELEASE_REPO,
            "v*",
        ])
        .env("GIT_TERMINAL_PROMPT", "0")
        .output()
        .ok()?;
    output
        .status
        .success()
        .then(|| String::from_utf8(output.stdout).ok())
        .flatten()
        .and_then(|tags| latest_release(&tags))
}

fn parse_version(value: &str) -> Option<[u64; 3]> {
    let mut parts = value.trim().trim_start_matches('v').split('.');
    let version = [
        parts.next()?.parse().ok()?,
        parts.next()?.parse().ok()?,
        parts.next()?.parse().ok()?,
    ];
    parts.next().is_none().then_some(version)
}

fn latest_release(tags: &str) -> Option<String> {
    let latest = tags
        .lines()
        .filter_map(|line| line.split_whitespace().nth(1))
        .filter_map(|reference| reference.strip_prefix("refs/tags/"))
        .filter_map(parse_version)
        .max()?;
    Some(format!("{}.{}.{}", latest[0], latest[1], latest[2]))
}

fn newer_version(current: &str, latest: &str) -> Option<String> {
    (parse_version(latest)? > parse_version(current)?).then(|| latest.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn newer_release_uses_latest_stable_semver_tag() {
        let tags = "a refs/tags/v0.3.0\nb refs/tags/v0.4.0-rc.1\nc refs/tags/v0.3.2\nd refs/tags/not-a-version\n";

        let latest = latest_release(tags).unwrap();
        assert_eq!(latest, "0.3.2");
        assert_eq!(newer_version("0.3.0", &latest), Some("0.3.2".into()));
        assert_eq!(newer_version("0.3.2", &latest), None);
    }

    #[test]
    fn release_ref_accepts_stable_versions_only() {
        assert_eq!(release_ref("0.3.3"), Some("v0.3.3".into()));
        assert_eq!(release_ref("0.3.3; rm -rf /"), None);
    }
}
