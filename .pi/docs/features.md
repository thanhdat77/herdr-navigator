# Feature Intent

## Picker goal

One overlay that answers: “where do I want to work next?”

Do not split into many specialized pickers unless the UX clearly needs it.

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
