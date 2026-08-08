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

& (Join-Path $PSScriptRoot "verify.ps1")
& (Join-Path $PSScriptRoot "verify_windows_release.ps1")
& (Join-Path $PSScriptRoot "benchmark_synthetic_library.ps1") `
    -MaxPeakWorkingSetBytes $MaxPeakWorkingSetBytes

if ($IncludeRealLibrary) {
    & (Join-Path $PSScriptRoot "verify_read_only_library_catalog.ps1") `
        -StorageRoot $AcceptanceStorageRoot `
        -RootA $RootA `
        -RootB $RootB `
        -AuthorizationToken $AuthorizationToken
}
