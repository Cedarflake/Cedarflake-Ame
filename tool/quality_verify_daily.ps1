[CmdletBinding()]
param(
    [ValidateSet(
        "all",
        "static",
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
        Invoke-AmeChecked $toolchain.Cargo @(
            "test",
            "--locked",
            "--manifest-path",
            "rust\Cargo.toml",
            "--all-targets",
            "--all-features",
            "--",
            "--test-threads=1"
        )
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
