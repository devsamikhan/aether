#!/bin/sh
# verify-install.sh - Checks AETHER installation validity
set -e

INSTALL_DIR="$HOME/.aether"
BINARY_PATH="$INSTALL_DIR/bin/aether"

echo "=================================================="
echo "Checking AETHER Installation Validity"
echo "=================================================="

if [ -f "$BINARY_PATH" ]; then
    echo "Found binary at $BINARY_PATH"
    if "$BINARY_PATH" --version >/dev/null 2>&1; then
        echo "Binary validation check: PASSED"
        echo "Installed version:"
        "$BINARY_PATH" --version
    else
        echo "Binary validation check: FAILED (Cannot execute version query)"
    fi
else
    echo "Binary NOT found at $BINARY_PATH"
fi

if [ -d "$INSTALL_DIR/libraries/std" ]; then
    echo "Standard libraries directory: OK"
else
    echo "Standard libraries directory: MISSING"
fi
