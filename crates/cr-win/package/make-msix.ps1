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
# WinUI's WebView2 loads it; the build put it next to the exe.
Copy-Item "$root\target\release\Microsoft.Web.WebView2.Core.dll" $layout
Copy-Item "$PSScriptRoot\Assets\*" "$layout\Assets"
$manifest = Get-Content "$PSScriptRoot\AppxManifest.xml" -Raw
$manifest.Replace('Version="0.0.0.0"', "Version=""$Version.0""") |
    Set-Content -NoNewline -Encoding utf8 "$layout\AppxManifest.xml"
function Find-Kit($name) {
    $tool = Get-ChildItem "${env:ProgramFiles(x86)}\Windows Kits\10\bin\*\x64\$name" |
        Sort-Object FullName -Descending | Select-Object -First 1
    if (-not $tool) { throw "$name not found; install the Windows SDK" }
    $tool.FullName
}
$makeappx = Find-Kit makeappx.exe
# The logos come at 100% and 200% scale; resources.pri tells Windows (and
# the Store's certification) which file a scale takes for each name in the
# manifest.
$makepri = Find-Kit makepri.exe
$config = Join-Path $env:TEMP "corerules-priconfig.xml"
& $makepri createconfig /cf $config /dq en-US /pv 10.0.0 /o | Out-Null
if ($LASTEXITCODE -ne 0) { throw "makepri createconfig failed" }
& $makepri new /pr $layout /cf $config /mn "$layout\AppxManifest.xml" /of "$layout\resources.pri" /o
if ($LASTEXITCODE -ne 0) { throw "makepri failed" }
New-Item -ItemType Directory -Force $Out | Out-Null
$package = Join-Path $Out "CoreRules_${Version}_x64.msix"
& $makeappx pack /d $layout /p $package /o
if ($LASTEXITCODE -ne 0) { throw "makeappx failed" }
Write-Output $package
