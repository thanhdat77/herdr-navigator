# Herdr Workdir Picker

A Herdr-native command palette for jumping to work directories.

It opens as a Herdr plugin overlay, searches across open workspaces, Herdr Plus projects, zoxide history, configured roots, and agents, then focuses an existing workspace or creates a new one.

```text
prefix+t -> search -> Enter -> focus existing workspace or create workspace
```

## Why this exists

Herdr's built-in `prefix+g` is excellent for navigating things that already exist. This plugin is for the `sesh`/`Ctrl-T` style workflow: find a project directory first, then land in the right Herdr workspace.

## Features

- Herdr plugin action + Herdr-managed overlay pane
- Rust TUI built with `ratatui` and `crossterm`
- Preview panel similar to `tv`
- Configurable search engine: `nucleo`, `skim`, or `simple`
- Configurable source priority order
- Reads Herdr Plus project templates when installed
- Uses Herdr theme custom tokens where available
- No dependency on external picker TUIs like `fzf` or `tv`

## Sources

| Source | What it reads | Enter behavior |
| --- | --- | --- |
| `workspace` | `herdr workspace list` + pane cwd | focus workspace |
| `project` | Herdr Plus `projects/*.toml` | focus existing cwd or create workspace + project tabs |
| `zoxide` | `zoxide query -l` | focus existing cwd or create workspace |
| `root` | configured filesystem roots | focus existing cwd or create workspace |
| `agent` | agent panes from `herdr pane list` | focus agent |

## Keybindings inside the picker

| Key | Action |
| --- | --- |
| type | fuzzy search |
| `Enter` | open/focus selected item |
| `Esc` / `Ctrl-C` | close |
| `Up` / `Down` | move selection |
| `Tab` | cycle source filters |
| `Ctrl-W` | show workspaces only |
| `Ctrl-P` | show Herdr Plus projects only |
| `Ctrl-Z` | show zoxide only |
| `Ctrl-R` | show root scan only |
| `Ctrl-A` | show agents only |
| `Ctrl-O` | toggle preview panel |
| `Ctrl-U` | clear query and filter |

## Local install

```bash
cd /home/fenix/workspace/herdr-workdir-picker
cargo build --release
herdr plugin link /home/fenix/workspace/herdr-workdir-picker
```

Run once without binding:

```bash
herdr plugin action invoke fenix.workdir-picker.open
```

## Bind to `prefix+t`

Add to `~/.config/herdr/config.toml`:

```toml
[[keys.command]]
key = "prefix+t"
type = "plugin_action"
command = "fenix.workdir-picker.open"
description = "workdir picker"
```

Reload Herdr:

```bash
herdr server reload-config
```

## Configuration

Find the managed plugin config directory:

```bash
herdr plugin config-dir fenix.workdir-picker
```

On first run, the plugin creates `config.toml` from [`examples/default-config.toml`](examples/default-config.toml).

### Default config

```toml
[picker]
reuse_existing = true
create_missing = true
engine = "nucleo" # nucleo | skim | simple
source_order = ["workspace", "project", "zoxide", "root", "agent"]
source_priority_boost = 25

[sources]
open_workspaces = true
herdr_plus_projects = true
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

### Search engines

| Engine | Use when |
| --- | --- |
| `nucleo` | default; fast, fzf-like ranking, good Unicode behavior |
| `skim` | compare against skim/fzf-style scoring |
| `simple` | debugging; tiny ordered-character matcher |

### Source priority

`source_order` controls source priority. Earlier sources get a ranking bonus and appear first on an empty query.

```toml
source_order = ["workspace", "project", "zoxide", "root", "agent"]
source_priority_boost = 25
```

Set the boost to zero for pure matcher score:

```toml
source_priority_boost = 0
```

Accepted names:

```text
workspace, open, project, zoxide, root, agent
```

## Debugging

List all candidates without opening the TUI:

```bash
./target/release/herdr-workdir-picker list
```

Show plugin actions:

```bash
herdr plugin action list --plugin fenix.workdir-picker
```

Unlink local plugin:

```bash
herdr plugin unlink fenix.workdir-picker
```

## Design notes

Herdr plugin v1 does not expose a native non-terminal custom UI API. This plugin follows the current Herdr-native pattern used by other plugins: an action opens a managed overlay pane, and the interactive TUI runs inside that pane.
