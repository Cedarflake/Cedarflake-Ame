[CmdletBinding()]
param(
    [UInt64]$MaxPeakWorkingSetBytes = 536870912,
    [switch]$IncludeRealLibrary,
    [string]$AcceptanceStorageRoot,
    [string]$RootA,
    [string]$RootB,
    [string]$AuthorizationToken
)

$ErrorActionPreference = "Stop"
. (Join-Path $PSScriptRoot "quality_common.ps1")

$toolLock = Enter-AmeRepositoryToolLock

$realLibraryArguments = @(
    $AcceptanceStorageRoot,
    $RootA,
    $RootB,
    $AuthorizationToken
)
$hasAnyRealLibraryArgument = $realLibraryArguments.Where({ -not [string]::IsNullOrWhiteSpace($_) }).Count -gt 0
$hasAllRealLibraryArguments = $realLibraryArguments.Where({ [string]::IsNullOrWhiteSpace($_) }).Count -eq 0

if ($IncludeRealLibrary -and -not $hasAllRealLibraryArguments) {
    throw "Real-library release verification requires storage, two roots, and authorization"
}
if (-not $IncludeRealLibrary -and $hasAnyRealLibraryArgument) {
    throw "Real-library paths require the explicit IncludeRealLibrary switch"
}

try {
    & (Join-Path $PSScriptRoot "quality_verify_daily.ps1")
    & (Join-Path $PSScriptRoot "release_verify_windows.ps1")
    & (Join-Path $PSScriptRoot "performance_benchmark_synthetic_library.ps1") `
        -MaxPeakWorkingSetBytes $MaxPeakWorkingSetBytes

    if ($IncludeRealLibrary) {
        & (Join-Path $PSScriptRoot "acceptance_verify_read_only_catalog.ps1") `
            -StorageRoot $AcceptanceStorageRoot `
            -RootA $RootA `
            -RootB $RootB `
            -AuthorizationToken $AuthorizationToken
    }
} finally {
    Exit-AmeRepositoryToolLock $toolLock
}
