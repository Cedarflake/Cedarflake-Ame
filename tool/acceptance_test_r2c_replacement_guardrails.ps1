$ErrorActionPreference = "Stop"

$repositoryRoot = Split-Path -Parent $PSScriptRoot
$testRoot = Join-Path $repositoryRoot "build\r2c-m-acceptance-guardrails-$PID"
$localRoot = Join-Path $testRoot "local"
$cloudRoot = Join-Path $testRoot "cloud"
$catalogPath = Join-Path $testRoot "catalog.sqlite3"
$storageRoot = Join-Path $testRoot "storage"
$requiredToken = "CEDARFLAKE_AME_R2C_REPLACEMENT_ACCEPTANCE_V1"

function Assert-Contains {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Value,
        [Parameter(Mandatory = $true)]
        [string]$Expected
    )

    if ($Value.IndexOf($Expected, [System.StringComparison]::Ordinal) -lt 0) {
        throw "Expected R2c-M guardrail output to contain: $Expected"
    }
}

New-Item -ItemType Directory -Path $localRoot -Force | Out-Null
New-Item -ItemType Directory -Path $cloudRoot -Force | Out-Null
[System.IO.File]::WriteAllText($catalogPath, "fixture")

try {
    try {
        & "$PSScriptRoot\acceptance_run_r2c_replacement_reliability.ps1" `
            -LocalRoot $localRoot `
            -CloudRoot $cloudRoot `
            -SourceCatalogPath $catalogPath `
            -StorageRoot $storageRoot `
            -AuthorizationToken "wrong" `
            -AcknowledgeCloudReadOnly `
            -ValidationOnly
        throw "Invalid R2c-M authorization unexpectedly succeeded"
    } catch {
        Assert-Contains $_.Exception.Message "exact current R2c-M replacement"
    }

    try {
        & "$PSScriptRoot\acceptance_run_r2c_replacement_reliability.ps1" `
            -LocalRoot $localRoot `
            -CloudRoot $cloudRoot `
            -SourceCatalogPath $catalogPath `
            -StorageRoot $storageRoot `
            -AuthorizationToken $requiredToken `
            -ValidationOnly
        throw "Missing cloud acknowledgement unexpectedly succeeded"
    } catch {
        Assert-Contains $_.Exception.Message "read-only acknowledgement"
    }

    $overlapStorage = Join-Path $localRoot "overlap"
    New-Item -ItemType Directory -Path $overlapStorage -Force | Out-Null
    try {
        & "$PSScriptRoot\acceptance_run_r2c_replacement_reliability.ps1" `
            -LocalRoot $localRoot `
            -CloudRoot $cloudRoot `
            -SourceCatalogPath $catalogPath `
            -StorageRoot $overlapStorage `
            -AuthorizationToken $requiredToken `
            -AcknowledgeCloudReadOnly `
            -ValidationOnly
        throw "Overlapping R2c-M storage unexpectedly succeeded"
    } catch {
        Assert-Contains $_.Exception.Message "outside every source path"
    }

    New-Item -ItemType Directory -Path $storageRoot -Force | Out-Null
    [System.IO.File]::WriteAllText((Join-Path $storageRoot "existing"), "fixture")
    try {
        & "$PSScriptRoot\acceptance_run_r2c_replacement_reliability.ps1" `
            -LocalRoot $localRoot `
            -CloudRoot $cloudRoot `
            -SourceCatalogPath $catalogPath `
            -StorageRoot $storageRoot `
            -AuthorizationToken $requiredToken `
            -AcknowledgeCloudReadOnly `
            -ValidationOnly
        throw "Nonempty R2c-M storage unexpectedly succeeded"
    } catch {
        Assert-Contains $_.Exception.Message "storage must be empty"
    }

    Remove-Item -LiteralPath $storageRoot -Recurse -Force
    New-Item -ItemType Directory -Path $storageRoot -Force | Out-Null
    $valid = & "$PSScriptRoot\acceptance_run_r2c_replacement_reliability.ps1" `
        -LocalRoot $localRoot `
        -CloudRoot $cloudRoot `
        -SourceCatalogPath $catalogPath `
        -StorageRoot $storageRoot `
        -AuthorizationToken $requiredToken `
        -AcknowledgeCloudReadOnly `
        -ValidationOnly
    Assert-Contains ($valid | Out-String) "AME_R2C_M_VALIDATION status=passed"

    $junctionTarget = Join-Path $localRoot "junction-storage"
    $junctionStorage = Join-Path $testRoot "junction-storage-alias"
    New-Item -ItemType Directory -Path $junctionTarget -Force | Out-Null
    New-Item -ItemType Junction -Path $junctionStorage -Target $junctionTarget | Out-Null
    try {
        & "$PSScriptRoot\acceptance_run_r2c_replacement_reliability.ps1" `
            -LocalRoot $localRoot `
            -CloudRoot $cloudRoot `
            -SourceCatalogPath $catalogPath `
            -StorageRoot $junctionStorage `
            -AuthorizationToken $requiredToken `
            -AcknowledgeCloudReadOnly `
            -ValidationOnly
        throw "Junction-aliased R2c-M storage unexpectedly succeeded"
    } catch {
        Assert-Contains $_.Exception.Message "outside every source path"
    } finally {
        [System.IO.Directory]::Delete($junctionStorage)
    }

    Write-Output "AME_R2C_M_GUARDRAILS status=passed"
} finally {
    if (Test-Path -LiteralPath $testRoot) {
        Remove-Item -LiteralPath $testRoot -Recurse -Force
    }
}
