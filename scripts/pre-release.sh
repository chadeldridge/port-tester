#!/usr/bin/env bash
set -e

parent=$(basename "$(pwd)")
if [[ "$parent" == "scripts" ]]; then
    cd ../
fi

echo "update --locked..."
cargo update -p port_tester --locked

# Run test jobs.
echo "test --locked --release..."
cargo test --locked --release

# Binary build and run.
echo
echo "cargo run -- 8.8.8.8 -c 1 -s"
cargo run --locked -- 8.8.8.8 -c 1 -s
echo "cargo run -- -v 8.8.8.8 53 -c 1"
cargo run --locked -- -v 8.8.8.8 53 -c 1

if ! command -v cargo-fmt &>/dev/null; then
    echo "cargo-fmt not installed!!!"
    exit 1
fi
echo "format check..."
cargo fmt --all --check

if ! command -v cargo-clippy &>/dev/null; then
    echo "cargo-clippy not installed!!!"
    exit 1
fi
echo "clippy..."
cargo clippy --workspace --all-targets --no-deps -- -D warnings

if ! command -v cargo-deny &>/dev/null; then
    echo "cargo-deny not installed!!!"
    exit 1
fi
# audit job (needs cargo-deny installed)
echo "deny check..."
cargo deny check bans licenses sources

if ! command -v typos &>/dev/null; then
    echo "typos not installed! Run: cargo install typos-cli"
    exit 1
fi
echo "spellcheck..."
typos

if ! command -v cargo-hack &>/dev/null; then
    echo "cargo-hack not installed! Run: cargo install cargo-hack"
    exit 1
fi
echo "run hack..."
cargo hack check --all-targets --rust-version --workspace --ignore-private --locked

if ! command -v cargo-semver-checks &>/dev/null; then
    echo "cargo-semver-checks not installed! Run: cargo install cargo-semver-checks"
    exit 1
fi
#echo "semver-checks..."
#cargo semver-checks

# Run audit before sending to CICD to try and catch issues early
if ! command -v cargo-audit &>/dev/null; then
    echo "cargo-audit not installed!!!"
    exit 1
fi
echo audit...
cargo audit

# Find and set the MSRV (Minimum Supported Rust Version) in Cargo.toml
if ! command -v cargo-msrv &>/dev/null; then
    echo "cargo-msrv not installed!!!"
    exit 1
fi
echo msrv...
cargo msrv find --write-msrv

# Publish library dryrun and checks.
echo
echo "cargo publish..."
cargo publish --dry-run --allow-dirty
du -sh target/package/tmp-crate/*.crate
cargo package --list --allow-dirty
