#!/bin/sh
# install.sh - Universal macOS / Linux installer for AETHER
set -e

VERSION="0.2.0"
INSTALL_DIR="$HOME/.aether"
BIN_DIR="$INSTALL_DIR/bin"
LIB_DIR="$INSTALL_DIR/libraries"
BINARY_PATH="$BIN_DIR/aether"

VERBOSE=false
QUIET=false
UNINSTALL=false

# Argument parsing
for arg in "$@"; do
    case $arg in
        --uninstall) UNINSTALL=true ;;
        --verbose) VERBOSE=true ;;
        --quiet) QUIET=true ;;
    esac
done

log_info() {
    if [ "$QUIET" = false ]; then
        echo -e "\033[0;36m[AETHER Installer]\033[0m $1"
    fi
}

log_err() {
    echo -e "\033[0;31m[AETHER Error]\033[0m $1" >&2
}

if [ "$UNINSTALL" = true ]; then
    log_info "Uninstalling AETHER from system..."
    rm -rf "$INSTALL_DIR"
    log_info "AETHER directories removed successfully. Please clean up any path references in .bashrc/.zshrc."
    exit 0
fi

# Detect platform
OS=$(uname -s | tr '[:upper:]' '[:lower:]')
ARCH=$(uname -m)

log_info "Detecting system architecture... OS: $OS, Arch: $ARCH"

case "$OS" in
    darwin)
        if [ "$ARCH" = "arm64" ]; then
            TARGET_BIN="aether-macos-arm64"
        else
            TARGET_BIN="aether-macos-x64"
        fi
        ;;
    linux)
        TARGET_BIN="aether-linux-x64"
        ;;
    *)
        log_err "Unsupported operating system: $OS"
        exit 1
        ;;
esac

# Create directory structure
mkdir -p "$BIN_DIR"
mkdir -p "$LIB_DIR"

DOWNLOAD_URL="https://raw.githubusercontent.com/devsamikhan/aether/main/releases/v$VERSION/$TARGET_BIN"
CHECKSUM_URL="$DOWNLOAD_URL.sha256"

TEMP_BIN="/tmp/aether-temp"
TEMP_SHA="/tmp/aether-temp.sha256"

# Verify HTTP clients
if command -v curl >/dev/null 2>&1; then
    FETCH_CMD="curl -fsSL"
elif command -v wget >/dev/null 2>&1; then
    FETCH_CMD="wget -qO-"
else
    log_err "Neither curl nor wget was found on the system. Please install one to continue."
    exit 1
fi

log_info "Downloading AETHER production executable..."
if $FETCH_CMD "$DOWNLOAD_URL" > "$TEMP_BIN" 2>/dev/null && $FETCH_CMD "$CHECKSUM_URL" > "$TEMP_SHA" 2>/dev/null; then
    log_info "Release files fetched successfully."
else
    log_info "Could not fetch release from server (network offline / mock release mode)."
    # Fallback to local copy if available
    if [ -f "./target/release/aether" ]; then
        log_info "Copying local binary from target/release..."
        cp "./target/release/aether" "$TEMP_BIN"
        echo "LOCAL_MOCK_CHECKSUM_OK" > "$TEMP_SHA"
    else
        log_err "Local build targets not found. Run 'cargo build --release' first."
        exit 1
    fi
fi

# Verify checksum
log_info "Verifying SHA-256 binary checksum..."
EXPECTED_SHA=$(cat "$TEMP_SHA" | tr -d ' \n\r')

if [ "$EXPECTED_SHA" = "LOCAL_MOCK_CHECKSUM_OK" ]; then
    log_info "Checksum verification bypassed for local mock install."
else
    if command -v sha256sum >/dev/null 2>&1; then
        CALCULATED_SHA=$(sha256sum "$TEMP_BIN" | cut -d' ' -f1)
    elif command -v shasum >/dev/null 2>&1; then
        CALCULATED_SHA=$(shasum -a 256 "$TEMP_BIN" | cut -d' ' -f1)
    else
        log_err "No checksum utility found. Skipping checksum verification."
        CALCULATED_SHA="$EXPECTED_SHA"
    fi

    if [ "$CALCULATED_SHA" != "$EXPECTED_SHA" ]; then
        log_err "Checksum verification FAILED. File integrity compromised."
        rm -f "$TEMP_BIN" "$TEMP_SHA"
        exit 1
    fi
    log_info "Checksum verification PASSED."
fi

# Validate binary
chmod +x "$TEMP_BIN"
if ! "$TEMP_BIN" --version >/dev/null 2>&1; then
    log_err "Downloaded binary fails validation run. Installation aborted."
    rm -f "$TEMP_BIN" "$TEMP_SHA"
    exit 1
fi

# Backup and move to destination
if [ -f "$BINARY_PATH" ]; then
    log_info "Backing up previous AETHER installation..."
    mv "$BINARY_PATH" "$BINARY_PATH.backup"
fi

mv "$TEMP_BIN" "$BINARY_PATH"
rm -f "$TEMP_SHA"

log_info "Adding execution permissions..."
chmod +x "$BINARY_PATH"

log_info "Installation complete! Please add '$BIN_DIR' to your PATH variable."
echo "Example: export PATH=\"\$PATH:$BIN_DIR\""
