$ErrorActionPreference = "Stop"
$root = Split-Path $PSScriptRoot -Parent
Set-Location $root

$stage = Join-Path $root "target\installer-staging"
$app = Join-Path $stage "QTrans"
$output = Join-Path $root "dist\QTrans-Setup-x64.exe"
$iss = Join-Path $PSScriptRoot "qtrans.iss"
$workspaceManifest = Get-Content -Raw (Join-Path $root "Cargo.toml")
if ($workspaceManifest -notmatch '(?ms)^\[workspace\.package\]\s*.*?^version\s*=\s*"([^"]+)"') {
    throw "Unable to read workspace version from Cargo.toml."
}
$appVersion = $Matches[1]

function Find-InnoSetupCompiler {
    $command = Get-Command "ISCC.exe" -ErrorAction SilentlyContinue
    if ($command) {
        return $command.Source
    }

    $registryPaths = @(
        "HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall",
        "HKLM:\SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall",
        "HKCU:\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall"
    )
    $installLocations = foreach ($registryPath in $registryPaths) {
        Get-ChildItem $registryPath -ErrorAction SilentlyContinue | ForEach-Object {
            $entry = Get-ItemProperty $_.PSPath -ErrorAction SilentlyContinue
            if ($entry.DisplayName -like "Inno Setup*") {
                $entry.InstallLocation
            }
        }
    }

    $candidates = @(
        $installLocations | Where-Object { $_ } | ForEach-Object { Join-Path $_ "ISCC.exe" }
        "${env:ProgramFiles(x86)}\Inno Setup 7\ISCC.exe"
        "${env:ProgramFiles(x86)}\Inno Setup 6\ISCC.exe"
        "$env:ProgramFiles\Inno Setup 7\ISCC.exe"
        "$env:ProgramFiles\Inno Setup 6\ISCC.exe"
    )
    foreach ($candidate in $candidates) {
        if (Test-Path -LiteralPath $candidate -PathType Leaf) {
            return $candidate
        }
    }

    throw "Inno Setup compiler (ISCC.exe) not found. Add it to PATH or install Inno Setup."
}

$iscc = Find-InnoSetupCompiler

Write-Host "== Build Rust applications ==" -ForegroundColor Cyan
if (Get-Command "rtk" -ErrorAction SilentlyContinue) {
    rtk cargo build --release -p qtrans-cli -p qtrans-gui
} else {
    cargo build --release -p qtrans-cli -p qtrans-gui
}
if ($LASTEXITCODE -ne 0) { throw "Rust application build failed." }

Write-Host "== Build Go translation service ==" -ForegroundColor Cyan
New-Item -ItemType Directory -Force -Path (Join-Path $root "bin") | Out-Null
Push-Location (Join-Path $root "go\translator")
try {
    $goLdFlags = "-X main.version=$appVersion"
    if (Get-Command "rtk" -ErrorAction SilentlyContinue) {
        rtk go build -ldflags $goLdFlags -o (Join-Path $root "bin\qtrans-translator.exe") .
    } else {
        go build -ldflags $goLdFlags -o (Join-Path $root "bin\qtrans-translator.exe") .
    }
    if ($LASTEXITCODE -ne 0) { throw "Go translation service build failed." }
} finally {
    Pop-Location
}

if (Test-Path -LiteralPath $app) {
    Remove-Item -LiteralPath $app -Recurse -Force
}
New-Item -ItemType Directory -Force -Path $app | Out-Null
New-Item -ItemType Directory -Force -Path (Join-Path $app "bin") | Out-Null
Copy-Item "target\release\qtrans.exe" $app
Copy-Item "target\release\qtrans-gui.exe" $app
Copy-Item "bin\qtrans-translator.exe" (Join-Path $app "bin")
Copy-Item "README.md" $app
Copy-Item "assets" $app -Recurse

$brandDirectory = Join-Path $app "assets\brand"
$iconPath = Join-Path $brandDirectory "qtrans-icon.ico"
Add-Type -AssemblyName System.Drawing
Add-Type @"
using System.Runtime.InteropServices;
public static class QTransNativeIcon {
    [DllImport("user32.dll", SetLastError = true)]
    public static extern bool DestroyIcon(System.IntPtr handle);
}
"@
$bitmap = [System.Drawing.Bitmap]::new(
    64,
    64,
    [System.Drawing.Imaging.PixelFormat]::Format32bppArgb
)
$graphics = [System.Drawing.Graphics]::FromImage($bitmap)
$graphics.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::AntiAlias
$roundedTile = [System.Drawing.Drawing2D.GraphicsPath]::new()
$navy = [System.Drawing.SolidBrush]::new([System.Drawing.Color]::FromArgb(11, 16, 32))
$light = [System.Drawing.SolidBrush]::new([System.Drawing.Color]::FromArgb(241, 245, 249))
$mint = [System.Drawing.SolidBrush]::new([System.Drawing.Color]::FromArgb(73, 212, 157))
$iconHandle = [IntPtr]::Zero
$icon = $null
$stream = $null
try {
    $graphics.Clear([System.Drawing.Color]::White)
    $roundedTile.AddArc(2, 2, 16, 16, 180, 90)
    $roundedTile.AddArc(46, 2, 16, 16, 270, 90)
    $roundedTile.AddArc(46, 46, 16, 16, 0, 90)
    $roundedTile.AddArc(2, 46, 16, 16, 90, 90)
    $roundedTile.CloseFigure()
    $graphics.FillPath($navy, $roundedTile)
    foreach ($cell in @(
        "20,12", "28,12", "36,12",
        "14,20", "42,20",
        "14,28", "42,28",
        "20,36", "28,36"
    )) {
        $coordinates = $cell -split ","
        $graphics.FillRectangle($light, [int]$coordinates[0], [int]$coordinates[1], 8, 8)
    }
    $graphics.FillRectangle($mint, 36, 36, 8, 8)
    $graphics.FillRectangle($mint, 46, 46, 8, 8)
    $iconHandle = $bitmap.GetHicon()
    $icon = [System.Drawing.Icon]::FromHandle($iconHandle)
    $stream = [System.IO.File]::Create($iconPath)
    $icon.Save($stream)
    $stream.Flush()
    if ($stream.Length -eq 0) {
        throw "Generated QTrans icon is empty."
    }
} finally {
    if ($stream) { $stream.Dispose() }
    if ($icon) { $icon.Dispose() }
    if ($iconHandle -ne [IntPtr]::Zero) {
        [void][QTransNativeIcon]::DestroyIcon($iconHandle)
    }
    $mint.Dispose()
    $light.Dispose()
    $navy.Dispose()
    $roundedTile.Dispose()
    $graphics.Dispose()
    $bitmap.Dispose()
}

Get-ChildItem "target\release" -Filter "*.dll" -File | ForEach-Object {
    Copy-Item -LiteralPath $_.FullName -Destination $app
}

foreach ($directory in @("modpacks", "zip", "work\projects", "work\output", "work\cache")) {
    New-Item -ItemType Directory -Force -Path (Join-Path $app $directory) | Out-Null
}
Set-Content -Encoding utf8 -Path (Join-Path $app "modpacks\README.txt") `
    -Value "Place source modpack ZIP files here."
Set-Content -Encoding utf8 -Path (Join-Path $app "zip\README.txt") `
    -Value "Optional ZIP input and export folder."
Set-Content -Encoding utf8 -Path (Join-Path $app "work\README.txt") `
    -Value "QTrans projects, outputs, and translation cache are stored here."

Write-Host "== Compile Inno Setup installer ==" -ForegroundColor Cyan
New-Item -ItemType Directory -Force -Path (Split-Path $output -Parent) | Out-Null
$languageFile = Join-Path (Split-Path $iscc -Parent) "Languages\ChineseTraditional.isl"
$useChinese = if (Test-Path -LiteralPath $languageFile) { "1" } else { "0" }
& $iscc "/O$(Split-Path $output -Parent)" "/DAppVersion=$appVersion" "/DUseChinese=$useChinese" $iss
if ($LASTEXITCODE -ne 0) { throw "Inno Setup compilation failed." }
if (-not (Test-Path -LiteralPath $output -PathType Leaf)) {
    throw "Installer compilation completed but the expected output was not created: $output"
}
Write-Host "Created: $output" -ForegroundColor Green
