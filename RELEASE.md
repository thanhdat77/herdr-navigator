# Release

This project uses SemVer and Git tags.

## Checklist

1. Update `Cargo.toml` and `herdr-plugin.toml` to the same version.
2. Move changelog entries from `Unreleased` to the new version.
3. Run checks:

   ```bash
   cargo fmt --check
   cargo clippy -- -D warnings
   cargo test
   cargo build --release
   ```

4. Commit, tag, and push:

   ```bash
   git tag v0.1.0
   git push origin main --tags
   ```

GitHub Actions builds release archives for Linux and macOS when a `v*` tag is pushed.
