# Verify a translated modpack build:
#  - counts strings vs translations
#  - lists remaining untranslated entries
#  - inspects the produced ZIP (zh_tw files, replaced snbt)
param(
    [Parameter(Mandatory = $true)][string]$Project
)
$ErrorActionPreference = "Stop"
Set-Location (Split-Path $PSScriptRoot -Parent)

$qtrans = "target\release\qtrans.exe"
if (-not (Test-Path $qtrans)) { $qtrans = "target\debug\qtrans.exe" }

& $qtrans status --project $Project

$dir = Resolve-Path $Project
$zip = Get-ChildItem (Join-Path $dir "output") -Filter "*-zh_tw.zip" -ErrorAction SilentlyContinue |
    Sort-Object LastWriteTime -Descending | Select-Object -First 1

if (-not $zip) {
    Write-Host "no output zip yet: run 'qtrans build --project $Project'" -ForegroundColor Yellow
    return
}

Add-Type -AssemblyName System.IO.Compression.FileSystem
$archive = [System.IO.Compression.ZipFile]::OpenRead($zip.FullName)
try {
    $lang = $archive.Entries | Where-Object { $_.FullName -match '/lang/zh_tw\.json$' }
    $snbt = $archive.Entries | Where-Object { $_.FullName -match '\.snbt$' }
    Write-Host ""
    Write-Host ("zip        : {0} ({1:N1} MB)" -f $zip.Name, ($zip.Length / 1MB))
    Write-Host ("entries    : {0}" -f $archive.Entries.Count)
    Write-Host ("zh_tw files: {0}" -f $lang.Count)
    foreach ($entry in $lang) { Write-Host ("  {0} ({1:N0} bytes)" -f $entry.FullName, $entry.Length) }
    Write-Host ("snbt files : {0}" -f $snbt.Count)
} finally {
    $archive.Dispose()
}

$untranslated = Join-Path $dir "untranslated.csv"
if (Test-Path $untranslated) {
    $rows = (Get-Content $untranslated | Measure-Object -Line).Lines - 1
    Write-Host ("untranslated: {0} (see {1})" -f $rows, $untranslated)
}

$manifestPath = Join-Path $dir "manifest.json"
if (Test-Path $manifestPath) {
    $manifest = Get-Content $manifestPath -Raw | ConvertFrom-Json
    $changedScripts = @($manifest.changed | Where-Object {
        $_ -match '^kubejs/(client_scripts|server_scripts|startup_scripts)/.+\.js$'
    })
    Write-Host ("changed KubeJS scripts: {0}" -f $changedScripts.Count)
    foreach ($script in $changedScripts) { Write-Host ("  {0}" -f $script) }
}
