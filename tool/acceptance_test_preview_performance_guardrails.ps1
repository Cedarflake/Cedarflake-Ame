$ErrorActionPreference = "Stop"

$repositoryRoot = Split-Path -Parent $PSScriptRoot
$buildRoot = Join-Path $repositoryRoot "build"
$testRoot = Join-Path $buildRoot "preview-acceptance-guardrails-$PID"
$sourceRoot = Join-Path $testRoot "source"
$catalogPath = Join-Path $testRoot "catalog.sqlite3"
$storageRoot = Join-Path $testRoot "storage"

function Assert-Contains {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Value,
        [Parameter(Mandatory = $true)]
        [string]$Expected
    )

    if ($Value.IndexOf($Expected, [System.StringComparison]::Ordinal) -lt 0) {
        throw "Expected preview acceptance output to contain: $Expected"
    }
}

New-Item -ItemType Directory -Path $sourceRoot -Force | Out-Null
[System.IO.File]::WriteAllText($catalogPath, "fixture")

try {
    try {
        & "$PSScriptRoot\acceptance_run_preview_performance.ps1" `
            -RootPath $sourceRoot `
            -SourceCatalogPath $catalogPath `
            -StorageRoot $storageRoot `
            -AuthorizationToken "wrong" `
            -ValidationOnly
        throw "Invalid preview acceptance authorization unexpectedly succeeded"
    } catch {
        Assert-Contains $_.Exception.Message "exact current preview performance acceptance"
    }

    try {
        & "$PSScriptRoot\acceptance_run_preview_performance.ps1" `
            -RootPath $sourceRoot `
            -SourceCatalogPath $catalogPath `
            -StorageRoot (Join-Path $sourceRoot "overlap") `
            -AuthorizationToken "CEDARFLAKE_AME_PREVIEW_PERFORMANCE_ACCEPTANCE_V1" `
            -ValidationOnly
        throw "Overlapping preview acceptance storage unexpectedly succeeded"
    } catch {
        Assert-Contains $_.Exception.Message "outside the source root"
    }

    New-Item -ItemType Directory -Path $storageRoot -Force | Out-Null
    [System.IO.File]::WriteAllText((Join-Path $storageRoot "existing"), "fixture")
    try {
        & "$PSScriptRoot\acceptance_run_preview_performance.ps1" `
            -RootPath $sourceRoot `
            -SourceCatalogPath $catalogPath `
            -StorageRoot $storageRoot `
            -AuthorizationToken "CEDARFLAKE_AME_PREVIEW_PERFORMANCE_ACCEPTANCE_V1" `
            -ValidationOnly
        throw "Nonempty preview acceptance storage unexpectedly succeeded"
    } catch {
        Assert-Contains $_.Exception.Message "storage must be empty"
    }

    Remove-Item -LiteralPath $storageRoot -Recurse -Force
    $valid = & "$PSScriptRoot\acceptance_run_preview_performance.ps1" `
        -RootPath $sourceRoot `
        -SourceCatalogPath $catalogPath `
        -StorageRoot $storageRoot `
        -AuthorizationToken "CEDARFLAKE_AME_PREVIEW_PERFORMANCE_ACCEPTANCE_V1" `
        -ValidationOnly
    Assert-Contains ($valid | Out-String) "AME_PREVIEW_ACCEPTANCE_VALIDATION passed"

    Write-Output "AME_PREVIEW_ACCEPTANCE_GUARDRAILS passed"
} finally {
    if (Test-Path -LiteralPath $testRoot) {
        Remove-Item -LiteralPath $testRoot -Recurse -Force
    }
}
