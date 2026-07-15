# Decisions

## Public name

Since v0.3.2, use `herdr-navigator` consistently for the plugin id, Cargo package/binary, config directory, repository, and action prefix. This intentionally replaces the pre-v0.3.2 `herdr-picker-plus` technical id while the project is still young; first run copies missing files from the legacy config directory.
Do not introduce another technical id or personal prefix.

## Minimum release quality

Keep these files:
- README
- LICENSE
- CHANGELOG
- SECURITY
- CONTRIBUTING
- RELEASE
- GitHub CI/release workflows
- issue/PR templates

But do not add enterprise boilerplate beyond that.

## No new dependencies for picker UX

The plugin is itself a Rust TUI. Do not depend on `fzf`, `tv`, etc.
`zoxide` is optional because it is a data source, not UI.

## Herdr Plus dependency stays optional

If Herdr Plus config dirs are absent, project/quick sources should degrade quietly.
No hard failure on missing Herdr Plus.

## Theme implementation is local mapping

Known limitation: Herdr plugin v1 does not provide active theme palette.
Local mapping + custom override is the accepted solution for now.

## Simplicity bias

This project should stay a compact plugin. Avoid speculative abstractions, plugin SDK wrappers, or multi-file refactors unless code size starts blocking safe changes.

## Server access uses remote handoff

Treat a remote server as a Herdr remote target, not a remote session. `Ctrl-S` filters servers; remote rows run `herdr --remote TARGET --handoff` to avoid nested Herdr. Local session rows stay local and run `herdr session attach NAME`. Picker should not own SSH config parsing, `.herdr-server.toml`, autossh tabs, or remote terminal attach listing unless terminal-level search becomes an explicit UX goal.

## Integration contract v1

Use a command/JSON list-open contract before building a plugin SDK. This keeps contributor burden low and avoids a speculative framework. Herdr Plus remains built in because it needs Herdr-specific workspace/tab bootstrap behavior.

Navigator owns notifications for integration open success/failure so plugin authors only implement list/open.

## Agent search feature shape

Use visible Herdr state first: agent name, workspace label/id, cwd, pane/tab/terminal ids, status. Add token filters for precision and aliases for user memory. Do not invent session names inside Navigator; aliases are search-only.

For now, `@` without text is the only agent-view shortcut. It is equivalent to Ctrl-A: main agent view, using `picker.agent_sort`. Default `herdr` reads Herdr `agent_panel_sort`; `priority` forces block first/done second/rest; `spaces` keeps Herdr/pane order. `@text` stays agent-only and matches workspace/session label/id or status text for fast navigation.

Agent display identity is the pane `cwd` where the agent was opened. `foreground_cwd` is searchable only; do not let a later `cd` rename/move the agent row.
