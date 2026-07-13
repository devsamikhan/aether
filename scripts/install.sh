#!/bin/bash
set -e

echo "[INSTALL] Installing AETHER..."

# Check prerequisites
if ! command -v cargo &> /dev/null; then
    echo "[ERROR] Error: Rust/Cargo is not installed"
    echo "Please install Rust from https://rustup.rs/"
    exit 1
fi

# Get installation directory
INSTALL_DIR="${AETHER_INSTALL_DIR:-$HOME/.aether}"
BIN_DIR="$INSTALL_DIR/bin"

echo "[INFO] Installing to $INSTALL_DIR"

# Create directories
mkdir -p "$BIN_DIR"

# Build release
echo "[BUILD] Building AETHER..."
cargo build --release

# Copy binary
echo "[COPY] Copying binary..."
cp target/release/aether "$BIN_DIR/aether"

# Make executable
chmod +x "$BIN_DIR/aether"

# Add to PATH (bash/zsh)
SHELL_PROFILE=""
if [ -f "$HOME/.bashrc" ]; then
    SHELL_PROFILE="$HOME/.bashrc"
elif [ -f "$HOME/.zshrc" ]; then
    SHELL_PROFILE="$HOME/.zshrc"
elif [ -f "$HOME/.bash_profile" ]; then
    SHELL_PROFILE="$HOME/.bash_profile"
fi

if [ -n "$SHELL_PROFILE" ]; then
    if ! grep -q "$BIN_DIR" "$SHELL_PROFILE"; then
        echo "" >> "$SHELL_PROFILE"
        echo "# AETHER" >> "$SHELL_PROFILE"
        echo "export PATH=\"$BIN_DIR:\$PATH\"" >> "$SHELL_PROFILE"
        echo "export AETHER_INSTALL_DIR=\"$INSTALL_DIR\"" >> "$SHELL_PROFILE"
        echo "[SUCCESS] Added AETHER to PATH in $SHELL_PROFILE"
    fi
fi

# Verify installation
echo ""
echo "[SUCCESS] AETHER installed successfully!"
echo ""
echo "To use AETHER, run:"
echo "  source $SHELL_PROFILE"
echo "  aether --version"
echo ""
echo "Or add to PATH manually:"
echo "  export PATH=\"$BIN_DIR:\$PATH\""
