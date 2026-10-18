$ErrorActionPreference = "Stop"

function Get-AmeRustTestPartitions {
    return @(
        [pscustomobject]@{
            Name = "catalog_history"
            Include = @(
                "adapters::sqlite_catalog::migrations::",
                "adapters::sqlite_catalog::change_queue::"
            )
            Exclude = @()
        },
        [pscustomobject]@{
            Name = "catalog"
            Include = @("adapters::sqlite_catalog::")
            Exclude = @(
                "adapters::sqlite_catalog::migrations::",
                "adapters::sqlite_catalog::change_queue::"
            )
        },
        [pscustomobject]@{
            Name = "sync_scan"
            Include = @(
                "application::library_synchronization::",
                "application::scan_library::"
            )
            Exclude = @()
        },
        [pscustomobject]@{
            Name = "other"
            Include = @()
            Exclude = @(
                "adapters::sqlite_catalog::",
                "application::library_synchronization::",
                "application::scan_library::"
            )
        }
    )
}

function Get-AmeRustTestArguments {
    param(
        [ValidateSet("all", "catalog_history", "catalog", "sync_scan", "other")]
        [string]$Partition = "all"
    )

    $arguments = @("--test-threads=1")
    if ($Partition -eq "all") { return $arguments }
    $definition = Get-AmeRustTestPartitions |
        Where-Object { $_.Name -ceq $Partition }
    $arguments += $definition.Include
    foreach ($excluded in $definition.Exclude) {
        $arguments += @("--skip", $excluded)
    }
    return $arguments
}

function ConvertFrom-AmeRustTestInventory {
    param([Parameter(Mandatory = $true)][string[]]$Lines)

    $names = [System.Collections.Generic.List[string]]::new()
    foreach ($line in $Lines) {
        if ([string]::IsNullOrWhiteSpace($line)) { continue }
        if ($line -cmatch '^(?<name>\S+): (?:test|benchmark)$') {
            $names.Add($Matches.name)
            continue
        }
        if ($line -cmatch '^\d+ tests?, \d+ benchmarks?$') { continue }
        throw "Unrecognized Rust test inventory output: $line"
    }
    if ($names.Count -eq 0) { throw "Rust test inventory is empty" }
    return $names.ToArray()
}

function Assert-AmeRustTestPartitionCoverage {
    param(
        [Parameter(Mandatory = $true)][string[]]$TestNames,
        [object[]]$Partitions = (Get-AmeRustTestPartitions)
    )

    if ($TestNames.Count -eq 0 -or $Partitions.Count -eq 0) {
        throw "Rust partition coverage requires a complete nonempty inventory and partition set"
    }
    $counts = [ordered]@{}
    foreach ($partition in $Partitions) {
        if ([string]::IsNullOrWhiteSpace($partition.Name) -or
            $counts.Contains($partition.Name)) {
            throw "Rust partition names must be nonempty and unique"
        }
        foreach ($filter in @($partition.Include) + @($partition.Exclude)) {
            if ([string]::IsNullOrWhiteSpace($filter)) {
                throw "Rust partition filters must be nonempty"
            }
        }
        $counts.Add($partition.Name, 0)
    }
    foreach ($name in $TestNames) {
        $owners = @($Partitions | Where-Object {
            $included = @($_.Include | Where-Object {
                $name.IndexOf($_, [StringComparison]::Ordinal) -ge 0
            })
            $excluded = @($_.Exclude | Where-Object {
                $name.IndexOf($_, [StringComparison]::Ordinal) -ge 0
            })
            ($_.Include.Count -eq 0 -or $included.Count -gt 0) -and $excluded.Count -eq 0
        })
        if ($owners.Count -ne 1) {
            throw "Rust test must belong to exactly one hosted partition: $name"
        }
        $counts[$owners[0].Name]++
    }
    foreach ($partition in $Partitions) {
        if ($counts[$partition.Name] -eq 0) {
            throw "Rust partition has no tests: $($partition.Name)"
        }
        [pscustomobject]@{ Partition = $partition.Name; Tests = $counts[$partition.Name] }
    }
}
