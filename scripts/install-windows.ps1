Write-Host "[INSTALL] Installing AETHER..." -ForegroundColor Green

# Check prerequisites
if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
    Write-Host "[ERROR] Error: Rust/Cargo is not installed" -ForegroundColor Red
    Write-Host "Please install Rust from https://rustup.rs/" -ForegroundColor Yellow
    exit 1
}

# Get installation directory
$InstallDir = if ($env:AETHER_INSTALL_DIR) { $env:AETHER_INSTALL_DIR } else { "$env:LOCALAPPDATA\aether" }
$BinDir = "$InstallDir\bin"

Write-Host "[INFO] Installing to $InstallDir"

# Create directories
New-Item -ItemType Directory -Force -Path $BinDir | Out-Null

# Build release
Write-Host "[BUILD] Building AETHER..."
cargo build --release
if ($LASTEXITCODE -ne 0) {
    Write-Host "[ERROR] Build failed!" -ForegroundColor Red
    exit 1
}

# Copy binary
Write-Host "[COPY] Copying binary..."
Copy-Item "target\release\aether.exe" "$BinDir\aether.exe" -Force

# Add to PATH
$CurrentPath = [Environment]::GetEnvironmentVariable("Path", "User")
if ($CurrentPath -notlike "*$BinDir*") {
    [Environment]::SetEnvironmentVariable("Path", "$CurrentPath;$BinDir", "User")
    Write-Host "[SUCCESS] Added AETHER to PATH" -ForegroundColor Green
}

# Set environment variable
[Environment]::SetEnvironmentVariable("AETHER_INSTALL_DIR", $InstallDir, "User")

# Verify installation
Write-Host ""
Write-Host "[SUCCESS] AETHER installed successfully!" -ForegroundColor Green
Write-Host ""
Write-Host "To use AETHER, restart your terminal and run:" -ForegroundColor Yellow
Write-Host "  aether --version"
Write-Host ""
Write-Host "Or add to PATH manually:" -ForegroundColor Yellow
Write-Host "  `$env:PATH += `";$BinDir`""
