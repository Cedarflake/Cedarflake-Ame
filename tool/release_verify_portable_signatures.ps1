[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [string]$ArchivePath,
    [Parameter(Mandatory = $true)]
    [string]$Tag,
    [Parameter(Mandatory = $true)]
    [string]$ExpectedBrokerPublisher,
    [string]$PubspecPath,
    [string]$CargoManifestPath
)

$ErrorActionPreference = "Stop"
. (Join-Path $PSScriptRoot "quality_common.ps1")
. (Join-Path $PSScriptRoot "release_journal_broker_common.ps1")

if ([string]::IsNullOrWhiteSpace($ExpectedBrokerPublisher)) {
    throw "Portable signature verification requires the exact expected publisher"
}

& (Join-Path $PSScriptRoot "release_verify_portable_archive.ps1") `
    -ArchivePath $ArchivePath `
    -Tag $Tag `
    -PubspecPath $PubspecPath `
    -CargoManifestPath $CargoManifestPath

$repositoryRoot = Get-AmeRepositoryRoot
$buildRoot = [System.IO.Path]::GetFullPath((Join-Path $repositoryRoot "build"))
$scratchRoot = [System.IO.Path]::GetFullPath((Join-Path `
    $buildRoot `
    "portable-signature-verify-$PID-$([Guid]::NewGuid().ToString('N'))"))
$buildPrefix = "$buildRoot$([System.IO.Path]::DirectorySeparatorChar)"
if (-not $scratchRoot.StartsWith(
    $buildPrefix,
    [System.StringComparison]::OrdinalIgnoreCase
)) {
    throw "Portable signature scratch directory must remain inside the repository build directory"
}
if (Test-Path -LiteralPath $scratchRoot) {
    throw "Portable signature scratch directory must be fresh"
}

try {
    New-Item -ItemType Directory -Path $scratchRoot | Out-Null
    Add-Type -AssemblyName System.IO.Compression.FileSystem
    [System.IO.Compression.ZipFile]::ExtractToDirectory(
        [System.IO.Path]::GetFullPath($ArchivePath),
        $scratchRoot
    )
    $payloadRoot = Join-Path $scratchRoot "Cedarflake-Ame"
    $applicationPath = Join-Path $payloadRoot $script:AmeApplicationBinaryName
    $brokerPath = Join-Path $payloadRoot (Get-AmeBrokerServicePlan).BinaryName
    Assert-AmeSignedX64Binary `
        -Path $applicationPath `
        -ExpectedPublisher $ExpectedBrokerPublisher | Out-Null
    Assert-AmeBrokerBinary `
        -Path $brokerPath `
        -ExpectedPublisher $ExpectedBrokerPublisher | Out-Null
} finally {
    if (Test-Path -LiteralPath $scratchRoot) {
        Remove-Item -LiteralPath $scratchRoot -Recurse -Force
    }
    if (Test-Path -LiteralPath $scratchRoot) {
        throw "Portable signature scratch directory still exists after cleanup"
    }
}

Write-Output "portable_signatures_verified"
