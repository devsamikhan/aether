@echo off
echo ===================================================
echo Building Production Release: Windows x64
echo ===================================================
cargo build --release
if %errorlevel% neq 0 (
    echo Error: Cargo build failed.
    exit /b %errorlevel%
)

copy /Y ..\target\release\aether.exe ..\releases\v1.0.0\aether-windows-x64.exe
powershell -Command "Get-FileHash -Path ..\releases\v1.0.0\aether-windows-x64.exe -Algorithm SHA256 | Select-Object -ExpandProperty Hash | Out-File -FilePath ..\releases\v1.0.0\aether-windows-x64.exe.sha256 -NoNewline -Encoding ascii"
echo Checksum generated:
type ..\releases\v1.0.0\aether-windows-x64.exe.sha256
echo.
echo Windows build finished successfully.
