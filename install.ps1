# ==============================================================================
# AETHER 2.0 Global Toolchain Installer for Windows (PowerShell)
# Installs standalone native binary to %USERPROFILE%\.aether\bin & configures PATH
# Zero external installers, zero administrative privileges required.
# ==============================================================================

Write-Host "================================================================================" -ForegroundColor Cyan
Write-Host "Installing AETHER 2.0 Programming Language Toolchain..." -ForegroundColor Cyan
Write-Host "================================================================================" -ForegroundColor Cyan

$AetherHome = if ($env:AETHER_HOME) { $env:AETHER_HOME } else { "$env:USERPROFILE\.aether" }
$BinDir = "$AetherHome\bin"
$LibDir = "$AetherHome\libraries"

# 1. Ensure Directories Exist
if (-not (Test-Path $BinDir)) {
    New-Item -ItemType Directory -Path $BinDir -Force | Out-Null
}
if (-not (Test-Path $LibDir)) {
    New-Item -ItemType Directory -Path $LibDir -Force | Out-Null
}

# 2. Locate Active Executable or Download Pre-built Release
$Installed = $false

if (Test-Path "target\release\aether.exe") {
    Copy-Item -Path "target\release\aether.exe" -Destination "$BinDir\aether.exe" -Force
    Write-Host "  [OK] Installed local release binary to: $BinDir\aether.exe" -ForegroundColor Green
    $Installed = $true
} elseif (Test-Path "target\debug\aether.exe") {
    Copy-Item -Path "target\debug\aether.exe" -Destination "$BinDir\aether.exe" -Force
    Write-Host "  [OK] Installed local debug binary to: $BinDir\aether.exe" -ForegroundColor Green
    $Installed = $true
} elseif ((Get-Command cargo -ErrorAction SilentlyContinue) -and (Test-Path "Cargo.toml")) {
    Write-Host "  Building release binary via cargo..." -ForegroundColor Yellow
    cargo build --release
    Copy-Item -Path "target\release\aether.exe" -Destination "$BinDir\aether.exe" -Force
    Write-Host "  [OK] Installed compiled binary to: $BinDir\aether.exe" -ForegroundColor Green
    $Installed = $true
} else {
    Write-Host "  Downloading precompiled AETHER binary from GitHub Release..." -ForegroundColor Yellow
    $ReleaseUrl = "https://github.com/devsamikhan/aether/releases/latest/download/aether-windows-x64.exe"
    try {
        Invoke-WebRequest -Uri $ReleaseUrl -OutFile "$BinDir\aether.exe" -UseBasicParsing
        Write-Host "  [OK] Downloaded standalone binary to: $BinDir\aether.exe" -ForegroundColor Green
        $Installed = $true
    } catch {
        Write-Host "  Could not download precompiled release: $_" -ForegroundColor Red
    }
}

if (-not $Installed -or -not (Test-Path "$BinDir\aether.exe")) {
    Write-Error "Failed to install AETHER. Please ensure internet access or install Rust."
    exit 1
}

# 3. Copy Standard Libraries
if (Test-Path "libraries") {
    Copy-Item -Path "libraries\*.ae" -Destination $LibDir -Force
    Write-Host "  [OK] Installed standard libraries to: $LibDir" -ForegroundColor Green
}

# 4. Configure User PATH Permanently
$UserPath = [Environment]::GetEnvironmentVariable("PATH", "User")
$PathArray = $UserPath -split ';'
if ($PathArray -notcontains $BinDir) {
    if ($UserPath) {
        $NewPath = "$UserPath;$BinDir"
    } else {
        $NewPath = $BinDir
    }
    [Environment]::SetEnvironmentVariable("PATH", $NewPath, "User")
    $env:PATH = "$env:PATH;$BinDir"
    Write-Host "  [OK] Registered '$BinDir' in User PATH environment variable." -ForegroundColor Green
} else {
    Write-Host "  [OK] Toolchain directory '$BinDir' already in PATH." -ForegroundColor Green
}

Write-Host "================================================================================" -ForegroundColor Cyan
Write-Host "AETHER 2.0 INSTALLATION COMPLETE!" -ForegroundColor Green
Write-Host "  Version: 1.1.0 (stable-x86_64)"
Write-Host "  Binary:  $BinDir\aether.exe"
Write-Host ""
Write-Host "To verify installation, open a new terminal and run:" -ForegroundColor Yellow
Write-Host "    aether doctor" -ForegroundColor White
Write-Host "    aether bench" -ForegroundColor White
Write-Host "    aether new my_project" -ForegroundColor White
Write-Host "================================================================================" -ForegroundColor Cyan
