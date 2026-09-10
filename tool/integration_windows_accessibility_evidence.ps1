. (Join-Path $PSScriptRoot "integration_windows_accessibility_cleanup.ps1")
. (Join-Path $PSScriptRoot "integration_windows_accessibility_timing.ps1")

function Write-AmeWindowsUiaProbeRecord {
    param(
        [Parameter(Mandatory = $true)] [string]$ResultPath,
        [Parameter(Mandatory = $true)] [string]$Token,
        [Parameter(Mandatory = $true)] [string]$Phase,
        [Parameter(Mandatory = $true)] [int]$TargetProcessId,
        [Parameter(Mandatory = $true)] [int]$ProbeProcessId,
        [ValidateSet("progress", "complete")] [string]$Status = "progress",
        [ValidateSet("loading-assemblies", "loading-uia-types", "loading-uia-client", "locating-window", "activating-cache", "finding-elements", "disposing-cache", "reading-properties", "asserting-contract", "complete")]
        [string]$Stage,
        [ValidateRange(0, 2147483647)] [int]$Attempt = 0,
        [ValidateRange(0, 2147483647)] [int]$WindowCount = 0,
        [ValidateRange(0, 2147483647)] [int]$ElementCount = 0,
        [ValidateRange(0, 2147483647)] [int]$ElapsedMilliseconds = 0,
        [AllowNull()] [string]$LastMismatch,
        [AllowNull()] [string]$Failure,
        [AllowNull()] [System.Collections.IDictionary]$StageMilliseconds,
        [ValidateRange(0, 2147483647)] [long]$EvidenceWriteMilliseconds = 0
    )

    $record = [ordered]@{
        format = "ame-windows-uia-probe-v2"
        token = $Token
        phase = $Phase
        targetProcessId = $TargetProcessId
        probeProcessId = $ProbeProcessId
        status = $Status
        stage = $Stage
        attempt = $Attempt
        windowCount = $WindowCount
        elementCount = $ElementCount
        elapsedMilliseconds = $ElapsedMilliseconds
        lastMismatch = $(if ($LastMismatch) { $LastMismatch } else { $null })
        failure = $(if ($Failure) { $Failure } else { $null })
    }
    if ($null -ne $StageMilliseconds) {
        $record.stageMilliseconds = $StageMilliseconds
        $record.evidenceWriteMilliseconds = $EvidenceWriteMilliseconds
    }
    $utf8 = [System.Text.UTF8Encoding]::new($false)
    $payload = $record | ConvertTo-Json -Compress -Depth 4
    if ($utf8.GetByteCount($payload) -gt 65536) {
        throw "Windows UIA probe evidence exceeds its bounded record size"
    }
    $draftPath = [System.IO.Path]::ChangeExtension($ResultPath, "tmp")
    [System.IO.File]::WriteAllText($draftPath, $payload, $utf8)
    if ([System.IO.File]::Exists($ResultPath)) {
        [System.IO.File]::Replace(
            $draftPath,
            $ResultPath,
            [System.Management.Automation.Language.NullString]::Value
        )
    } else {
        [System.IO.File]::Move($draftPath, $ResultPath)
    }
}

function Read-AmeWindowsUiaProbeRecord {
    param(
        [Parameter(Mandatory = $true)] [string]$ResultPath,
        [Parameter(Mandatory = $true)] [string]$Token,
        [Parameter(Mandatory = $true)] [string]$Phase,
        [Parameter(Mandatory = $true)] [int]$TargetProcessId,
        [Parameter(Mandatory = $true)] [int]$ProbeProcessId
    )

    if ((Get-Item -LiteralPath $ResultPath).Length -gt 65536) {
        throw "Windows UIA probe evidence exceeds its bounded record size"
    }
    $record = [System.IO.File]::ReadAllText($ResultPath) | ConvertFrom-Json
    if (
        $record.format -cne "ame-windows-uia-probe-v2" -or
        $record.token -cne $Token -or $record.phase -cne $Phase -or
        $record.targetProcessId -ne $TargetProcessId -or
        $record.probeProcessId -ne $ProbeProcessId
    ) {
        throw "Windows UIA probe evidence does not match its owned request"
    }
    if (
        $record.status -cnotin @("progress", "complete") -or
        $record.stage -cnotin @(
            "loading-assemblies", "loading-uia-types", "loading-uia-client", "locating-window", "activating-cache", "finding-elements", "disposing-cache",
            "reading-properties", "asserting-contract", "complete"
        ) -or
        (($record.status -ceq "complete") -ne ($record.stage -ceq "complete"))
    ) {
        throw "Windows UIA probe evidence has an invalid progress state"
    }
    foreach ($field in @("targetProcessId", "probeProcessId", "attempt", "windowCount", "elementCount", "elapsedMilliseconds")) {
        $value = $record.$field
        if (($value -isnot [int] -and $value -isnot [long]) -or $value -lt 0 -or $value -gt [int]::MaxValue) {
            throw "Windows UIA probe evidence has an invalid numeric field"
        }
    }
    foreach ($field in @("lastMismatch", "failure")) {
        if ($null -ne $record.$field -and $record.$field -isnot [string]) {
            throw "Windows UIA probe evidence has an invalid diagnostic field"
        }
    }
    Assert-AmeWindowsUiaTimingRecord -Record $record
    return $record
}

function Complete-AmeWindowsUiaProbe {
    param(
        [Parameter(Mandatory = $true)] [string]$ResultPath,
        [Parameter(Mandatory = $true)] [string]$Token,
        [Parameter(Mandatory = $true)] [string]$Phase,
        [Parameter(Mandatory = $true)] [int]$TargetProcessId,
        [Parameter(Mandatory = $true)] [int]$ProbeProcessId,
        [AllowNull()] [Nullable[int]]$ExitCode,
        [AllowNull()] [Exception]$ProbeFailure,
        [Parameter(Mandatory = $true)] [object]$Cleanup
    )

    $cleanupFailure = Get-AmeWindowsAccessibilityCleanupFailure $Cleanup
    $result = $null
    $evidenceStatus = if ($ProbeProcessId -gt 0) { "missing" } else { "not-started" }
    try {
        if (Test-Path -LiteralPath $ResultPath -PathType Leaf) {
            $result = Read-AmeWindowsUiaProbeRecord `
                -ResultPath $ResultPath -Token $Token -Phase $Phase `
                -TargetProcessId $TargetProcessId -ProbeProcessId $ProbeProcessId
            $evidenceStatus = "verified"
        }
    } catch {
        $evidenceStatus = "invalid"
        if ($null -eq $ProbeFailure) { $ProbeFailure = $_.Exception }
    }
    if ($null -eq $ProbeFailure) {
        if ($ExitCode -ne 0) {
            $ProbeFailure = [InvalidOperationException]::new(
                "Windows UIA probe '$Phase' failed with a nonzero process exit"
            )
        } elseif ($null -eq $result -or $result.status -cne "complete") {
            $ProbeFailure = [InvalidOperationException]::new(
                "Windows UIA probe '$Phase' produced no complete result"
            )
        } elseif ($null -ne $result.failure) {
            $ProbeFailure = [InvalidOperationException]::new(
                "Windows UIA probe '$Phase' failed: $($result.failure)"
            )
        } elseif ($null -ne $cleanupFailure) {
            $ProbeFailure = $cleanupFailure
        }
    }
    if ($null -ne $ProbeFailure) {
        $ProbeFailure.Data["ameWindowsUiaProbeEvidenceStatus"] = $evidenceStatus
        $ProbeFailure.Data["ameWindowsUiaProbeProgress"] = $result
        if ($null -ne $cleanupFailure) {
            $ProbeFailure.Data["ameWindowsUiaProbeCleanupFailure"] = $cleanupFailure.Message
            $ProbeFailure.Data["ameWindowsUiaProbeCleanupFailures"] = @(
                Get-AmeWindowsAccessibilityCleanupRecords $Cleanup
            )
        }
        throw $ProbeFailure
    }
    return $result
}

function Format-AmeWindowsUiaProbeTranscript {
    param([Parameter(Mandatory = $true)] [object]$Record)

    return "AME_WINDOWS_UIA_PROBE_RESULT $($Record | ConvertTo-Json -Compress -Depth 4)"
}

function Get-AmeWindowsAccessibilityCompletionOutput {
    param(
        [string]$CapturedOutput, [string]$ProbeTranscript, [int]$ExitCode,
        [AllowNull()] [Exception]$RunFailure, [object]$Cleanup,
        [bool]$EvidencePersisted
    )

    $cleanupFailure = Get-AmeWindowsAccessibilityCleanupFailure $Cleanup
    $completion = [ordered]@{
        exitCode = $ExitCode
        runFailure = $(if ($null -ne $RunFailure) { $RunFailure.Message } else { $null })
        cleanupFailure = $(if ($null -ne $cleanupFailure) { $cleanupFailure.Message } else { $null })
        cleanupFailures = @(Get-AmeWindowsAccessibilityCleanupRecords $Cleanup)
        evidencePersisted = $EvidencePersisted
        primaryProcessExited = $Cleanup.ProcessExited
        ownedJobClosed = $Cleanup.JobClosed
        scratchStorage = [ordered]@{
            path = $Cleanup.ScratchPath
            disposition = $Cleanup.ScratchDisposition
            retentionReason = $Cleanup.ScratchRetentionReason
        }
        probeEvidenceStatus = $(
            if ($null -ne $RunFailure) { $RunFailure.Data["ameWindowsUiaProbeEvidenceStatus"] } else { $null }
        )
        probeProgress = $(
            if ($null -ne $RunFailure) { $RunFailure.Data["ameWindowsUiaProbeProgress"] } else { $null }
        )
        probeParentElapsedMilliseconds = $(
            if ($null -ne $RunFailure) {
                $RunFailure.Data["ameWindowsUiaProbeParentElapsedMilliseconds"]
            } else { $null }
        )
        probeCleanupFailure = $(
            if ($null -ne $RunFailure) { $RunFailure.Data["ameWindowsUiaProbeCleanupFailure"] } else { $null }
        )
        probeCleanupFailures = $(
            if ($null -ne $RunFailure) { $RunFailure.Data["ameWindowsUiaProbeCleanupFailures"] } else { $null }
        )
    } | ConvertTo-Json -Compress -Depth 6
    return @(
        $CapturedOutput.TrimEnd()
        $ProbeTranscript.TrimEnd()
        "AME_WINDOWS_ACCESSIBILITY_COMPLETION $completion"
    ) -join [Environment]::NewLine
}

function Complete-AmeWindowsAccessibilityRun {
    param(
        [Parameter(Mandatory = $true)] [string]$OutputPath,
        [AllowEmptyString()] [string]$CapturedOutput = "",
        [AllowEmptyString()] [string]$ProbeTranscript = "",
        [int]$ExitCode = 0,
        [AllowNull()] [Exception]$RunFailure,
        [AllowNull()] [Exception]$CleanupFailure,
        [AllowNull()] [object]$Cleanup
    )

    if ($null -eq $Cleanup) { $Cleanup = New-AmeWindowsAccessibilityCleanup }
    if ($null -ne $CleanupFailure) {
        $Cleanup.Failures.Add([pscustomobject]@{
            stage = "reported-cleanup"; exception = $CleanupFailure
        })
    }
    if ($null -eq $RunFailure -and $ExitCode -ne 0) {
        $RunFailure = [InvalidOperationException]::new(
            "Windows accessibility integration failed with exit code $ExitCode"
        )
    }
    $outputArguments = @{
        CapturedOutput = $CapturedOutput; ProbeTranscript = $ProbeTranscript
        ExitCode = $ExitCode; RunFailure = $RunFailure; Cleanup = $Cleanup
    }
    $Cleanup.EvidencePersisted = $false
    $output = Get-AmeWindowsAccessibilityCompletionOutput @outputArguments -EvidencePersisted $true
    Invoke-AmeWindowsAccessibilityCleanupStep $Cleanup "persist-completion" {
        Write-AmeWindowsAccessibilityCompletionEvidence $OutputPath "$output$([Environment]::NewLine)"
        $Cleanup.EvidencePersisted = $true
    }
    if (-not $Cleanup.EvidencePersisted) {
        $output = Get-AmeWindowsAccessibilityCompletionOutput @outputArguments -EvidencePersisted $false
    }
    Write-Host $output

    if ($null -ne $RunFailure) { throw $RunFailure }
    $failure = Get-AmeWindowsAccessibilityCleanupFailure $Cleanup
    if ($null -ne $failure) { throw $failure }
}
