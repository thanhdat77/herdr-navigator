# Release Notes for Agents

## Current release process

1. Bump `Cargo.toml` and `herdr-plugin.toml` together.
2. Update `CHANGELOG.md`.
3. Run:

```bash
cargo fmt --check
cargo clippy -- -D warnings
cargo test
cargo build --release
```

4. Commit.
5. Tag `vX.Y.Z`.
6. Push branch and tag.

Release workflow builds archives:
- `herdr-picker-plus-linux-x86_64.tar.gz`
- `herdr-picker-plus-macos-aarch64.tar.gz`

## Avoid

- Do not reuse old package/binary names.
- Do not force-update tags after public usage unless explicitly asked.
- Do not add screenshots/badges with fake URLs.

## Useful commands

```bash
git status --short
gh run list --repo <owner>/herdr-picker-plus --limit 5
gh release view vX.Y.Z --repo <owner>/herdr-picker-plus
```
