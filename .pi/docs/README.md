# Agent Docs: Herdr Navigator

Purpose: compact project-only docs for future agents. Prefer intent over line-by-line code notes.

## Project intent

Herdr Navigator is a Herdr picker-center plugin for one fast command-palette flow:

```text
prefix+t -> search -> Enter -> land in the right place
```

It should unify:
- open Herdr workspaces
- Herdr Plus project templates
- Herdr Plus Quick Actions
- zoxide directories
- configured root scans
- agent panes

Core UX: if something is already open, focus it; otherwise create/open the smallest useful Herdr workspace.

## Current public identity

- Cargo package / binary: `herdr-navigator`
- Herdr plugin id: `herdr-navigator`
- Main action: `herdr-navigator.open`
- Overlay pane id: `picker`
- Plugin manifest: `herdr-plugin.toml`
- Main code: `src/main.rs`
- Default config template: `examples/default-config.toml`

Avoid reintroducing old names or personal prefixes.

## Fast checks

```bash
cargo fmt --check
cargo clippy -- -D warnings
cargo test
cargo build --release
./target/release/herdr-navigator list
```

## Docs map

- `architecture.md`: runtime flow and data model
- `features.md`: current feature behavior and UX intent
- `decisions.md`: durable decisions; do not casually reverse
- `bugs-and-lessons.md`: bugs hit during development and fixes
- `release.md`: publish/release notes for agents
