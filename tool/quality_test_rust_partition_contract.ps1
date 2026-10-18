$ErrorActionPreference = "Stop"
. (Join-Path $PSScriptRoot "quality_rust_test_partitions.ps1")

$inventory = @(
    "adapters::sqlite_catalog::migrations::tests::upgrade: test",
    "adapters::sqlite_catalog::change_queue::tests::lease: test",
    "adapters::sqlite_catalog::tests::rollback: test",
    "application::library_synchronization::tests::priority: test",
    "application::scan_library::tests::cancel: test",
    "adapters::local_files::tests::identity: test",
    "new_module::future_case: test",
    "tests::broker_cli: test",
    "tests::broker_cli: test",
    "9 tests, 0 benchmarks"
)
$names = @(ConvertFrom-AmeRustTestInventory $inventory)
$coverage = @(Assert-AmeRustTestPartitionCoverage $names)
$expectedCounts = @{
    catalog_history = 2
    catalog = 1
    sync_scan = 2
    other = 4
}
if ($names.Count -ne 9 -or $coverage.Count -ne 4) {
    throw "Rust inventory lost tests, including same-named cases in different targets"
}
foreach ($entry in $coverage) {
    if ($entry.Tests -ne $expectedCounts[$entry.Partition]) {
        throw "Rust partition ownership changed: $($entry.Partition)"
    }
}

$expectedArguments = @{
    all = @("--test-threads=1")
    catalog_history = @(
        "--test-threads=1", "adapters::sqlite_catalog::migrations::",
        "adapters::sqlite_catalog::change_queue::"
    )
    catalog = @(
        "--test-threads=1", "adapters::sqlite_catalog::",
        "--skip", "adapters::sqlite_catalog::migrations::",
        "--skip", "adapters::sqlite_catalog::change_queue::"
    )
    sync_scan = @(
        "--test-threads=1", "application::library_synchronization::",
        "application::scan_library::"
    )
    other = @(
        "--test-threads=1", "--skip", "adapters::sqlite_catalog::",
        "--skip", "application::library_synchronization::",
        "--skip", "application::scan_library::"
    )
}
foreach ($partition in $expectedArguments.Keys) {
    $actual = @(Get-AmeRustTestArguments $partition)
    if (($actual -join "`n") -cne ($expectedArguments[$partition] -join "`n")) {
        throw "Rust partition arguments lost complete coverage or case isolation: $partition"
    }
}

$missing = @(Get-AmeRustTestPartitions | Where-Object { $_.Name -cne "catalog" })
$overlap = @(Get-AmeRustTestPartitions)
$overlap[1].Exclude = @()
$duplicate = @(Get-AmeRustTestPartitions)
$duplicate[1].Name = "catalog_history"
$blankFilter = @(Get-AmeRustTestPartitions)
$blankFilter[0].Include = @(" ")
foreach ($partitions in @($missing, $overlap, $duplicate, $blankFilter)) {
    $rejected = $false
    try { Assert-AmeRustTestPartitionCoverage $names -Partitions $partitions | Out-Null }
    catch { $rejected = $true }
    if (-not $rejected) { throw "Rust coverage guard accepted a missing or ambiguous owner" }
}
foreach ($invalidInventory in @(@(""), @("unknown test protocol"), @("0 tests, 0 benchmarks"))) {
    $rejected = $false
    try { ConvertFrom-AmeRustTestInventory $invalidInventory | Out-Null }
    catch { $rejected = $true }
    if (-not $rejected) { throw "Rust inventory accepted an empty or unknown test protocol" }
}
$rejected = $false
try { Get-AmeRustTestArguments -Partition "unsupported" | Out-Null }
catch [System.Management.Automation.ParameterBindingException] { $rejected = $true }
if (-not $rejected) { throw "Rust partition selection accepted an unknown owner" }

Write-Output "Rust partition inventory, ownership, arguments and rejecting cases verified"
