#!/bin/bash
set -e

echo "[TEST] Testing AETHER Installation..."

# Test 1: Build from source
echo "Test 1: Building from source..."
cargo build --release
if [ $? -eq 0 ]; then
    echo "[SUCCESS] Build successful"
else
    echo "[ERROR] Build failed"
    exit 1
fi

# Test 2: Run tests
echo "Test 2: Running tests..."
cargo test --release
if [ $? -eq 0 ]; then
    echo "[SUCCESS] Tests passed"
else
    echo "[ERROR] Tests failed"
    exit 1
fi

# Test 3: Check formatting
echo "Test 3: Checking formatting..."
cargo fmt -- --check
if [ $? -eq 0 ]; then
    echo "[SUCCESS] Formatting clean"
else
    echo "[ERROR] Formatting issues"
    exit 1
fi

# Test 4: Check clippy
echo "Test 4: Running clippy..."
cargo clippy -- -D warnings
if [ $? -eq 0 ]; then
    echo "[SUCCESS] Clippy clean"
else
    echo "[ERROR] Clippy warnings"
    exit 1
fi

# Test 5: Run CLI commands
echo "Test 5: Testing CLI commands..."
cargo run -- --version
cargo run -- --help
cargo run -- init test-project
cd test-project
cargo run -- build
cd ..
rm -rf test-project

echo ""
echo "[SUCCESS] All installation tests passed!"
