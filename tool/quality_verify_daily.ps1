[CmdletBinding()]
param(
    [ValidateSet(
        "all",
        "static",
        "rust_catalog_history",
        "rust_catalog",
        "rust_sync_scan",
        "rust_other",
        "flutter",
        "windows_scan",
        "windows_accessibility"
    )]
    [string]$Component = "all"
)

$ErrorActionPreference = "Stop"
. (Join-Path $PSScriptRoot "quality_common.ps1")

$repositoryRoot = Get-AmeRepositoryRoot
$toolchain = Get-AmeToolchain
$toolLock = Enter-AmeRepositoryToolLock
Push-Location $repositoryRoot

try {
    if ($Component -in @("all", "static")) {
        & (Join-Path $PSScriptRoot "quality_lint.ps1")
    }
    if ($Component -eq "all") {
        & (Join-Path $PSScriptRoot "quality_test_rust.ps1")
    } elseif ($Component.StartsWith("rust_", [StringComparison]::Ordinal)) {
        & (Join-Path $PSScriptRoot "quality_test_rust.ps1") -Partition $Component.Substring(5)
    }
    if ($Component -in @("all", "flutter")) {
        & (Join-Path $PSScriptRoot "quality_test_flutter.ps1")
    }
    if ($Component -in @("all", "windows_scan")) {
        & (Join-Path $PSScriptRoot "integration_test_windows.ps1")
    }
    if ($Component -in @("all", "windows_accessibility")) {
        & (Join-Path $PSScriptRoot "integration_test_windows_accessibility.ps1")
    }
    if ($Component -in @("all", "static")) {
        & (Join-Path $PSScriptRoot "quality_verify_bridge_contracts.ps1")
        Invoke-AmeChecked $toolchain.Git @("diff", "HEAD", "--check", "--")
    }
} finally {
    Pop-Location
    Exit-AmeRepositoryToolLock $toolLock
}
