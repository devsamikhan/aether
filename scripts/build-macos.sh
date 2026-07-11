#!/bin/bash
set -e
echo "==================================================="
echo "Building Production Release: macOS Universal"
echo "==================================================="

# Build for both architectures and lipo them
cargo build --release --target x86_64-apple-darwin
cargo build --release --target aarch64-apple-darwin

lipo -create \
  ../target/x86_64-apple-darwin/release/aether \
  ../target/aarch64-apple-darwin/release/aether \
  -output ../releases/v1.0.0/aether-macos-universal

shasum -a 256 ../releases/v1.0.0/aether-macos-universal | cut -d' ' -f1 > ../releases/v1.0.0/aether-macos-universal.sha256
echo "macOS build finished successfully."
