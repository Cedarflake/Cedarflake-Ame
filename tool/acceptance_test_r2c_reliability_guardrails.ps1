$ErrorActionPreference = "Stop"

$repositoryRoot = Split-Path -Parent $PSScriptRoot
$buildRoot = Join-Path $repositoryRoot "build"
$testRoot = Join-Path $buildRoot "r2c-h-acceptance-guardrails-$PID"
$localRoot = Join-Path $testRoot "local"
$cloudRoot = Join-Path $testRoot "cloud"
$catalogPath = Join-Path $testRoot "catalog.sqlite3"
$storageRoot = Join-Path $testRoot "storage"
$requiredToken = "CEDARFLAKE_AME_R2C_RELIABILITY_ACCEPTANCE_V1"

function Assert-Contains {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Value,
        [Parameter(Mandatory = $true)]
        [string]$Expected
    )

    if ($Value.IndexOf($Expected, [System.StringComparison]::Ordinal) -lt 0) {
        throw "Expected R2c-H guardrail output to contain: $Expected"
    }
}

New-Item -ItemType Directory -Path $localRoot -Force | Out-Null
New-Item -ItemType Directory -Path $cloudRoot -Force | Out-Null
[System.IO.File]::WriteAllText($catalogPath, "fixture")

try {
    try {
        & "$PSScriptRoot\acceptance_run_r2c_reliability.ps1" `
            -LocalRoot $localRoot `
            -CloudRoot $cloudRoot `
            -SourceCatalogPath $catalogPath `
            -StorageRoot $storageRoot `
            -AuthorizationToken "wrong" `
            -AcknowledgeCloudReadOnly `
            -ValidationOnly
        throw "Invalid R2c-H authorization unexpectedly succeeded"
    } catch {
        Assert-Contains $_.Exception.Message "exact current R2c-H reliability"
    }

    try {
        & "$PSScriptRoot\acceptance_run_r2c_reliability.ps1" `
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

    try {
        & "$PSScriptRoot\acceptance_run_r2c_reliability.ps1" `
            -LocalRoot $localRoot `
            -CloudRoot $cloudRoot `
            -SourceCatalogPath $catalogPath `
            -StorageRoot (Join-Path $localRoot "overlap") `
            -AuthorizationToken $requiredToken `
            -AcknowledgeCloudReadOnly `
            -ValidationOnly
        throw "Overlapping R2c-H storage unexpectedly succeeded"
    } catch {
        Assert-Contains $_.Exception.Message "outside every source path"
    }

    New-Item -ItemType Directory -Path $storageRoot -Force | Out-Null
    [System.IO.File]::WriteAllText((Join-Path $storageRoot "existing"), "fixture")
    try {
        & "$PSScriptRoot\acceptance_run_r2c_reliability.ps1" `
            -LocalRoot $localRoot `
            -CloudRoot $cloudRoot `
            -SourceCatalogPath $catalogPath `
            -StorageRoot $storageRoot `
            -AuthorizationToken $requiredToken `
            -AcknowledgeCloudReadOnly `
            -ValidationOnly
        throw "Nonempty R2c-H storage unexpectedly succeeded"
    } catch {
        Assert-Contains $_.Exception.Message "storage must be empty"
    }

    Remove-Item -LiteralPath $storageRoot -Recurse -Force
    $valid = & "$PSScriptRoot\acceptance_run_r2c_reliability.ps1" `
        -LocalRoot $localRoot `
        -CloudRoot $cloudRoot `
        -SourceCatalogPath $catalogPath `
        -StorageRoot $storageRoot `
        -AuthorizationToken $requiredToken `
        -AcknowledgeCloudReadOnly `
        -ValidationOnly
    Assert-Contains ($valid | Out-String) "AME_R2C_H_VALIDATION status=passed"

    Write-Output "AME_R2C_H_GUARDRAILS status=passed"
} finally {
    if (Test-Path -LiteralPath $testRoot) {
        Remove-Item -LiteralPath $testRoot -Recurse -Force
    }
}
