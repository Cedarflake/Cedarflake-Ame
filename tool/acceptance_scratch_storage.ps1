$ErrorActionPreference = "Stop"
. (Join-Path $PSScriptRoot "quality_common.ps1")

function Assert-AmeAcceptanceScratchPath {
    param([Parameter(Mandatory = $true)][string]$Path)

    $repository = [IO.Path]::GetFullPath((Get-AmeRepositoryRoot)).TrimEnd("\", "/")
    $resolved = [IO.Path]::GetFullPath($Path)
    if (-not $resolved.StartsWith($repository + [IO.Path]::DirectorySeparatorChar, [StringComparison]::OrdinalIgnoreCase)) {
        throw "Acceptance scratch must remain inside the repository"
    }
    $current = $resolved
    while ($current.Length -ge $repository.Length) {
        if (Test-Path -LiteralPath $current) {
            $entry = Get-Item -LiteralPath $current -Force
            if (($entry.Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0) {
                throw "Acceptance scratch cannot traverse a reparse point"
            }
        }
        $current = Split-Path -Parent $current
    }
    return $resolved
}

function Remove-AmeAcceptanceScratch {
    param(
        [Parameter(Mandatory = $true)][string]$Root,
        [Parameter(Mandatory = $true)][string]$Name
    )

    if ($Name -cnotmatch "^[a-z][a-z0-9-]{0,63}$") { throw "Invalid acceptance scratch name" }
    $resolved = Assert-AmeAcceptanceScratchPath $Root
    $expected = Join-Path (Get-AmeRepositoryRoot) "build\test\derived\temporary\$Name"
    if ($resolved -ine [IO.Path]::GetFullPath($expected)) {
        throw "Unexpected acceptance scratch directory"
    }
    if (-not (Test-Path -LiteralPath $resolved)) { return }
    $marker = Join-Path $resolved ".ame-scratch.json"
    Assert-AmeAcceptanceScratchPath $marker | Out-Null
    $receipt = Get-Content -LiteralPath $marker -Raw -Encoding UTF8 | ConvertFrom-Json
    if ($receipt.format -cne "ame-acceptance-scratch-v1" -or $receipt.name -cne $Name) {
        throw "Unrecognized acceptance scratch ownership"
    }
    $directories = [Collections.Generic.Queue[string]]::new()
    $directories.Enqueue($resolved)
    while ($directories.Count -gt 0) {
        foreach ($entry in Get-ChildItem -LiteralPath $directories.Dequeue() -Force) {
            if (($entry.Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0) {
                throw "Acceptance scratch contains a reparse point"
            }
            if ($entry.PSIsContainer) {
                $directories.Enqueue($entry.FullName)
            } else {
                $handle = [IO.File]::Open($entry.FullName, [IO.FileMode]::Open, [IO.FileAccess]::Read, [IO.FileShare]::None)
                $handle.Dispose()
            }
        }
    }
    Remove-Item -LiteralPath $resolved -Recurse -Force
}

function Invoke-AmeAcceptanceScratch {
    param(
        [Parameter(Mandatory = $true)][string]$Name,
        [Parameter(Mandatory = $true)][scriptblock]$Action
    )

    if ($Name -cnotmatch "^[a-z][a-z0-9-]{0,63}$") {
        throw "Invalid acceptance scratch name"
    }
    $parent = Join-Path (Get-AmeRepositoryRoot) "build\test\derived\temporary"
    Assert-AmeAcceptanceScratchPath $parent | Out-Null
    [IO.Directory]::CreateDirectory($parent) | Out-Null
    $root = Join-Path $parent $Name
    $leasePath = Join-Path $parent "$Name.lease"
    Assert-AmeAcceptanceScratchPath $leasePath | Out-Null
    $lease = [IO.FileStream]::new(
        $leasePath, [IO.FileMode]::OpenOrCreate, [IO.FileAccess]::ReadWrite,
        [IO.FileShare]::None, 4096, [IO.FileOptions]::DeleteOnClose
    )
    try {
        Remove-AmeAcceptanceScratch -Root $root -Name $Name
        [IO.Directory]::CreateDirectory($root) | Out-Null
        $receipt = @{ format = "ame-acceptance-scratch-v1"; name = $Name } | ConvertTo-Json
        [IO.File]::WriteAllText((Join-Path $root ".ame-scratch.json"), $receipt, [Text.UTF8Encoding]::new($false))
        $operationFailure = $null
        try {
            & $Action $root
        } catch {
            $operationFailure = $_
        } finally {
            try {
                Remove-AmeAcceptanceScratch -Root $root -Name $Name
            } catch {
                if ($null -ne $operationFailure) {
                    $failures = [Exception[]]@($operationFailure.Exception, $_.Exception)
                    $message = "Acceptance and scratch cleanup failed for '$Name'. " +
                        "Action: $($failures[0].Message) Cleanup: $($failures[1].Message)"
                    throw [AggregateException]::new($message, $failures)
                }
                throw
            }
        }
        if ($null -ne $operationFailure) { throw $operationFailure }
    } finally {
        $lease.Dispose()
    }
}
