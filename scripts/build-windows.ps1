param([switch]$SkipInstaller)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$repoRoot = Join-Path $PSScriptRoot '..'
Set-Location $repoRoot

& powershell -NoProfile -File (Join-Path $PSScriptRoot 'check-layers.ps1')
if ($LASTEXITCODE -ne 0) { throw 'Dependency layer check failed.' }

& cargo build --release --locked -p crane-desktop
if ($LASTEXITCODE -ne 0) { throw 'Native release build failed.' }

$exe = Join-Path $repoRoot 'target\release\crane.exe'
if (-not (Test-Path -LiteralPath $exe -PathType Leaf)) { throw "Release binary missing: $exe" }
if ((Get-Item -LiteralPath $exe).Length -le 0) { throw "Release binary is empty: $exe" }

$artifactDir = Join-Path $repoRoot 'artifacts\windows'
New-Item -ItemType Directory -Path $artifactDir -Force | Out-Null
$stagedExe = Join-Path $artifactDir 'crane.exe'
Copy-Item -LiteralPath $exe -Destination $stagedExe -Force
$exeHash = (Get-FileHash -LiteralPath $stagedExe -Algorithm SHA256).Hash
Set-Content -LiteralPath (Join-Path $artifactDir 'crane.exe.sha256') -Value "$exeHash  crane.exe" -NoNewline
& powershell -NoProfile -File (Join-Path $PSScriptRoot 'smoke-windows.ps1') -ArtifactPath $stagedExe -ChecksumPath (Join-Path $artifactDir 'crane.exe.sha256')
if ($LASTEXITCODE -ne 0) { throw 'Release binary smoke check failed.' }
Write-Host "Verified native binary: $stagedExe ($exeHash)"

if ($SkipInstaller) {
    Write-Host 'Installer skipped by request; only the verified binary artifact was produced.'
    exit 0
}

$makensis = Get-Command makensis -ErrorAction SilentlyContinue
if (-not $makensis) {
    Write-Host 'NSIS (makensis) was not found. Install NSIS, then re-run without -SkipInstaller to produce Crane_0.3.0_x64-setup.exe. The verified crane.exe artifact above is ready; no installer was built.'
    exit 0
}

$script = Join-Path $repoRoot 'packaging\windows\crane.nsi'
& $makensis.Source $script
if ($LASTEXITCODE -ne 0) { throw 'NSIS installer build failed.' }

$installer = Join-Path $artifactDir 'Crane_0.3.0_x64-setup.exe'
if (-not (Test-Path -LiteralPath $installer -PathType Leaf)) { throw "Installer missing: $installer" }
$installerHash = (Get-FileHash -LiteralPath $installer -Algorithm SHA256).Hash
Set-Content -LiteralPath (Join-Path $artifactDir 'Crane_0.3.0_x64-setup.exe.sha256') -Value "$installerHash  Crane_0.3.0_x64-setup.exe" -NoNewline
& powershell -NoProfile -File (Join-Path $PSScriptRoot 'smoke-windows.ps1') -ArtifactPath $installer -ChecksumPath (Join-Path $artifactDir 'Crane_0.3.0_x64-setup.exe.sha256')
if ($LASTEXITCODE -ne 0) { throw 'Installer smoke check failed.' }
Set-Content -LiteralPath (Join-Path $artifactDir 'build-provenance.txt') -Value "crane 0.3.0`nrustc: $(rustc --version)`ncargo: $(cargo --version)`ninstaller-sha256: $installerHash`nbinary-sha256: $exeHash" -NoNewline
Write-Host "Verified installer: $installer ($installerHash)"
