[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [string]$ArchivePath,
    [Parameter(Mandatory = $true)]
    [string]$Tag,
    [ValidateSet('x64', 'arm64')][string]$Architecture = 'x64',
    [Parameter(Mandatory = $true)]
    [string]$ExpectedBrokerPublisher,
    [string]$PubspecPath,
    [string]$CargoManifestPath
)

$ErrorActionPreference = "Stop"
. (Join-Path $PSScriptRoot "quality_common.ps1")
. (Join-Path $PSScriptRoot "release_journal_broker_common.ps1")
. (Join-Path $PSScriptRoot "quality_windows_pe_image.ps1")

if ([string]::IsNullOrWhiteSpace($ExpectedBrokerPublisher)) {
    throw "Portable signature verification requires the exact expected publisher"
}

& (Join-Path $PSScriptRoot "release_verify_portable_archive.ps1") `
    -ArchivePath $ArchivePath `
    -Tag $Tag `
    -Architecture $Architecture `
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
    $payloadRoot = Join-Path $scratchRoot "Cedarflake-Ame/current"
    $applicationPath = Join-Path $payloadRoot $script:AmeApplicationBinaryName
    $brokerPath = Join-Path $payloadRoot (Get-AmeBrokerServicePlan).BinaryName
    foreach ($target in @($applicationPath, $brokerPath)) {
        Assert-AmeWindowsPeImage -Path $target -Architecture $Architecture
        $signature = Get-AuthenticodeSignature -LiteralPath $target
        $publisher = if ($null -eq $signature.SignerCertificate) { $null } else { $signature.SignerCertificate.Subject }
        Assert-AmeBrokerSignatureFacts -Status ([string]$signature.Status) -SignerSubject $publisher `
            -ExpectedPublisher $ExpectedBrokerPublisher
    }
    Get-AmeBrokerProtocolFacts -Path $brokerPath | Out-Null
} finally {
    if (Test-Path -LiteralPath $scratchRoot) {
        Remove-Item -LiteralPath $scratchRoot -Recurse -Force
    }
    if (Test-Path -LiteralPath $scratchRoot) {
        throw "Portable signature scratch directory still exists after cleanup"
    }
}

Write-Output "portable_signatures_verified"
