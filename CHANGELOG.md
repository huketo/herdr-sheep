# Changelog

Generated from Conventional Commit messages with [git-cliff](https://git-cliff.org).
The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and the
project uses [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.2.0] - 2026-08-31

### Added

- **pasture**: Fence the zones and raise a barn on the horizon
- **pasture**: Select a sheep by clicking it, focus it by clicking again
- **plugin**: Open the pasture over a pane or beside it, from one keybinding each
- **release**: Install a prebuilt binary instead of building on install

### Development

- Adopt rustfmt, cargo-deny, git-cliff, and a justfile
- Check formatting, lints, tests, and the mouse path on every push

### Documentation

- Document the fence, the mouse, both keybindings, and releasing
- Make AGENTS.md the canonical doc for changing this code

### Tests

- Cover the CLI surface and the broken-bridge path end to end

## [0.1.0] - 2026-08-31

### Added

- Herdr-sheep, an animated ASCII sheep pasture for Herdr agents

