[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [string]$Tag,
    [string]$ReleaseRoot,
    [string]$OutputDirectory,
    [string]$PubspecPath,
    [string]$CargoManifestPath
)

$ErrorActionPreference = "Stop"
. (Join-Path $PSScriptRoot "quality_common.ps1")

$repositoryRoot = Get-AmeRepositoryRoot
if ([string]::IsNullOrWhiteSpace($ReleaseRoot)) {
    $ReleaseRoot = Join-Path $repositoryRoot "build\windows\x64\runner\Release"
}
if ([string]::IsNullOrWhiteSpace($OutputDirectory)) {
    $OutputDirectory = Join-Path $repositoryRoot "build\release-artifacts"
}

& (Join-Path $PSScriptRoot "release_validate_version.ps1") `
    -Tag $Tag `
    -PubspecPath $PubspecPath `
    -CargoManifestPath $CargoManifestPath

$resolvedReleaseRoot = [System.IO.Path]::GetFullPath($ReleaseRoot)
$resolvedOutputDirectory = [System.IO.Path]::GetFullPath($OutputDirectory)
if (-not (Test-Path -LiteralPath $resolvedReleaseRoot -PathType Container)) {
    throw "Windows Release directory was not found: $resolvedReleaseRoot"
}
$releasePrefix = "$resolvedReleaseRoot$([System.IO.Path]::DirectorySeparatorChar)"
if (
    $resolvedOutputDirectory.Equals(
        $resolvedReleaseRoot,
        [System.StringComparison]::OrdinalIgnoreCase
    ) -or
    $resolvedOutputDirectory.StartsWith(
        $releasePrefix,
        [System.StringComparison]::OrdinalIgnoreCase
    )
) {
    throw "Portable artifact output must remain outside the Release directory"
}

$requiredSourcePaths = @(
    "cedarflake_ame.exe",
    "rust_lib_cedarflake_ame.dll",
    "flutter_windows.dll",
    "data\app.so",
    "data\icudtl.dat",
    "data\flutter_assets"
)
foreach ($requiredSourcePath in $requiredSourcePaths) {
    $candidatePath = Join-Path $resolvedReleaseRoot $requiredSourcePath
    if (-not (Test-Path -LiteralPath $candidatePath)) {
        throw "Windows Release directory is incomplete: $requiredSourcePath"
    }
}

$buildRoot = [System.IO.Path]::GetFullPath((Join-Path $repositoryRoot "build"))
$scratchRoot = [System.IO.Path]::GetFullPath(
    (Join-Path $buildRoot "release-package-$PID")
)
$buildPrefix = "$buildRoot$([System.IO.Path]::DirectorySeparatorChar)"
if (-not $scratchRoot.StartsWith(
    $buildPrefix,
    [System.StringComparison]::OrdinalIgnoreCase
)) {
    throw "Portable packaging scratch storage must remain inside the build directory"
}

$payloadRoot = Join-Path $scratchRoot "Cedarflake-Ame"
$archiveName = "Cedarflake-Ame-$Tag-windows-x64-portable.zip"
$archivePath = Join-Path $resolvedOutputDirectory $archiveName

try {
    if (Test-Path -LiteralPath $scratchRoot) {
        Remove-Item -LiteralPath $scratchRoot -Recurse -Force
    }
    New-Item -ItemType Directory -Path $payloadRoot -Force | Out-Null
    New-Item -ItemType Directory -Path $resolvedOutputDirectory -Force | Out-Null
    Get-ChildItem -LiteralPath $resolvedReleaseRoot -Force | ForEach-Object {
        Copy-Item -LiteralPath $_.FullName `
            -Destination $payloadRoot `
            -Recurse `
            -Force
    }

    if (Test-Path -LiteralPath $archivePath) {
        Remove-Item -LiteralPath $archivePath -Force
    }
    Add-Type -AssemblyName System.IO.Compression.FileSystem
    [System.IO.Compression.ZipFile]::CreateFromDirectory(
        $scratchRoot,
        $archivePath,
        [System.IO.Compression.CompressionLevel]::Optimal,
        $false
    )

    & (Join-Path $PSScriptRoot "release_verify_portable_archive.ps1") `
        -ArchivePath $archivePath `
        -Tag $Tag `
        -PubspecPath $PubspecPath `
        -CargoManifestPath $CargoManifestPath
} finally {
    if (Test-Path -LiteralPath $scratchRoot) {
        Remove-Item -LiteralPath $scratchRoot -Recurse -Force
    }
}

Write-Output $archivePath
