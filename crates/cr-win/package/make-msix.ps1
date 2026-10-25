# Lays out the MSIX package of the Windows app from a release build
# (cargo build --release -p cr-win) and packs it with makeappx from the
# Windows SDK. The package is not signed: the Microsoft Store signs it, and
# a developer installs the layout with
#   Add-AppxPackage -Register target\msix\AppxManifest.xml
# (Developer Mode on).
param(
    # The version, as in Cargo.toml; the Store wants the fourth part 0.
    [Parameter(Mandatory)] [string] $Version,
    [string] $Out = "target"
)
$ErrorActionPreference = "Stop"
$root = Resolve-Path "$PSScriptRoot\..\..\.."
$layout = Join-Path $root "target\msix"
if (Test-Path $layout) { Remove-Item -Recurse -Force $layout }
New-Item -ItemType Directory "$layout\Assets" | Out-Null
Copy-Item "$root\target\release\CoreRules.exe" $layout
Copy-Item "$PSScriptRoot\Assets\*" "$layout\Assets"
$manifest = Get-Content "$PSScriptRoot\AppxManifest.xml" -Raw
$manifest.Replace('Version="0.0.0.0"', "Version=""$Version.0""") |
    Set-Content -NoNewline -Encoding utf8 "$layout\AppxManifest.xml"
$makeappx = Get-ChildItem "${env:ProgramFiles(x86)}\Windows Kits\10\bin\*\x64\makeappx.exe" |
    Sort-Object FullName -Descending | Select-Object -First 1
if (-not $makeappx) { throw "makeappx.exe not found; install the Windows SDK" }
New-Item -ItemType Directory -Force $Out | Out-Null
$package = Join-Path $Out "CoreRules_${Version}_x64.msix"
& $makeappx.FullName pack /d $layout /p $package /o
if ($LASTEXITCODE -ne 0) { throw "makeappx failed" }
Write-Output $package
