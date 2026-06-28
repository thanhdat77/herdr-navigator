# Feature Intent

## Picker goal

One picker center that answers: “where do I want to work next, or what Herdr action do I need now?”

Do not split into many specialized pickers unless the UX clearly needs it. The product direction is “kinda like tv, but deeply integrated with Herdr.”

## Sources

Default source order:

```toml
["workspace", "project", "zoxide", "root", "agent", "quick"]
```

Source priority is intentional: existing/open things first, creation sources later, quick actions available but not dominant.

## Keybindings

- `Tab`: cycle source filters
- `Ctrl-W`: workspace
- `Ctrl-P`: Herdr Plus projects
- `Ctrl-Q`: Herdr Plus Quick Actions
- `Ctrl-Z`: zoxide
- `Ctrl-R`: roots
- `Ctrl-A`: agents
- `Ctrl-O`: preview
- `Ctrl-U`: clear query/filter

Keep keybindings mnemonic and few.

## Herdr Plus

Project should be usable from this picker directly:
- already open -> focus existing workspace
- not open -> create workspace and apply project tabs

Quick Actions should be accessible here, but the real Quick Actions UI remains owned by Herdr Plus.
This plugin only launches it.

## Theme

User cares that the picker visually belongs inside Herdr. “Inherit theme” means practical visual matching, not perfect API-level inheritance, because Herdr does not expose palette to plugin v1.

Prefer adding only palettes users actually need.

## Command/JSON plugin integrations

Users can add external tools without Rust changes:

```toml
[[integrations]]
id = "my-plugin"
label = "My Plugin"
collect = "my-plugin list --json"
open = "my-plugin open {{id}}"
```

Collect JSON minimum: `id`, `title`. Optional: `subtitle`, `path`, `kind`.
