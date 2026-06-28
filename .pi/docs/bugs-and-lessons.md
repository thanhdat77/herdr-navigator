# Bugs and Lessons

## Old plugin id caused stale behavior

Symptom: user selected project; existing workspace could focus, but new project did not open as expected.

Root cause found: Herdr was still linked/bound to the old plugin id/action:

```text
old: fenix.workdir-picker.open
new: herdr-picker-plus.open
```

Fix:

```bash
herdr plugin unlink fenix.workdir-picker 2>/dev/null || true
herdr plugin link "$PWD"
herdr server reload-config
```

Also update Herdr config keybinding to:

```toml
command = "herdr-picker-plus.open"
```

Lesson: after renaming plugin id/binary, always check:

```bash
herdr plugin list
herdr plugin action list --plugin herdr-picker-plus
rg "fenix.workdir-picker|herdr-workdir-picker" ~/.config/herdr/config.toml .
```

## Theme inheritance misunderstanding

Symptom: “why hardcode all colors; can’t plugin get from Herdr?”

Fact: Herdr plugin v1 does not expose active palette. Only config/env/context are available.

Fix: use One Light fallback, map a small set of supported `theme.name`, then apply `[theme.custom]` overrides.

Lesson: phrase docs honestly: “maps supported Herdr theme names locally,” not “native palette access.”

## Release assets kept old names after tag force

Symptom: release had both old and new asset names.

Root cause: tag was force-updated; GitHub release retained previous uploaded assets.

Fix: delete stale release assets manually:

```bash
gh release delete-asset v0.1.0 old-name.tar.gz -y
```

Lesson: prefer new patch tag after public release. Force tags only before users depend on them.

## Config drift in dotfiles

Herdr active config is stow-managed from dotfiles. Plugin behavior can appear wrong if repo plugin is updated but dotfiles keybinding still points to old action.

Always check both:

```bash
rg "herdr-picker-plus|fenix.workdir-picker" ~/.config/herdr/config.toml /home/fenix/dotfiles/herdr/.config/herdr/config.toml
```

## Project open path assumptions

Project reuse depends on canonical cwd matching `working_dir`. If paths differ by symlink/case/relative expansion, reuse may fail and a duplicate workspace can be created.

Keep `canonical_str()` logic conservative. If bugs appear, first inspect `pane list` cwd/foreground_cwd and project `working_dir`.
