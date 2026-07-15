# Architecture

Herdr Navigator is a picker center for Herdr: one overlay for choosing where to go or what Herdr-adjacent action to run.

It is similar in spirit to `tv`, but deeper integrated with Herdr. Instead of only returning a selected path/item, it can focus existing Herdr state, create Herdr workspaces, apply Herdr Plus project tabs, focus agents, or launch Herdr Plus Quick Actions.

## Runtime shape

```text
Herdr keybinding
  -> plugin action: herdr-navigator.open
  -> Herdr opens overlay pane: picker
  -> binary runs: herdr-navigator ui
  -> collect sources
  -> fuzzy filter/rank
  -> Enter dispatches Herdr action
```

The plugin is intentionally a terminal TUI running inside a Herdr-managed overlay pane. Herdr plugin v1 does not expose a native custom UI surface.

## Entry points

| Command | Purpose |
| --- | --- |
| `herdr-navigator open` | Ask Herdr to open the picker overlay pane |
| `herdr-navigator ui` | Run the interactive TUI inside that pane |
| `herdr-navigator list` | Debug: print collected entries without opening TUI |

## Code layout

```text
src/main.rs      CLI entrypoints
src/app.rs       picker state, filtering, open dispatch
src/tui.rs       terminal UI and command execution
src/keymap.rs    shared key registry, labels, groups, and active states
src/config.rs    plugin config loading and defaults
src/model.rs     shared Source/Entry/Project models
src/sources.rs   workspace/project/zoxide/root/agent/quick collectors
src/herdr.rs     small Herdr CLI wrapper
src/theme.rs     theme mapping and custom overrides
src/matcher.rs   fuzzy scoring engines
src/paths.rs     path/config helpers
```

Keep new integrations in `sources.rs` unless they grow enough to deserve their own module. Keep Herdr CLI calls behind `herdr.rs` where practical.

Picker input is an exclusive `InputMode` state machine (`Normal`, `Search`, `Help`); key scopes, footer hints, and transitions are defined through `keymap.rs`.

## Sources

| Source | Input | Enter behavior |
| --- | --- | --- |
| `workspace` | `herdr workspace list` + pane cwd | focus existing workspace |
| `project` | Herdr Plus project TOML files | focus existing cwd or create workspace + tabs |
| `quick` | Herdr Plus Quick Actions | open Herdr Plus Quick Actions picker |
| `zoxide` | `zoxide query -l` | focus existing cwd or create workspace |
| `root` | configured filesystem roots | focus existing cwd or create workspace |
| `agent` | `herdr pane list` agent metadata | focus agent pane |

## Core open rule

The picker should prefer reuse over duplication:

```text
selected item path already open -> focus existing workspace
not open + creation allowed -> create workspace
project template selected -> create workspace, then apply tabs
agent selected -> focus agent pane
quick selected -> delegate to Herdr Plus Quick Actions
```

This keeps the picker useful as a navigation center, not only a launcher.

## Herdr Plus boundary

Herdr Navigator integrates with Herdr Plus but does not copy all Herdr Plus behavior.

Current integration:

- reads project templates from Herdr Plus config
- creates/focuses workspaces for projects
- applies project tabs and startup commands
- launches the Herdr Plus Quick Actions picker

Boundary:

- Herdr Plus remains the owner of full Quick Actions UI and action execution
- this plugin only surfaces Quick Actions as one source inside the picker center

## Theme boundary

Herdr plugin v1 does not expose the active theme palette directly to external plugins.

Current behavior:

1. read `~/.config/herdr/config.toml`
2. map supported `theme.name` values locally
3. apply `[theme.custom]` overrides last
4. fall back to One Light

This is practical theme inheritance, not native palette access.

## Design goals

- One fast overlay for “where next?”
- Deep Herdr actions on selection, not just printing paths
- Optional integrations degrade quietly
- Small Rust binary, no external picker UI dependency
- Keep source model simple enough to add more Herdr-aware sources later

## Non-goals

- Replacing Herdr Plus
- Replacing Herdr's built-in `prefix+g`
- Building a generic plugin framework
- Perfect theme parity until Herdr exposes plugin palette data
