# herdr-sheep dev tasks — run `just <task>` (https://github.com/casey/just)

# default: list tasks
default:
    @just --list

# format the code
fmt:
    cargo fmt --all

# check formatting (CI parity)
fmt-check:
    cargo fmt --all --check

# lint with clippy, warnings as errors (CI parity)
lint:
    cargo clippy --all-targets --all-features -- -D warnings

# run the test suite
test:
    cargo test --all-features

# dependency advisories, licenses, and sources (CI parity)
audit:
    cargo deny check

# the crate version, the plugin manifest, and an optional tag must agree
check-version tag="":
    bash herdr/check-version.sh {{ tag }}

# regenerate CHANGELOG.md from the Conventional Commit history. Pass the tag being
# released to file the pending commits under it instead of Unreleased.
changelog tag="":
    git cliff --config cliff.toml {{ if tag == "" { "" } else { "--tag " + tag } }} --output CHANGELOG.md

# what the next release body will say
release-notes:
    git cliff --config cliff.toml --unreleased --strip header

# print one frame with a sheep in every state
demo size="100x38":
    cargo run --release --quiet -- --demo {{ size }}

# `herdr plugin link` skips build commands, so a linked checkout stages its own
# binary: build release and put it where the plugin manifest looks for it
install:
    cargo build --release
    mkdir -p bin
    install -m 0755 target/release/herdr-sheep bin/herdr-sheep

# link this checkout into Herdr as the live plugin
link: install
    herdr plugin link "$PWD"

# pty smoke test of the mouse path against a real release binary
smoke:
    cargo build --release
    python3 scripts/smoke-click.py

# everything CI runs, locally
ci: check-version fmt-check lint test smoke
    cargo run --release --quiet -- --demo 100x38 > /dev/null
