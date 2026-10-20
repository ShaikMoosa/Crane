param(
    [string]$ArtifactPath,
    [string]$ChecksumPath,
    [switch]$SelfTest
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

function Test-WindowsArtifact {
    param(
        [Parameter(Mandatory = $true)][string]$Path,
        [Parameter(Mandatory = $true)][string]$Sha256Path
    )

    if (-not (Test-Path -LiteralPath $Path -PathType Leaf)) {
        throw "Windows artifact does not exist: $Path"
    }
    $artifact = Get-Item -LiteralPath $Path
    if ($artifact.Length -le 0) {
        throw "Windows artifact is empty: $Path"
    }
    if (-not (Test-Path -LiteralPath $Sha256Path -PathType Leaf)) {
        throw "Checksum file does not exist: $Sha256Path"
    }
    $checksumText = (Get-Content -LiteralPath $Sha256Path -Raw).Trim()
    $expected = ($checksumText -split '\s+')[0]
    if ($expected -notmatch '^[0-9A-Fa-f]{64}$') {
        throw "Checksum file does not start with a SHA-256 value: $Sha256Path"
    }
    $actual = (Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash
    if ($actual -ne $expected.ToUpperInvariant()) {
        throw "SHA-256 mismatch for $Path"
    }
    [pscustomobject]@{
        Path = $artifact.FullName
        Bytes = $artifact.Length
        Sha256 = $actual
    }
}

function Assert-Fails {
    param([scriptblock]$Action, [string]$Label)
    try {
        & $Action | Out-Null
    }
    catch {
        return
    }
    throw "Self-test expected failure: $Label"
}

if ($SelfTest) {
    $testRoot = Join-Path ([System.IO.Path]::GetTempPath()) ("crane-smoke-selftest-" + [guid]::NewGuid())
    New-Item -ItemType Directory -Path $testRoot | Out-Null
    try {
        $missing = Join-Path $testRoot 'missing.exe'
        $checksum = Join-Path $testRoot 'artifact.sha256'
        Set-Content -LiteralPath $checksum -Value ('0' * 64) -NoNewline
        Assert-Fails { Test-WindowsArtifact -Path $missing -Sha256Path $checksum } 'missing artifact'

        $artifact = Join-Path $testRoot 'artifact.exe'
        [System.IO.File]::WriteAllBytes($artifact, [byte[]]@())
        Assert-Fails { Test-WindowsArtifact -Path $artifact -Sha256Path $checksum } 'zero-byte artifact'

        [System.IO.File]::WriteAllBytes($artifact, [byte[]](1, 2, 3, 4))
        Assert-Fails { Test-WindowsArtifact -Path $artifact -Sha256Path $checksum } 'unexpected checksum'

        $actual = (Get-FileHash -LiteralPath $artifact -Algorithm SHA256).Hash
        Set-Content -LiteralPath $checksum -Value "$actual  artifact.exe" -NoNewline
        Test-WindowsArtifact -Path $artifact -Sha256Path $checksum | Out-Null
        Write-Host 'smoke-windows self-test passed'
    }
    finally {
        Remove-Item -LiteralPath $testRoot -Recurse -Force
    }
    exit 0
}

if ([string]::IsNullOrWhiteSpace($ArtifactPath) -or [string]::IsNullOrWhiteSpace($ChecksumPath)) {
    throw 'Provide -ArtifactPath and -ChecksumPath.'
}
Test-WindowsArtifact -Path $ArtifactPath -Sha256Path $ChecksumPath
