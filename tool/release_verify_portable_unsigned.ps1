[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [string]$ArchivePath,
    [Parameter(Mandatory = $true)]
    [string]$Tag,
    [string]$PubspecPath,
    [string]$CargoManifestPath
)

$ErrorActionPreference = "Stop"
. (Join-Path $PSScriptRoot "quality_common.ps1")
. (Join-Path $PSScriptRoot "release_journal_broker_common.ps1")

& (Join-Path $PSScriptRoot "release_verify_portable_archive.ps1") `
    -ArchivePath $ArchivePath `
    -Tag $Tag `
    -PubspecPath $PubspecPath `
    -CargoManifestPath $CargoManifestPath

$repositoryRoot = Get-AmeRepositoryRoot
$buildRoot = [System.IO.Path]::GetFullPath((Join-Path $repositoryRoot "build"))
$scratchRoot = [System.IO.Path]::GetFullPath((Join-Path `
    $buildRoot `
    "portable-unsigned-verify-$PID-$([Guid]::NewGuid().ToString('N'))"))
$buildPrefix = "$buildRoot$([System.IO.Path]::DirectorySeparatorChar)"
if (-not $scratchRoot.StartsWith(
    $buildPrefix,
    [System.StringComparison]::OrdinalIgnoreCase
)) {
    throw "Portable unsigned scratch directory must remain inside the repository build directory"
}
if (Test-Path -LiteralPath $scratchRoot) {
    throw "Portable unsigned scratch directory must be fresh"
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
    foreach ($target in @($applicationPath, $brokerPath)) {
        $signature = Get-AuthenticodeSignature -LiteralPath $target
        if ($signature.Status -ne [System.Management.Automation.SignatureStatus]::NotSigned) {
            throw "Unsigned portable executable unexpectedly carries an Authenticode signature"
        }
    }
    Assert-AmeBrokerPayload -Path $applicationPath | Out-Null
    Assert-AmeBrokerBinary -Path $brokerPath | Out-Null
} finally {
    if (Test-Path -LiteralPath $scratchRoot) {
        Remove-Item -LiteralPath $scratchRoot -Recurse -Force
    }
    if (Test-Path -LiteralPath $scratchRoot) {
        throw "Portable unsigned scratch directory still exists after cleanup"
    }
}

Write-Output "portable_unsigned_verified"
