[CmdletBinding()]
param(
    [ValidateSet("all", "catalog_history", "catalog", "sync_scan", "other")]
    [string]$Partition = "all"
)

$ErrorActionPreference = "Stop"
. (Join-Path $PSScriptRoot "quality_common.ps1")
. (Join-Path $PSScriptRoot "quality_rust_test_partitions.ps1")

$repositoryRoot = Get-AmeRepositoryRoot
$toolchain = Get-AmeToolchain
$toolLock = Enter-AmeRepositoryToolLock
Push-Location $repositoryRoot
try {
    $cargoArguments = @(
        "test", "--locked", "--manifest-path", "rust\Cargo.toml",
        "--all-targets", "--all-features", "--"
    )
    if ($Partition -ne "all") {
        $inventory = @(Invoke-AmeChecked $toolchain.Cargo (
            $cargoArguments + @("--list", "--format=terse")
        ))
        $testNames = @(ConvertFrom-AmeRustTestInventory $inventory)
        $coverage = @(Assert-AmeRustTestPartitionCoverage $testNames)
        foreach ($entry in $coverage) {
            Write-Output "Rust partition $($entry.Partition): $($entry.Tests) listed tests"
        }
    }
    Invoke-AmeChecked $toolchain.Cargo (
        $cargoArguments + @(Get-AmeRustTestArguments -Partition $Partition)
    )
} finally {
    Pop-Location
    Exit-AmeRepositoryToolLock $toolLock
}
