function New-AmeWindowsAccessibilityCleanup {
    [pscustomobject]@{
        Failures = [Collections.Generic.List[object]]::new()
        ProcessExited = $null
        JobClosed = $null
        CapturedOutput = ""
        OutputCaptured = $false
        ProbeTranscript = ""
        EvidencePersisted = $false
        ScratchPath = $null
        ScratchDisposition = "not-created"
        ScratchRetentionReason = $null
    }
}

function Invoke-AmeWindowsAccessibilityCleanupStep {
    param([object]$Cleanup, [string]$Stage, [scriptblock]$Action)

    try { & $Action | Out-Null } catch {
        $Cleanup.Failures.Add([pscustomobject]@{
            stage = $Stage
            exception = $_.Exception
        })
    }
}

function Close-AmeWindowsAccessibilityProcess {
    param([object]$Cleanup, [AllowNull()] [object]$Job, [AllowNull()] [object]$Process)

    if ($null -ne $Job) {
        Invoke-AmeWindowsAccessibilityCleanupStep $Cleanup "close-owned-job" {
            $Job.Dispose()
            $Cleanup.JobClosed = $true
        }
    }
    if ($null -ne $Process) {
        Invoke-AmeWindowsAccessibilityCleanupStep $Cleanup "wait-owned-process" {
            $Cleanup.ProcessExited = $Process.WaitForExit(5000)
            if (-not $Cleanup.ProcessExited) {
                throw [TimeoutException]::new(
                    "Windows accessibility process did not terminate after its Job Object closed"
                )
            }
        }
        Invoke-AmeWindowsAccessibilityCleanupStep $Cleanup "dispose-owned-process" { $Process.Dispose() }
    }
}

function Restore-AmeWindowsAccessibilityEnvironment {
    param([string]$Name, [AllowNull()] [object]$PreviousValue)

    if ($null -eq $PreviousValue) {
        [Environment]::SetEnvironmentVariable(
            $Name, [System.Management.Automation.Language.NullString]::Value, "Process"
        )
    } else {
        [Environment]::SetEnvironmentVariable($Name, $PreviousValue, "Process")
    }
}

function Invoke-AmeWindowsAccessibilityCacheScope {
    param([object]$CacheRequest, [scriptblock]$Read, [scriptblock]$BeforeDispose)

    $activeCache = $CacheRequest.Activate()
    $cleanup = New-AmeWindowsAccessibilityCleanup
    $failure = $null
    $result = $null
    try {
        $result = & $Read
    } catch {
        $failure = $_.Exception
    } finally {
        Invoke-AmeWindowsAccessibilityCleanupStep $cleanup "cache-dispose-progress" $BeforeDispose
        Invoke-AmeWindowsAccessibilityCleanupStep $cleanup "dispose-cache" { $activeCache.Dispose() }
    }
    if ($null -eq $failure) { $failure = Get-AmeWindowsAccessibilityCleanupFailure $cleanup }
    if ($null -ne $failure) {
        if ($cleanup.Failures.Count -gt 0) {
            $failure.Data["ameWindowsUiaCacheCleanupFailures"] = @(
                Get-AmeWindowsAccessibilityCleanupRecords $cleanup
            )
        }
        throw $failure
    }
    return ,$result
}

function Complete-AmeWindowsAccessibilityCleanup {
    param(
        [object]$Cleanup,
        [AllowNull()] [object]$Job,
        [AllowNull()] [object]$Process,
        [System.Collections.IDictionary]$EnvironmentValues,
        [scriptblock]$CaptureOutput,
        [scriptblock]$CaptureTranscript,
        [scriptblock]$RemoveScratch
    )

    Close-AmeWindowsAccessibilityProcess $Cleanup $Job $Process
    foreach ($name in $EnvironmentValues.Keys) {
        Invoke-AmeWindowsAccessibilityCleanupStep $Cleanup "restore-environment:$name" {
            Restore-AmeWindowsAccessibilityEnvironment $name $EnvironmentValues[$name]
        }
    }
    Invoke-AmeWindowsAccessibilityCleanupStep $Cleanup "read-current-output" {
        $Cleanup.CapturedOutput = & $CaptureOutput
        $Cleanup.OutputCaptured = $true
    }
    Invoke-AmeWindowsAccessibilityCleanupStep $Cleanup "capture-probe-transcript" {
        $Cleanup.ProbeTranscript = & $CaptureTranscript
    }
    $Cleanup.ScratchDisposition = "retained"
    if ($null -ne $Job -and $Cleanup.JobClosed -ne $true) {
        $Cleanup.ScratchRetentionReason = "job-close-unconfirmed"
    } elseif (($null -ne $Job -or $null -ne $Process) -and $Cleanup.ProcessExited -ne $true) {
        $Cleanup.ScratchRetentionReason = "process-exit-unconfirmed"
    } elseif (-not $Cleanup.OutputCaptured) {
        $Cleanup.ScratchRetentionReason = "output-capture-failed"
    } elseif (@($Cleanup.Failures | Where-Object { $_.stage -ceq "capture-probe-transcript" }).Count -gt 0) {
        $Cleanup.ScratchRetentionReason = "probe-transcript-capture-failed"
    } else {
        $Cleanup.ScratchRetentionReason = "scratch-removal-failed"
        Invoke-AmeWindowsAccessibilityCleanupStep $Cleanup "remove-owned-scratch" {
            if ((& $RemoveScratch) -eq $true) {
                $Cleanup.ScratchDisposition = "removed"
                $Cleanup.ScratchRetentionReason = $null
            } else {
                $Cleanup.ScratchRetentionReason = "scratch-removal-unconfirmed"
            }
        }
    }
}

function Get-AmeWindowsAccessibilityCleanupFailure {
    param([object]$Cleanup)

    if ($Cleanup.Failures.Count -gt 0) { return $Cleanup.Failures[0].exception }
}

function Get-AmeWindowsAccessibilityCleanupRecords {
    param([object]$Cleanup)

    foreach ($failure in $Cleanup.Failures) {
        [ordered]@{ stage = $failure.stage; message = $failure.exception.Message }
    }
}

function Write-AmeWindowsAccessibilityCompletionEvidence {
    param([string]$OutputPath, [string]$Content)

    $resolved = [IO.Path]::GetFullPath($OutputPath)
    $directory = Split-Path -Parent $resolved
    New-Item -ItemType Directory -Path $directory -Force | Out-Null
    [IO.File]::WriteAllText($resolved, $Content, [Text.UTF8Encoding]::new($false))
}
