# Decisions

## Public name

Use only `herdr-picker-plus` / `Herdr Picker Plus`.
Do not use old plugin ids, old binary names, or personal prefixes.

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

## Integration contract v1

Use a command/JSON list-open contract before building a plugin SDK. This keeps contributor burden low and avoids a speculative framework. Herdr Plus remains built in because it needs Herdr-specific workspace/tab bootstrap behavior.

Picker Plus owns notifications for integration open success/failure so plugin authors only implement list/open.
