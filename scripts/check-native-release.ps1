param([switch]$SelfTest)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

# Fails if the resolved native build still carries the legacy web stack.
# Evidence sources: Cargo metadata (dependencies + resolved features),
# git-tracked paths, build hooks, and remote-asset references in code dirs.
# Historical text under docs/ and dated specs/plans is never scanned.

$script:ForbiddenRuntime = @(
    'tauri', 'tauri-build', 'tauri-runtime', 'tauri-runtime-wry',
    'wry', 'webview2-com', 'webview2-com-sys'
)

function Test-DependencyAppliesToWindows {
    param([AllowNull()][object]$Target)
    if ($null -eq $Target) { return $true }
    $text = [string]$Target
    if ([string]::IsNullOrWhiteSpace($text)) { return $true }
    return $text -match 'windows'
}

function Test-WorkspaceDependencies {
    param([Parameter(Mandatory = $true)][object[]]$Packages)

    $violations = [System.Collections.Generic.List[string]]::new()
    foreach ($package in $Packages) {
        foreach ($dependency in $package.dependencies) {
            if (-not (Test-DependencyAppliesToWindows $dependency.target)) { continue }
            if ($script:ForbiddenRuntime -contains $dependency.name) {
                $display = if ($dependency.rename) { "$($dependency.rename) (package $($dependency.name))" } else { $dependency.name }
                $violations.Add("$($package.name) depends on forbidden web runtime $display")
            }
            if ($dependency.name -in @('eframe', 'egui')) {
                foreach ($feature in @($dependency.features)) {
                    if ($feature -match 'inspection') {
                        $violations.Add("$($package.name) enables the optional inspection feature '$feature' on $($dependency.name)")
                    }
                }
            }
        }
    }
    return $violations.ToArray()
}

function Test-ResolvedFeatures {
    param(
        [Parameter(Mandatory = $true)][hashtable]$NameById,
        [Parameter(Mandatory = $true)][object[]]$Nodes
    )

    $violations = [System.Collections.Generic.List[string]]::new()
    foreach ($node in $Nodes) {
        $name = $NameById[$node.id]
        if ($name -notin @('eframe', 'egui')) { continue }
        foreach ($feature in @($node.features)) {
            if ($feature -match 'inspection') {
                $violations.Add("resolved $name feature set includes the optional inspection feature '$feature'")
            }
        }
    }
    return $violations.ToArray()
}

function Test-TrackedPaths {
    param([Parameter(Mandatory = $true)][AllowEmptyCollection()][string[]]$Paths)

    $violations = [System.Collections.Generic.List[string]]::new()
    foreach ($path in $Paths) {
        $normalized = $path -replace '\\', '/'
        if ($normalized -eq 'src-tauri' -or $normalized.StartsWith('src-tauri/')) {
            $violations.Add("tracked legacy Tauri path remains: $normalized")
        } elseif ($normalized -eq 'ui' -or $normalized.StartsWith('ui/')) {
            $violations.Add("tracked legacy web UI path remains: $normalized")
        } elseif ($normalized -eq 'tests' -or $normalized.StartsWith('tests/')) {
            $violations.Add("tracked legacy JavaScript test path remains: $normalized")
        } elseif ($normalized -match '(^|/)package(-lock)?\.json$') {
            $violations.Add("tracked Node manifest remains: $normalized")
        } elseif ($normalized -match '(^|/)node_modules/') {
            $violations.Add("tracked Node modules remain: $normalized")
        }
    }
    return $violations.ToArray()
}

function Test-RemoteAssets {
    param(
        [Parameter(Mandatory = $true)][string]$RepoRoot,
        [Parameter(Mandatory = $true)][AllowEmptyCollection()][string[]]$Paths
    )

    $violations = [System.Collections.Generic.List[string]]::new()
    foreach ($path in $Paths) {
        $normalized = $path -replace '\\', '/'
        if ($normalized -notmatch '^(apps|crates|packaging|scripts)/') { continue }
        if ($normalized -notmatch '\.(html|htm|css)$') { continue }
        $full = Join-Path $RepoRoot $normalized
        if (-not (Test-Path -LiteralPath $full -PathType Leaf)) { continue }
        $content = Get-Content -LiteralPath $full -Raw
        if ($content -match '(src|href)\s*=\s*["'']https?://' -or $content -match 'url\(\s*["'']?https?://') {
            $violations.Add("remote frontend asset referenced by tracked file: $normalized")
        }
    }
    return $violations.ToArray()
}

function Test-BuildHooks {
    param(
        [Parameter(Mandatory = $true)][string]$RepoRoot,
        [Parameter(Mandatory = $true)][AllowEmptyCollection()][string[]]$Paths
    )

    $violations = [System.Collections.Generic.List[string]]::new()
    foreach ($path in $Paths) {
        $normalized = $path -replace '\\', '/'
        $full = Join-Path $RepoRoot $normalized
        if (-not (Test-Path -LiteralPath $full -PathType Leaf)) { continue }
        if ($normalized -match '(^|/)build\.rs$') {
            $content = Get-Content -LiteralPath $full -Raw
            if ($content -match '\b(npm|npx|node|yarn|pnpm)\b') {
                $violations.Add("Node build hook in tracked build script: $normalized")
            }
        } elseif ($normalized -match '^\.github/workflows/.*\.ya?ml$') {
            $content = Get-Content -LiteralPath $full -Raw
            if ($content -match 'setup-node' -or $content -match '\b(npm|npx|yarn|pnpm)\s') {
                $violations.Add("Node step in tracked CI workflow: $normalized")
            }
        }
    }
    return $violations.ToArray()
}

function Test-WindowsPackagingScripts {
    param(
        [Parameter(Mandatory = $true)][string]$BuildScriptContent,
        [Parameter(Mandatory = $true)][string]$InstallerScriptContent
    )

    $violations = [System.Collections.Generic.List[string]]::new()
    if ($BuildScriptContent -match "target[\\\\/]release[\\\\/]crane\.exe") {
        $violations.Add('Windows build script hard-codes target/release/crane.exe instead of Cargo metadata target_directory')
    }
    if ($BuildScriptContent -notmatch '\.target_directory') {
        $violations.Add('Windows build script does not resolve Cargo target_directory')
    }
    if ($InstallerScriptContent -match "target[\\\\/]release") {
        $violations.Add('Windows installer packages an unstaged target/release binary')
    }
    if ($InstallerScriptContent -notmatch "artifacts[\\\\/]windows[\\\\/]\$\{EXE_NAME\}") {
        $violations.Add('Windows installer does not package the verified staged binary')
    }
    if ($InstallerScriptContent -match 'StrCmp \$1 "" running_check_done') {
        $violations.Add('Windows installer treats any tasklist output as a running Crane process')
    }
    if ($InstallerScriptContent -match '\$\{StrStr\}' -and $InstallerScriptContent -notmatch '\$\{Using:StrFunc\}\s+StrStr') {
        $violations.Add('Windows installer uses StrStr without enabling the NSIS StrFunc macro')
    }
    return $violations.ToArray()
}

function Get-TrackedPaths {
    param([Parameter(Mandatory = $true)][string]$RepoRoot)
    $lines = & git -C $RepoRoot ls-files 2>&1
    if ($LASTEXITCODE -ne 0) { throw "git ls-files failed:`n$($lines -join "`n")" }
    return @($lines | ForEach-Object { [string]$_ })
}

function Get-CargoMetadata {
    param([Parameter(Mandatory = $true)][string]$ManifestPath)
    # Full metadata resolves every platform's graph and may need registry
    # downloads; fall back to the workspace-only view (same approach as
    # check-layers.ps1) and still check every declared dependency/feature.
    $previousPreference = $ErrorActionPreference
    $ErrorActionPreference = 'Continue'
    try {
        $json = & cargo metadata --format-version 1 --locked --manifest-path $ManifestPath 2>$null
        $fullExit = $LASTEXITCODE
        if ($fullExit -eq 0) { return (($json -join "`n") | ConvertFrom-Json) }
        Write-Warning 'Full cargo metadata unavailable; checking declared workspace dependencies and features only.'
        $json = & cargo metadata --format-version 1 --no-deps --locked --manifest-path $ManifestPath 2>$null
        $fallbackExit = $LASTEXITCODE
    }
    finally {
        $ErrorActionPreference = $previousPreference
    }
    if ($fallbackExit -ne 0) { throw 'cargo metadata failed for the native release check.' }
    return ($json -join "`n") | ConvertFrom-Json
}

function Test-NativeRelease {
    param([Parameter(Mandatory = $true)][string]$RepoRoot)

    $violations = [System.Collections.Generic.List[string]]::new()
    $tracked = Get-TrackedPaths -RepoRoot $RepoRoot
    foreach ($item in (Test-TrackedPaths -Paths $tracked)) { $violations.Add($item) }
    foreach ($item in (Test-RemoteAssets -RepoRoot $RepoRoot -Paths $tracked)) { $violations.Add($item) }
    foreach ($item in (Test-BuildHooks -RepoRoot $RepoRoot -Paths $tracked)) { $violations.Add($item) }

    $buildScriptContent = Get-Content -LiteralPath (Join-Path $RepoRoot 'scripts/build-windows.ps1') -Raw
    $installerScriptContent = Get-Content -LiteralPath (Join-Path $RepoRoot 'packaging/windows/crane.nsi') -Raw
    foreach ($item in (Test-WindowsPackagingScripts -BuildScriptContent $buildScriptContent -InstallerScriptContent $installerScriptContent)) { $violations.Add($item) }

    $metadata = Get-CargoMetadata -ManifestPath (Join-Path $RepoRoot 'Cargo.toml')
    $workspaceIds = @{}
    foreach ($id in $metadata.workspace_members) { $workspaceIds[$id] = $true }
    $workspacePackages = @($metadata.packages | Where-Object { $workspaceIds.ContainsKey($_.id) })
    foreach ($item in (Test-WorkspaceDependencies -Packages $workspacePackages)) { $violations.Add($item) }

    if ($null -ne $metadata.resolve -and $null -ne $metadata.resolve.nodes) {
        $nameById = @{}
        foreach ($package in $metadata.packages) { $nameById[$package.id] = $package.name }
        foreach ($item in (Test-ResolvedFeatures -NameById $nameById -Nodes @($metadata.resolve.nodes))) { $violations.Add($item) }
    }

    if ($violations.Count -gt 0) {
        throw ("Native release check failed:`n" + ($violations -join "`n"))
    }
    Write-Host 'Crane native release checks are valid.'
}

function Assert-Fails {
    param([scriptblock]$Action, [string]$ExpectedText)
    try {
        & $Action | Out-Null
    }
    catch {
        if ($_.Exception.Message -notmatch [regex]::Escape($ExpectedText)) {
            throw "Expected '$ExpectedText' but got: $($_.Exception.Message)"
        }
        return
    }
    throw "Self-test expected failure containing '$ExpectedText'."
}

function New-Dependency {
    param([string]$Name, [string]$Rename = '', [object]$Target = $null, [string[]]$Features = @())
    [pscustomobject]@{ name = $Name; rename = $Rename; target = $Target; features = $Features; kind = 'normal'; optional = $false; uses_default_features = $true; req = '*' }
}

function New-Package {
    param([string]$Name, [object[]]$Dependencies)
    [pscustomobject]@{ id = "path+file:///fixture#$Name@0.1.0"; name = $Name; dependencies = $Dependencies }
}

if ($SelfTest) {
    # Tracked-path rules: clean native paths pass, each legacy class fails.
    Test-TrackedPaths -Paths @('apps/desktop/src/main.rs', 'crates/domain/src/lib.rs', 'docs/migration/behavior-parity.md') | Out-Null
    Assert-Fails { throw ((Test-TrackedPaths -Paths @('ui/index.html')) -join "`n") } 'tracked legacy web UI path remains: ui/index.html'
    Assert-Fails { throw ((Test-TrackedPaths -Paths @('src-tauri/Cargo.toml')) -join "`n") } 'tracked legacy Tauri path remains: src-tauri/Cargo.toml'
    Assert-Fails { throw ((Test-TrackedPaths -Paths @('tests/core.test.mjs')) -join "`n") } 'tracked legacy JavaScript test path remains: tests/core.test.mjs'
    Assert-Fails { throw ((Test-TrackedPaths -Paths @('package.json')) -join "`n") } 'tracked Node manifest remains: package.json'

    # Dependency rules: unconditional and Windows-targeted web runtimes fail,
    # renamed packages are reported, non-Windows conditionals are accepted.
    $clean = @((New-Package 'crane-domain' @()), (New-Package 'crane-desktop' @((New-Dependency 'eframe' -Features @('accesskit', 'wgpu')))))
    Test-WorkspaceDependencies -Packages $clean | Out-Null
    Assert-Fails { throw ((Test-WorkspaceDependencies -Packages @((New-Package 'crane-domain' @((New-Dependency 'tauri'))))) -join "`n") } 'depends on forbidden web runtime tauri'
    Assert-Fails { throw ((Test-WorkspaceDependencies -Packages @((New-Package 'crane-domain' @((New-Dependency 'tauri' -Rename 'tauri-alias'))))) -join "`n") } 'tauri-alias (package tauri)'
    Assert-Fails { throw ((Test-WorkspaceDependencies -Packages @((New-Package 'crane-domain' @((New-Dependency 'wry' -Target 'cfg(windows)'))))) -join "`n") } 'depends on forbidden web runtime wry'
    Test-WorkspaceDependencies -Packages @((New-Package 'crane-domain' @((New-Dependency 'wry' -Target 'cfg(target_os = "macos")')))) | Out-Null
    Assert-Fails { throw ((Test-WorkspaceDependencies -Packages @((New-Package 'crane-desktop' @((New-Dependency 'eframe' -Features @('inspection')))))) -join "`n") } 'optional inspection feature'

    # Resolved-feature rule: applies to resolved eframe/egui nodes only.
    $names = @{ 'id-egui' = 'egui'; 'id-other' = 'serde' }
    $nodes = @(
        [pscustomobject]@{ id = 'id-egui'; features = @('default', 'inspection') },
        [pscustomobject]@{ id = 'id-other'; features = @('inspection') }
    )
    Assert-Fails { throw ((Test-ResolvedFeatures -NameById $names -Nodes $nodes) -join "`n") } 'resolved egui feature set includes'

    # Windows packaging must consume Cargo's actual target directory, package
    # the verified staged binary, and distinguish tasklist's no-match text.
    $cleanBuildScript = '$metadata.target_directory; Join-Path $targetDirectory ''release\crane.exe'''
    $cleanInstallerScript = '${Using:StrFunc} StrStr; ${StrStr} $2 $1 ''"crane.exe"''; File "..\..\artifacts\windows\${EXE_NAME}"'
    Test-WindowsPackagingScripts -BuildScriptContent $cleanBuildScript -InstallerScriptContent $cleanInstallerScript | Out-Null
    Assert-Fails {
        throw ((Test-WindowsPackagingScripts -BuildScriptContent 'Join-Path $repoRoot ''target\release\crane.exe''' -InstallerScriptContent 'File "..\..\target\release\${EXE_NAME}"; StrCmp $1 "" running_check_done') -join "`n")
    } 'hard-codes target/release/crane.exe'
    Assert-Fails {
        throw ((Test-WindowsPackagingScripts -BuildScriptContent $cleanBuildScript -InstallerScriptContent '${StrStr} $2 $1 ''"crane.exe"''; File "..\..\artifacts\windows\${EXE_NAME}"') -join "`n")
    } 'uses StrStr without enabling the NSIS StrFunc macro'

    # Remote-asset and build-hook rules run against isolated fixture files.
    $testRoot = Join-Path ([System.IO.Path]::GetTempPath()) ("crane-release-selftest-" + [guid]::NewGuid())
    New-Item -ItemType Directory -Path (Join-Path $testRoot 'apps/desktop') -Force | Out-Null
    New-Item -ItemType Directory -Path (Join-Path $testRoot '.github/workflows') -Force | Out-Null
    try {
        Set-Content -LiteralPath (Join-Path $testRoot 'apps/desktop/local.html') -Value '<script src="app.js"></script>' -NoNewline
        Test-RemoteAssets -RepoRoot $testRoot -Paths @('apps/desktop/local.html') | Out-Null
        Set-Content -LiteralPath (Join-Path $testRoot 'apps/desktop/remote.html') -Value '<script src="https://example.invalid/app.js"></script>' -NoNewline
        Assert-Fails { throw ((Test-RemoteAssets -RepoRoot $testRoot -Paths @('apps/desktop/remote.html')) -join "`n") } 'remote frontend asset referenced by tracked file: apps/desktop/remote.html'
        Set-Content -LiteralPath (Join-Path $testRoot '.github/workflows/ci.yml') -Value 'steps: []' -NoNewline
        Test-BuildHooks -RepoRoot $testRoot -Paths @('.github/workflows/ci.yml') | Out-Null
        Set-Content -LiteralPath (Join-Path $testRoot '.github/workflows/node.yml') -Value 'steps: [{ uses: actions/setup-node@v4 }]' -NoNewline
        Assert-Fails { throw ((Test-BuildHooks -RepoRoot $testRoot -Paths @('.github/workflows/node.yml')) -join "`n") } 'Node step in tracked CI workflow: .github/workflows/node.yml'
    }
    finally {
        Remove-Item -LiteralPath $testRoot -Recurse -Force
    }
    Write-Host 'check-native-release self-test passed'
    exit 0
}

Test-NativeRelease -RepoRoot (Join-Path $PSScriptRoot '..')
