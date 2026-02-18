#!/usr/bin/env bash
set -e

parent=$(basename "$(pwd)")
if [[ "$parent" == "scripts" ]]; then
    cd ../
fi

# Run audit before sending to CICD to try and catch issues early
cargo audit
# Find and set the MSRV (Minimum Supported Rust Version) in Cargo.toml
cargo msrv find --write-msrv