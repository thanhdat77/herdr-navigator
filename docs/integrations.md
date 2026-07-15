# Integrations

Herdr Navigator is intended to be a picker center. Other Herdr plugins should integrate at the boundary of Herdr actions and config, not by depending on internal Rust modules.

## Stable public surface

| Surface | Value |
| --- | --- |
| Plugin id | `herdr-picker-plus` |
| Open action | `herdr-picker-plus.open` |
| Binary | `herdr-picker-plus` |
| Debug list | `herdr-picker-plus list` |

## From Herdr config

Bind the picker to any key:

```toml
[[keys.command]]
key = "prefix+t"
type = "plugin_action"
command = "herdr-picker-plus.open"
description = "picker center"
```

## From another plugin

Another plugin can open Navigator by invoking its action through Herdr:

```bash
herdr plugin action invoke herdr-picker-plus.open
```

Use this when your plugin wants to hand control back to the central picker instead of building its own picker UI.

## Herdr Plus integration pattern

Current built-in integrations:

- Projects: read Herdr Plus project TOML files and open/apply them directly.
- Quick Actions: add one `quick` entry that delegates to Herdr Plus Quick Actions.

This is the preferred pattern:

```text
If the external plugin owns complex UI/execution, Navigator should launch/delegate.
If the external plugin exposes simple declarative data, Navigator may read it as a source.
```

## Adding future plugin sources

A good source should provide:

- a stable entry label
- searchable title/subtitle/path-like context
- one clear Enter behavior
- quiet degradation when the plugin is not installed

Avoid hard dependencies. Missing plugin config or binaries should not break the picker.
