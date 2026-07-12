# Register .aether file association on Windows
# Run as Administrator

Write-Host "Registering .aether file association on Windows..."

$aetherExe = (Get-Command aether -ErrorAction SilentlyContinue).Source
if (-not $aetherExe) {
    $aetherExe = "$env:LOCALAPPDATA\aether\aether.exe"
}

# Create registry entries
New-Item -Path "Registry::HKEY_CLASSES_ROOT\.aether" -Force | Out-Null
Set-ItemProperty -Path "Registry::HKEY_CLASSES_ROOT\.aether" -Name "(Default)" -Value "AETHER.File"

New-Item -Path "Registry::HKEY_CLASSES_ROOT\AETHER.File" -Force | Out-Null
Set-ItemProperty -Path "Registry::HKEY_CLASSES_ROOT\AETHER.File" -Name "(Default)" -Value "AETHER Source File"

New-Item -Path "Registry::HKEY_CLASSES_ROOT\AETHER.File\DefaultIcon" -Force | Out-Null
Set-ItemProperty -Path "Registry::HKEY_CLASSES_ROOT\AETHER.File\DefaultIcon" -Name "(Default)" -Value "$aetherExe,0"

New-Item -Path "Registry::HKEY_CLASSES_ROOT\AETHER.File\shell\open\command" -Force | Out-Null
Set-ItemProperty -Path "Registry::HKEY_CLASSES_ROOT\AETHER.File\shell\open\command" -Name "(Default)" -Value "`"$aetherExe`" `"%1`""

Write-Host "✅ File association registered!"
Write-Host "Note: You may need to restart Explorer or log out/in for changes to take effect."
