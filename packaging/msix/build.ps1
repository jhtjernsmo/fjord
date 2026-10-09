# Builds the unsigned MSIX for the Microsoft Store from a built Fjord.exe.
#   pwsh packaging/msix/build.ps1 -Exe target/release/fjord-app.exe -Version 0.2.13 -Out Fjord_0.2.13_x64.msix
# Needs makeappx.exe from the Windows SDK (installed on GitHub's windows runners).
param(
  [Parameter(Mandatory)] [string] $Exe,
  [Parameter(Mandatory)] [string] $Version,
  [Parameter(Mandatory)] [string] $Out
)
$ErrorActionPreference = 'Stop'
$here = $PSScriptRoot
$root = Resolve-Path "$here/../.."
$identity = Get-Content "$here/identity.json" -Raw | ConvertFrom-Json
if ($identity.identityName -eq 'TODO') { throw 'Fill in packaging/msix/identity.json from Partner Center first.' }
if ($Version -notmatch '^\d+\.\d+\.\d+$') { throw "Version must look like 1.2.3, got '$Version'." }

$stage = Join-Path ([System.IO.Path]::GetTempPath()) "fjord-msix-$([guid]::NewGuid())"
New-Item -ItemType Directory -Path "$stage/Assets" | Out-Null
Copy-Item $Exe "$stage/Fjord.exe"
foreach ($icon in 'StoreLogo.png', 'Square44x44Logo.png', 'Square150x150Logo.png', 'Square310x310Logo.png') {
  Copy-Item "$root/src-tauri/icons/$icon" "$stage/Assets/$icon"
}
# The Store wants a four-part version whose last part is 0.
(Get-Content "$here/AppxManifest.xml" -Raw).
  Replace('{{IDENTITY_NAME}}', $identity.identityName).
  Replace('{{PUBLISHER}}', $identity.publisher).
  Replace('{{PUBLISHER_DISPLAY_NAME}}', $identity.publisherDisplayName).
  Replace('{{VERSION}}', "$Version.0") |
  Set-Content "$stage/AppxManifest.xml" -Encoding utf8

$makeappx = Get-ChildItem "${env:ProgramFiles(x86)}\Windows Kits\10\bin\*\x64\makeappx.exe" |
  Sort-Object FullName -Descending | Select-Object -First 1
if (-not $makeappx) { throw 'makeappx.exe not found; install the Windows SDK.' }
& $makeappx.FullName pack /o /d $stage /p $Out
if ($LASTEXITCODE -ne 0) { throw "makeappx failed ($LASTEXITCODE)" }
Remove-Item -Recurse -Force $stage
Write-Host "Built $Out"
