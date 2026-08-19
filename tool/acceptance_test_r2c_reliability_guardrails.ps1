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
        $overlapStorage = Join-Path $localRoot "overlap"
        New-Item -ItemType Directory -Path $overlapStorage -Force | Out-Null
        & "$PSScriptRoot\acceptance_run_r2c_reliability.ps1" `
            -LocalRoot $localRoot `
            -CloudRoot $cloudRoot `
            -SourceCatalogPath $catalogPath `
            -StorageRoot $overlapStorage `
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
    New-Item -ItemType Directory -Path $storageRoot -Force | Out-Null
    $valid = & "$PSScriptRoot\acceptance_run_r2c_reliability.ps1" `
        -LocalRoot $localRoot `
        -CloudRoot $cloudRoot `
        -SourceCatalogPath $catalogPath `
        -StorageRoot $storageRoot `
        -AuthorizationToken $requiredToken `
        -AcknowledgeCloudReadOnly `
        -ValidationOnly
    Assert-Contains ($valid | Out-String) "AME_R2C_H_VALIDATION status=passed"

    $junctionTarget = Join-Path $localRoot "junction-storage"
    $junctionStorage = Join-Path $testRoot "junction-storage-alias"
    New-Item -ItemType Directory -Path $junctionTarget -Force | Out-Null
    New-Item -ItemType Junction -Path $junctionStorage -Target $junctionTarget | Out-Null
    try {
        & "$PSScriptRoot\acceptance_run_r2c_reliability.ps1" `
            -LocalRoot $localRoot `
            -CloudRoot $cloudRoot `
            -SourceCatalogPath $catalogPath `
            -StorageRoot $junctionStorage `
            -AuthorizationToken $requiredToken `
            -AcknowledgeCloudReadOnly `
            -ValidationOnly
        throw "Junction-aliased R2c-H storage unexpectedly succeeded"
    } catch {
        Assert-Contains $_.Exception.Message "outside every source path"
    } finally {
        [System.IO.Directory]::Delete($junctionStorage)
    }

    $processJob = $null
    $jobProcess = $null
    $sideProcess = $null
    try {
        $hostExecutable = (Get-Process -Id $PID).Path
        $processJob = [AmeR2cProcessJob]::new()
        $jobProcess = $processJob.Start(
            $hostExecutable,
            '-NoProfile -NonInteractive -Command "Start-Sleep -Seconds 30"',
            $repositoryRoot
        )
        $sideProcess = Start-Process `
            -FilePath $hostExecutable `
            -ArgumentList @(
                "-NoProfile",
                "-NonInteractive",
                "-Command",
                "Start-Sleep -Seconds 30"
            ) `
            -WindowStyle Hidden `
            -PassThru
        Start-Sleep -Milliseconds 250
        if ([UInt64]$processJob.PeakMemoryBytes -eq 0) {
            throw "The R2c-H process job did not account for its child"
        }
        $processJob.Dispose()
        $processJob = $null
        if (-not $jobProcess.WaitForExit(5000)) {
            throw "The R2c-H process job did not stop its owned child"
        }
        $sideProcess.Refresh()
        if ($sideProcess.HasExited) {
            throw "The R2c-H process job stopped an unrelated same-name process"
        }
    } finally {
        if ($null -ne $processJob) {
            $processJob.Dispose()
        }
        if ($null -ne $jobProcess -and -not $jobProcess.HasExited) {
            $jobProcess.WaitForExit(5000) | Out-Null
        }
        if ($null -ne $sideProcess -and -not $sideProcess.HasExited) {
            Stop-Process -Id $sideProcess.Id -Force
            $sideProcess.WaitForExit()
        }
    }

    $exitCodeJob = $null
    try {
        $exitCodeJob = [AmeR2cProcessJob]::new()
        $exitCodeProcess = $exitCodeJob.Start(
            $hostExecutable,
            '-NoProfile -NonInteractive -Command "exit 7"',
            $repositoryRoot
        )
        $exitCodeProcess.WaitForExit()
        if ([int]$exitCodeJob.PrimaryExitCode -ne 7) {
            throw "The R2c-H process job lost its owned process exit code"
        }
    } finally {
        if ($null -ne $exitCodeJob) {
            $exitCodeJob.Dispose()
        }
    }

    Write-Output "AME_R2C_H_GUARDRAILS status=passed"
} finally {
    if (Test-Path -LiteralPath $testRoot) {
        Remove-Item -LiteralPath $testRoot -Recurse -Force
    }
}
