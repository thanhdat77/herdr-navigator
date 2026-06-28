# Herdr Picker Plus

A Herdr-native overlay picker for jumping to workspaces, projects, directories, agents, and Herdr Plus actions.

```text
prefix+t -> search -> Enter
```

## What it does

Herdr's built-in `prefix+g` is great for navigating things that already exist. Herdr Picker Plus is for the `sesh` / `Ctrl-T` workflow: start from a project or directory, then land in the right Herdr workspace.

It can:

- focus an already-open workspace
- open a Herdr Plus project template
- create a new workspace from zoxide or configured roots
- focus an agent pane
- launch Herdr Plus Quick Actions

## Features

- Herdr plugin action + Herdr-managed overlay pane
- Rust TUI built with `ratatui` and `crossterm`
- Preview panel for the selected entry
- Configurable matcher: `nucleo`, `skim`, or `simple`
- Configurable source priority order
- Herdr Plus Projects integration
- Herdr Plus Quick Actions integration
- Herdr theme-name inheritance with `[theme.custom]` overrides
- No external picker dependency (`fzf`, `tv`, etc.)

## Sources

| Source | Reads | Enter |
| --- | --- | --- |
| `workspace` | `herdr workspace list` + pane cwd | focus workspace |
| `project` | Herdr Plus `projects/*.toml` | focus existing cwd or create workspace + project tabs |
| `quick` | Herdr Plus Quick Actions | open Quick Actions picker |
| `zoxide` | `zoxide query -l` | focus existing cwd or create workspace |
| `root` | configured filesystem roots | focus existing cwd or create workspace |
| `agent` | agent panes from `herdr pane list` | focus agent pane |

## Keybindings

| Key | Action |
| --- | --- |
| type | fuzzy search |
| `Enter` | open selected item |
| `Esc` / `Ctrl-C` | close |
| `Up` / `Down` | move selection |
| `Tab` | cycle source filters |
| `Ctrl-W` | workspaces only |
| `Ctrl-P` | Herdr Plus projects only |
| `Ctrl-Q` | Herdr Plus Quick Actions only |
| `Ctrl-Z` | zoxide only |
| `Ctrl-R` | roots only |
| `Ctrl-A` | agents only |
| `Ctrl-O` | toggle preview |
| `Ctrl-U` | clear query and filter |

## Requirements

- Herdr `0.7.0` or newer
- Rust stable when building from source
- Optional: `zoxide` for the zoxide source
- Optional: Herdr Plus for `project` and `quick` sources

## Install

### From source

```bash
git clone <repo-url>
cd herdr-picker-plus
cargo build --release
herdr plugin link "$PWD"
```

### From release archive

Download the archive for your platform, extract it, then link the extracted directory:

```bash
herdr plugin link /path/to/herdr-picker-plus
```

Run once without a keybinding:

```bash
herdr plugin action invoke herdr-picker-plus.open
```

## Bind to `prefix+t`

Add this to `~/.config/herdr/config.toml`:

```toml
[[keys.command]]
key = "prefix+t"
type = "plugin_action"
command = "herdr-picker-plus.open"
description = "picker plus"
```

Reload Herdr:

```bash
herdr server reload-config
```

## Configuration

Find the plugin config directory:

```bash
herdr plugin config-dir herdr-picker-plus
```

On first run, the plugin creates `config.toml` from [`examples/default-config.toml`](examples/default-config.toml).

```toml
[picker]
reuse_existing = true
create_missing = true
engine = "nucleo" # nucleo | skim | simple
source_order = ["workspace", "project", "zoxide", "root", "agent", "quick"]
source_priority_boost = 25

[sources]
open_workspaces = true
herdr_plus_projects = true
herdr_plus_quick_actions = true
zoxide = true
roots = true
agents = true

[theme]
inherit_herdr = true

[[roots]]
path = "~/workspace"
max_depth = 3

[[roots]]
path = "~/projects"
max_depth = 3
```

### Source priority

`source_order` controls initial ordering and ranking bonuses. Accepted names:

```text
workspace, open, project, zoxide, root, agent, quick
```

Set the boost to zero for pure matcher score:

```toml
source_priority_boost = 0
```

### Theme

`inherit_herdr = true` reads `~/.config/herdr/config.toml`, maps supported theme names locally, then applies `[theme.custom]` overrides.

Supported built-in names:

```text
one-light, catppuccin, rose-pine, rose-pine-dawn, terminal
```

If Herdr config is unavailable, the picker falls back to One Light.

## Herdr Plus integration

When Herdr Plus is installed:

- `project` entries are loaded from `~/.config/herdr/plugins/config/cloudmanic.herdr-plus/projects/*.toml`
- selecting a project creates/focuses a workspace and applies the project's tabs
- `quick` opens the Herdr Plus Quick Actions picker

If Herdr Plus is not installed, those sources simply do not add useful entries.

## Troubleshooting

### `prefix+t` opens an old picker or selecting projects does nothing

Make sure Herdr is linked to the current plugin id and your keybinding uses `herdr-picker-plus.open`:

```bash
herdr plugin link "$PWD"
herdr server reload-config
herdr plugin action list --plugin herdr-picker-plus
```

Then verify `~/.config/herdr/config.toml` contains:

```toml
command = "herdr-picker-plus.open"
```

## Debugging

List all candidates without opening the TUI:

```bash
./target/release/herdr-picker-plus list
```

Show plugin actions:

```bash
herdr plugin action list --plugin herdr-picker-plus
```

Unlink local plugin:

```bash
herdr plugin unlink herdr-picker-plus
```

## Release

This project ships tagged GitHub releases with Linux and macOS archives. See [`RELEASE.md`](RELEASE.md).

## Security

See [`SECURITY.md`](SECURITY.md).

## Design notes

Herdr plugin v1 does not expose the active theme palette directly to external plugins. The picker reads Herdr config and maps supported theme names locally, with `[theme.custom]` overrides applied last.

Herdr plugin v1 also does not expose a native non-terminal custom UI API. This plugin follows the current Herdr-native pattern: an action opens a managed overlay pane, and the interactive TUI runs inside that pane.
