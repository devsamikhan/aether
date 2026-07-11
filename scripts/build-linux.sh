#!/bin/bash
set -e
echo "==================================================="
echo "Building Production Release: Linux x64 Statically Linked"
echo "==================================================="

# Use musl target for a static binary
cargo build --release --target x86_64-unknown-linux-musl

copy ../target/x86_64-unknown-linux-musl/release/aether ../releases/v1.0.0/aether-linux-x64
shasum -a 256 ../releases/v1.0.0/aether-linux-x64 | cut -d' ' -f1 > ../releases/v1.0.0/aether-linux-x64.sha256
echo "Linux build finished successfully."
