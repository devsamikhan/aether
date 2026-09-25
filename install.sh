#!/usr/bin/env bash
# ==============================================================================
# AETHER 2.0 Global Toolchain Installer for Linux & macOS (Bash / Sh)
# Installs standalone native binary to ~/.aether/bin & configures PATH
# Zero external package managers, zero sudo / root privileges required.
# ==============================================================================

set -e

# Terminal Colors
CYAN='\033[0;36m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

echo -e "${CYAN}================================================================================${NC}"
echo -e "${CYAN}🚀 Installing AETHER 2.0 Programming Language Toolchain...${NC}"
echo -e "${CYAN}================================================================================${NC}"

# 1. Resolve Target Directories
AETHER_HOME="${AETHER_HOME:-$HOME/.aether}"
BIN_DIR="$AETHER_HOME/bin"
LIB_DIR="$AETHER_HOME/libraries"

mkdir -p "$BIN_DIR"
mkdir -p "$LIB_DIR"

# 2. Detect Operating System and Architecture
OS="$(uname -s)"
ARCH="$(uname -m)"

case "$OS" in
    Linux)
        OS_TAG="linux"
        ;;
    Darwin)
        OS_TAG="macos"
        ;;
    *)
        echo -e "${RED}[ERROR] Unsupported Operating System: $OS${NC}"
        exit 1
        ;;
esac

case "$ARCH" in
    x86_64|amd64)
        ARCH_TAG="x86_64"
        ;;
    arm64|aarch64)
        ARCH_TAG="aarch64"
        ;;
    *)
        echo -e "${YELLOW}[WARN] Unknown architecture $ARCH, defaulting to x86_64${NC}"
        ARCH_TAG="x86_64"
        ;;
esac

echo -e "  Platform detected: ${GREEN}${OS_TAG}-${ARCH_TAG}${NC}"

# 3. Locate or Acquire AETHER Binary
INSTALLED=false

# Option A: Check local release binary in git repository
if [ -f "target/release/aether" ]; then
    echo -e "  ✓ Found local release binary at target/release/aether"
    cp "target/release/aether" "$BIN_DIR/aether"
    INSTALLED=true
elif [ -f "target/debug/aether" ]; then
    echo -e "  ✓ Found local debug binary at target/debug/aether"
    cp "target/debug/aether" "$BIN_DIR/aether"
    INSTALLED=true
fi

# Option B: If cargo is installed in current repo, build release
if [ "$INSTALLED" = false ] && command -v cargo >/dev/null 2>&1 && [ -f "Cargo.toml" ]; then
    echo -e "  ⚙️  Building release binary via cargo..."
    cargo build --release
    cp "target/release/aether" "$BIN_DIR/aether"
    INSTALLED=true
fi

# Option C: Download from GitHub Releases
if [ "$INSTALLED" = false ]; then
    DOWNLOAD_URL="https://github.com/devsamikhan/aether/releases/latest/download/aether-${OS_TAG}-${ARCH_TAG}.tar.gz"
    echo -e "  ⬇️  Downloading AETHER precompiled binary from release..."
    TMP_DIR="$(mktemp -d)"
    if command -v curl >/dev/null 2>&1; then
        curl -fsSL "$DOWNLOAD_URL" -o "$TMP_DIR/aether.tar.gz" || true
    elif command -v wget >/dev/null 2>&1; then
        wget -q "$DOWNLOAD_URL" -O "$TMP_DIR/aether.tar.gz" || true
    fi

    if [ -f "$TMP_DIR/aether.tar.gz" ]; then
        tar -xzf "$TMP_DIR/aether.tar.gz" -C "$TMP_DIR"
        if [ -f "$TMP_DIR/aether" ]; then
            cp "$TMP_DIR/aether" "$BIN_DIR/aether"
            INSTALLED=true
        fi
    fi
    rm -rf "$TMP_DIR"
fi

if [ "$INSTALLED" = false ]; then
    # Fallback attempt: direct binary fetch
    echo -e "  ⚠️  Attempting standalone binary direct fetch..."
    DIRECT_URL="https://github.com/devsamikhan/aether/releases/latest/download/aether-${OS_TAG}-${ARCH_TAG}"
    if command -v curl >/dev/null 2>&1; then
        curl -fsSL "$DIRECT_URL" -o "$BIN_DIR/aether" || true
    fi
    if [ -f "$BIN_DIR/aether" ]; then
        INSTALLED=true
    fi
fi

if [ ! -f "$BIN_DIR/aether" ]; then
    echo -e "${RED}[ERROR] Could not install or compile AETHER. Please ensure Rust/Cargo is installed or precompiled release is accessible.${NC}"
    exit 1
fi

chmod +x "$BIN_DIR/aether"
echo -e "  ✓ Installed executable to: ${GREEN}$BIN_DIR/aether${NC}"

# 4. Copy Standard Libraries
if [ -d "libraries" ]; then
    cp libraries/*.ae "$LIB_DIR/" 2>/dev/null || true
    echo -e "  ✓ Installed standard libraries to: ${GREEN}$LIB_DIR${NC}"
fi

# 5. Configure User Shell PATH
PROFILE_UPDATED=false
for PROF in "$HOME/.bashrc" "$HOME/.zshrc" "$HOME/.profile"; do
    if [ -f "$PROF" ]; then
        if ! grep -q "$BIN_DIR" "$PROF"; then
            echo "" >> "$PROF"
            echo "# AETHER Toolchain PATH" >> "$PROF"
            echo "export PATH=\"$BIN_DIR:\$PATH\"" >> "$PROF"
            echo "export AETHER_HOME=\"$AETHER_HOME\"" >> "$PROF"
            PROFILE_UPDATED=true
            echo -e "  ✓ Registered '$BIN_DIR' in $PROF"
        else
            echo -e "  ✓ Already registered in $PROF"
        fi
    fi
done

echo -e "${CYAN}================================================================================${NC}"
echo -e "${GREEN}✨ AETHER 2.0 INSTALLATION COMPLETE!${NC}"
echo -e "  Binary:  $BIN_DIR/aether"
echo -e "  Channel: stable-${ARCH_TAG}"
echo -e ""
echo -e "${YELLOW}To begin using AETHER, reload your current shell:${NC}"
echo -e "    export PATH=\"$BIN_DIR:\$PATH\""
echo -e ""
echo -e "${YELLOW}Run the system diagnostic doctor:${NC}"
echo -e "    aether doctor"
echo -e "    aether bench"
echo -e "    aether new my_ai_engine --template ai"
echo -e "${CYAN}================================================================================${NC}"
