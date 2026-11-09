# Build everything into ./target/release and ./bin
$ErrorActionPreference = "Stop"
Set-Location (Split-Path $PSScriptRoot -Parent)
$workspaceManifest = Get-Content -Raw "Cargo.toml"
if ($workspaceManifest -notmatch '(?ms)^\[workspace\.package\]\s*.*?^version\s*=\s*"([^"]+)"') {
    throw "Unable to read workspace version from Cargo.toml."
}
$appVersion = $Matches[1]

Write-Host "== Rust workspace ==" -ForegroundColor Cyan
rtk cargo build --release

Write-Host "== Go translation service ==" -ForegroundColor Cyan
Push-Location go\translator
try {
    rtk go build -ldflags "-X main.version=$appVersion" -o "../../bin/qtrans-translator.exe" .
} finally {
    Pop-Location
}

Write-Host ""
Write-Host "Done:" -ForegroundColor Green
Write-Host "  target\release\qtrans.exe"
Write-Host "  target\release\qtrans-gui.exe"
Write-Host "  bin\qtrans-translator.exe"
