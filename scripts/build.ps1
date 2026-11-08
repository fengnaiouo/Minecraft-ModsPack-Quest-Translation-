# Build everything into ./target/release and ./bin
$ErrorActionPreference = "Stop"
Set-Location (Split-Path $PSScriptRoot -Parent)

Write-Host "== Rust workspace ==" -ForegroundColor Cyan
rtk cargo build --release

Write-Host "== Go translation service ==" -ForegroundColor Cyan
Push-Location go\translator
try {
    rtk go build -o "../../bin/qtrans-translator.exe" .
} finally {
    Pop-Location
}

Write-Host ""
Write-Host "Done:" -ForegroundColor Green
Write-Host "  target\release\qtrans.exe"
Write-Host "  target\release\qtrans-gui.exe"
Write-Host "  bin\qtrans-translator.exe"
