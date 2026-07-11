#!/bin/sh
# uninstall.sh - Removes AETHER distribution completely
set -e

INSTALL_DIR="$HOME/.aether"

echo "=================================================="
echo "Removing AETHER Programming Language Environment"
echo "=================================================="

if [ -d "$INSTALL_DIR" ]; then
    rm -rf "$INSTALL_DIR"
    echo "Removed AETHER installation directory: $INSTALL_DIR"
else
    echo "AETHER installation not found at $INSTALL_DIR"
fi

echo "=================================================="
echo "Uninstall finished. Please check your .bashrc/.zshrc/PATH profiles."
