param(
    [Parameter(Mandatory = $true)][string]$Zip,
    [string]$Provider = "google_free",
    [string]$Service = "http://127.0.0.1:8787",
    [int]$Limit = 0,
    [switch]$SkipTranslate
)
$ErrorActionPreference = "Stop"
Set-Location (Split-Path $PSScriptRoot -Parent)

$qtrans = "target\release\qtrans.exe"
if (-not (Test-Path $qtrans)) { $qtrans = "target\debug\qtrans.exe" }

$qargs = @("run", $Zip, "--provider", $Provider, "--service", $Service)
if ($Limit -gt 0) { $qargs += @("--limit", "$Limit") }
if ($SkipTranslate) { $qargs += "--skip-translate" }

& $qtrans @args
