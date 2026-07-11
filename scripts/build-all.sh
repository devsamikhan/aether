#!/bin/bash
set -e
echo "Building all release targets..."

./build-windows.bat || true
./build-macos.sh || true
./build-linux.sh || true

echo "Cross compilation script execution completed."
