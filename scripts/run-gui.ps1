# Launch the GPUI desktop app
$ErrorActionPreference = "Stop"
Set-Location (Split-Path $PSScriptRoot -Parent)

$exe = "target\release\qtrans-gui.exe"
if (-not (Test-Path $exe)) { $exe = "target\debug\qtrans-gui.exe" }
if (-not (Test-Path $exe)) { throw "qtrans-gui not built. Run scripts\build.ps1 first." }

# GUI calls qtrans.exe from the same directory.
$cli = Join-Path (Split-Path $exe) "qtrans.exe"
if (-not (Test-Path $cli)) {
    Write-Host "warning: $cli not found, build the workspace first" -ForegroundColor Yellow
}

& $exe
