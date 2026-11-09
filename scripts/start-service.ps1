# Start the Go translation service on http://127.0.0.1:8787
$ErrorActionPreference = "Stop"
Set-Location (Split-Path $PSScriptRoot -Parent)

if (-not $env:QTRANS_ADDR) { $env:QTRANS_ADDR = "127.0.0.1:8787" }

$exe = Join-Path (Get-Location) "bin\qtrans-translator.exe"
if (-not (Test-Path $exe)) {
    Write-Host "translator not built, building..." -ForegroundColor Yellow
    $manifest = Get-Content -Raw "Cargo.toml"
    if ($manifest -notmatch '(?ms)^\[workspace\.package\]\s*.*?^version\s*=\s*"([^"]+)"') {
        throw "Unable to read workspace version from Cargo.toml."
    }
    $appVersion = $Matches[1]
    Push-Location go\translator
    try { rtk go build -ldflags "-X main.version=$appVersion" -o "../../bin/qtrans-translator.exe" . } finally { Pop-Location }
}

$running = Get-Process qtrans-translator -ErrorAction SilentlyContinue
if ($running) {
    Write-Host "translator already running (pid $($running.Id))" -ForegroundColor Yellow
    exit 0
}

Write-Host "listening on http://$($env:QTRANS_ADDR)  (Ctrl+C to stop)" -ForegroundColor Green
& $exe -addr $env:QTRANS_ADDR -cache "work\cache\translations.json" -workers 4
