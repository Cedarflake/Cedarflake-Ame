[CmdletBinding()]
param()

$ErrorActionPreference = "Stop"
. (Join-Path $PSScriptRoot "integration_windows_accessibility_evidence.ps1")

function Assert-AmeAccessibilityCleanup {
    param([bool]$Condition, [string]$Message)
    if (-not $Condition) { throw "Accessibility cleanup regression: $Message" }
}

function New-AmeAccessibilityCleanupFixture {
    param([string[]]$FailStages = @(), [bool]$Exits = $true)
    $fixture = [pscustomobject]@{
        Trace = [Collections.Generic.List[string]]::new()
        FailStages = $FailStages
        Exits = $Exits
        Output = ""
    }
    $job = [pscustomobject]@{ Fixture = $fixture }
    $job | Add-Member ScriptMethod Dispose {
        $this.Fixture.Trace.Add("close-owned-job")
        if ($this.Fixture.FailStages -contains "close-owned-job") { throw "job dispose failure" }
    }
    $process = [pscustomobject]@{ Fixture = $fixture }
    $process | Add-Member ScriptMethod WaitForExit {
        param([int]$Milliseconds)
        if ($Milliseconds -ne 5000) { throw "Cleanup changed its bounded wait" }
        $this.Fixture.Trace.Add("wait-owned-process")
        if ($this.Fixture.FailStages -contains "wait-owned-process") { throw "wait failure" }
        return $this.Fixture.Exits
    }
    $process | Add-Member ScriptMethod Dispose {
        $this.Fixture.Trace.Add("dispose-owned-process")
        if ($this.Fixture.FailStages -contains "dispose-owned-process") { throw "process dispose failure" }
    }
    return [pscustomobject]@{ State = $fixture; Job = $job; Process = $process }
}

$expectedStages = @(
    "close-owned-job", "wait-owned-process", "dispose-owned-process",
    "restore-environment:first", "restore-environment:second",
    "read-current-output", "capture-probe-transcript", "remove-owned-scratch"
)
foreach ($failedStage in $expectedStages) {
    & {
        param($FailureStage, $Expected)
        $fixture = New-AmeAccessibilityCleanupFixture @($FailureStage)
        $state = $fixture.State
        $cleanup = New-AmeWindowsAccessibilityCleanup
        function Restore-AmeWindowsAccessibilityEnvironment {
            param($Name, $PreviousValue)
            $stage = "restore-environment:$Name"
            $state.Trace.Add($stage)
            if ($state.FailStages -contains $stage) { throw "$stage failure" }
        }
        Complete-AmeWindowsAccessibilityCleanup -Cleanup $cleanup `
            -Job $fixture.Job -Process $fixture.Process `
            -EnvironmentValues ([ordered]@{ first = "old"; second = $null }) `
            -CaptureOutput {
                $state.Trace.Add("read-current-output")
                if ($FailureStage -ceq "read-current-output") { throw "output read failure" }
                "current Flutter output"
            } `
            -CaptureTranscript {
                $state.Trace.Add("capture-probe-transcript")
                if ($FailureStage -ceq "capture-probe-transcript") { throw "transcript failure" }
                "current verified phase"
            } `
            -RemoveScratch {
                $state.Trace.Add("remove-owned-scratch")
                if ($FailureStage -ceq "remove-owned-scratch") { throw "scratch failure" }
                $true
            }
        $retainedReason = switch ($FailureStage) {
            "close-owned-job" { "job-close-unconfirmed" }
            "wait-owned-process" { "process-exit-unconfirmed" }
            "read-current-output" { "output-capture-failed" }
            "capture-probe-transcript" { "probe-transcript-capture-failed" }
            default { $null }
        }
        if ($null -ne $retainedReason) {
            $Expected = @($Expected | Where-Object { $_ -cne "remove-owned-scratch" })
            Assert-AmeAccessibilityCleanup ($cleanup.ScratchDisposition -ceq "retained") "unsafe evidence deletion admitted"
            Assert-AmeAccessibilityCleanup ($cleanup.ScratchRetentionReason -ceq $retainedReason) "retention reason lost"
        }
        Assert-AmeAccessibilityCleanup (($state.Trace -join "|") -ceq ($Expected -join "|")) `
            "$FailureStage skipped or reordered a later cleanup action"
        Assert-AmeAccessibilityCleanup ($cleanup.Failures.Count -eq 1) "cleanup failure count"
        Assert-AmeAccessibilityCleanup ($cleanup.Failures[0].stage -ceq $FailureStage) "cleanup stage identity"
        if ($FailureStage -cne "read-current-output") {
            Assert-AmeAccessibilityCleanup ($cleanup.CapturedOutput -ceq "current Flutter output") "output lost"
        }
        if ($FailureStage -cne "capture-probe-transcript") {
            Assert-AmeAccessibilityCleanup ($cleanup.ProbeTranscript -ceq "current verified phase") "transcript lost"
        }
        function Write-AmeWindowsAccessibilityCompletionEvidence {
            param($OutputPath, $Content)
            $state.Output = $Content
        }
        function Write-Host { param($Object) }
        $runFailure = [InvalidOperationException]::new("original run failure")
        $caught = $null
        try {
            Complete-AmeWindowsAccessibilityRun -OutputPath "unused" `
                -CapturedOutput $cleanup.CapturedOutput -ProbeTranscript $cleanup.ProbeTranscript `
                -RunFailure $runFailure -Cleanup $cleanup
        } catch { $caught = $_.Exception }
        Assert-AmeAccessibilityCleanup ([object]::ReferenceEquals($caught, $runFailure)) "run error replaced"
        $record = ($state.Output -split "`r?`n" | Where-Object {
            $_.StartsWith("AME_WINDOWS_ACCESSIBILITY_COMPLETION ")
        }).Substring("AME_WINDOWS_ACCESSIBILITY_COMPLETION ".Length) | ConvertFrom-Json
        Assert-AmeAccessibilityCleanup ($record.evidencePersisted -eq $true) "successfully saved evidence not marked"
        Assert-AmeAccessibilityCleanup ($record.cleanupFailures[0].stage -ceq $FailureStage) "saved cleanup evidence lost"
        Assert-AmeAccessibilityCleanup ($record.scratchStorage.disposition -ceq $cleanup.ScratchDisposition) "saved scratch disposition lost"
        Assert-AmeAccessibilityCleanup ($record.scratchStorage.retentionReason -ceq $cleanup.ScratchRetentionReason) "saved scratch retention reason lost"
    } $failedStage $expectedStages
}

& {
    $fixture = New-AmeAccessibilityCleanupFixture @("close-owned-job", "wait-owned-process", "dispose-owned-process")
    $cleanup = New-AmeWindowsAccessibilityCleanup
    Close-AmeWindowsAccessibilityProcess $cleanup $fixture.Job $fixture.Process
    Assert-AmeAccessibilityCleanup ($cleanup.Failures.Count -eq 3) "multiple failures collapsed"
    Assert-AmeAccessibilityCleanup ($null -eq $cleanup.ProcessExited) "unknown exit was presented as exited"
    $timeoutFixture = New-AmeAccessibilityCleanupFixture -Exits $false
    $timeoutCleanup = New-AmeWindowsAccessibilityCleanup
    Close-AmeWindowsAccessibilityProcess $timeoutCleanup $timeoutFixture.Job $timeoutFixture.Process
    Assert-AmeAccessibilityCleanup ($timeoutCleanup.ProcessExited -eq $false) "unconfirmed exit was accepted"
    Assert-AmeAccessibilityCleanup ($timeoutCleanup.Failures[0].exception -is [TimeoutException]) "timeout type changed"
    Assert-AmeAccessibilityCleanup ($timeoutFixture.State.Trace[-1] -ceq "dispose-owned-process") "timeout leaked process handle"
}

foreach ($scenario in @("read-failure", "transcript-failure", "job-failure", "wait-failure", "wait-timeout")) {
    & {
        param($Scenario)
        $buildRoot = [IO.Path]::GetFullPath((Join-Path (Split-Path -Parent $PSScriptRoot) "build"))
        $scratchPath = [IO.Path]::GetFullPath((Join-Path $buildRoot "accessibility-cleanup-$([Guid]::NewGuid().ToString('N'))"))
        Assert-AmeAccessibilityCleanup ($scratchPath.StartsWith("$buildRoot$([IO.Path]::DirectorySeparatorChar)", [StringComparison]::OrdinalIgnoreCase)) "fixture escaped build"
        $null = New-Item -ItemType Directory -Path $scratchPath
        $logPath = Join-Path $scratchPath "flutter.log"
        $probePath = Join-Path $scratchPath "probe-01.json"
        $rawEvidence = "sole raw output must survive uncertain cleanup"
        $rawBytes = [Text.Encoding]::UTF8.GetBytes($rawEvidence)
        $probeBytes = [Text.Encoding]::UTF8.GetBytes('{"stage":"reading-properties"}')
        [IO.File]::WriteAllBytes($logPath, $rawBytes)
        [IO.File]::WriteAllBytes($probePath, $probeBytes)
        try {
            $failStages = switch ($Scenario) {
                "job-failure" { @("close-owned-job") }
                "wait-failure" { @("wait-owned-process") }
                default { @() }
            }
            $fixture = New-AmeAccessibilityCleanupFixture -FailStages $failStages -Exits ($Scenario -cne "wait-timeout")
            $cleanup = New-AmeWindowsAccessibilityCleanup
            $cleanup.ScratchPath = $scratchPath
            $deletion = [pscustomobject]@{ Calls = 0 }
            Complete-AmeWindowsAccessibilityCleanup -Cleanup $cleanup `
                -Job $fixture.Job -Process $fixture.Process -EnvironmentValues ([ordered]@{}) `
                -CaptureOutput {
                    if ($Scenario -ceq "read-failure") { throw [IO.IOException]::new("controlled log read failure") }
                    [IO.File]::ReadAllText($logPath)
                } `
                -CaptureTranscript {
                    if ($Scenario -ceq "transcript-failure") { throw [IO.IOException]::new("controlled transcript failure") }
                    "verified phase"
                } `
                -RemoveScratch {
                    $deletion.Calls += 1
                    [IO.File]::Delete($logPath)
                    [IO.File]::Delete($probePath)
                    [IO.Directory]::Delete($scratchPath)
                    $true
                }
            $expectedReason = switch ($Scenario) {
                "read-failure" { "output-capture-failed" }
                "transcript-failure" { "probe-transcript-capture-failed" }
                "job-failure" { "job-close-unconfirmed" }
                default { "process-exit-unconfirmed" }
            }
            Assert-AmeAccessibilityCleanup ($deletion.Calls -eq 0) "$Scenario invoked destructive evidence cleanup"
            Assert-AmeAccessibilityCleanup ([IO.File]::Exists($logPath)) "$Scenario lost raw evidence"
            Assert-AmeAccessibilityCleanup ([Convert]::ToBase64String([IO.File]::ReadAllBytes($logPath)) -ceq [Convert]::ToBase64String($rawBytes)) "$Scenario changed raw output bytes"
            Assert-AmeAccessibilityCleanup ([IO.File]::Exists($probePath)) "$Scenario lost raw phase evidence"
            Assert-AmeAccessibilityCleanup ([Convert]::ToBase64String([IO.File]::ReadAllBytes($probePath)) -ceq [Convert]::ToBase64String($probeBytes)) "$Scenario changed raw phase bytes"
            Assert-AmeAccessibilityCleanup ($cleanup.ScratchDisposition -ceq "retained") "$Scenario falsely claimed removal"
            Assert-AmeAccessibilityCleanup ($cleanup.ScratchPath -ceq $scratchPath) "$Scenario lost retained path"
            Assert-AmeAccessibilityCleanup ($cleanup.ScratchRetentionReason -ceq $expectedReason) "$Scenario lost retention reason"
            Assert-AmeAccessibilityCleanup ($fixture.State.Trace[-1] -ceq "dispose-owned-process") "$Scenario prevented independent handle release"
        } finally {
            if ([IO.File]::Exists($logPath)) { [IO.File]::Delete($logPath) }
            if ([IO.File]::Exists($probePath)) { [IO.File]::Delete($probePath) }
            if ([IO.Directory]::Exists($scratchPath)) { [IO.Directory]::Delete($scratchPath) }
        }
    } $scenario
}

& {
    $state = [pscustomobject]@{ Output = ""; Writes = 0 }
    function Write-AmeWindowsAccessibilityCompletionEvidence {
        param($OutputPath, $Content)
        $state.Writes += 1
        throw [IO.IOException]::new("completion write failed")
    }
    function Write-Host { param($Object) $state.Output = [string]$Object }
    $runFailure = [InvalidOperationException]::new("original native timeout")
    $cleanup = New-AmeWindowsAccessibilityCleanup
    $caught = $null
    try {
        Complete-AmeWindowsAccessibilityRun -OutputPath "unused" -CapturedOutput "current output" `
            -ProbeTranscript "verified phase" -RunFailure $runFailure -Cleanup $cleanup
    } catch { $caught = $_.Exception }
    Assert-AmeAccessibilityCleanup ([object]::ReferenceEquals($caught, $runFailure)) "persistence masked run failure"
    Assert-AmeAccessibilityCleanup ($state.Writes -eq 1) "persistence retried without authority"
    Assert-AmeAccessibilityCleanup ($state.Output.Contains("current output")) "console lost current output"
    Assert-AmeAccessibilityCleanup ($state.Output.Contains("verified phase")) "console lost native phases"
    $record = ($state.Output -split "`r?`n" | Where-Object {
        $_.StartsWith("AME_WINDOWS_ACCESSIBILITY_COMPLETION ")
    }).Substring("AME_WINDOWS_ACCESSIBILITY_COMPLETION ".Length) | ConvertFrom-Json
    Assert-AmeAccessibilityCleanup ($record.evidencePersisted -eq $false) "failed write claimed persisted"
    Assert-AmeAccessibilityCleanup ($record.runFailure -ceq "original native timeout") "original error absent in console evidence"
    Assert-AmeAccessibilityCleanup ($record.cleanupFailures[0].stage -ceq "persist-completion") "write error absent in console evidence"

    $caught = $null
    try { Complete-AmeWindowsAccessibilityRun -OutputPath "unused" } catch { $caught = $_.Exception }
    Assert-AmeAccessibilityCleanup ($caught -is [IO.IOException]) "standalone write failure became success"
}

& {
    $name = "AME_ACCESSIBILITY_CLEANUP_GUARDRAIL_$([Guid]::NewGuid().ToString('N'))"
    $prior = [Environment]::GetEnvironmentVariable($name, "Process")
    try {
        [Environment]::SetEnvironmentVariable($name, "changed", "Process")
        Restore-AmeWindowsAccessibilityEnvironment $name "original"
        Assert-AmeAccessibilityCleanup ([Environment]::GetEnvironmentVariable($name, "Process") -ceq "original") "existing environment not restored"
        Restore-AmeWindowsAccessibilityEnvironment $name $null
        Assert-AmeAccessibilityCleanup ($null -eq [Environment]::GetEnvironmentVariable($name, "Process")) "absent environment became empty"
        Assert-AmeAccessibilityCleanup (-not [Environment]::GetEnvironmentVariables("Process").Contains($name)) "deleted environment still enumerated"
    } finally { Restore-AmeWindowsAccessibilityEnvironment $name $prior }
}

# Execute the real facade's acquisition/finally/completion span without importing or starting a toolchain.
$mainPath = Join-Path $PSScriptRoot "integration_test_windows_accessibility.ps1"
$mainAst = [Management.Automation.Language.Parser]::ParseFile($mainPath, [ref]$null, [ref]$null)
$start = @($mainAst.EndBlock.Statements | Where-Object { $_.Extent.Text -ceq '$toolLock = $null' })
Assert-AmeAccessibilityCleanup ($start.Count -eq 1) "facade acquisition boundary missing"
$mainText = [IO.File]::ReadAllText($mainPath)
$runBody = [scriptblock]::Create($mainText.Substring($start[0].Extent.StartOffset))
foreach ($scenario in @("push", "pop", "run-and-release")) {
    & {
        param($Scenario, $RunBody)
        $trace = [Collections.Generic.List[string]]::new()
        $lockSentinel = [pscustomobject]@{ Owned = $true }
        $saved = [pscustomobject]@{ Output = "" }
        $repositoryRoot = "owned-repository"
        $OutputPath = "unused"
        $capturedOutputMode = $true
        $CapturedOutputPath = "captured-output"
        $CapturedProbeTranscriptPath = "captured-transcript"
        function Enter-AmeRepositoryToolLock { $trace.Add("lock"); return $lockSentinel }
        function Push-Location {
            param($Path)
            $trace.Add("push")
            if ($Scenario -ceq "push") { throw [InvalidOperationException]::new("push failed") }
        }
        function Pop-Location {
            $trace.Add("pop")
            if ($Scenario -cne "push") { throw [InvalidOperationException]::new("pop failed") }
        }
        function Exit-AmeRepositoryToolLock {
            param($Lock)
            Assert-AmeAccessibilityCleanup ([object]::ReferenceEquals($Lock, $lockSentinel)) "wrong lock released"
            $trace.Add("release")
            if ($Scenario -ceq "run-and-release") { throw [InvalidOperationException]::new("release failed") }
        }
        function Get-Content {
            param($LiteralPath, [switch]$Raw, $Encoding)
            if ($Scenario -ceq "run-and-release" -and $LiteralPath -ceq "captured-output") {
                return "Failed to update ui::AXTree"
            }
            return "current captured output"
        }
        function Assert-AmeWindowsUiaProbeTranscript { param($Transcript) }
        function Write-AmeWindowsAccessibilityCompletionEvidence {
            param($OutputPath, $Content)
            $trace.Add("persist")
            $saved.Output = $Content
        }
        function Write-Host { param($Object) }
        $caught = $null
        try { & $RunBody } catch { $caught = $_.Exception }
        $expected = if ($Scenario -ceq "push") { "lock|push|release|persist" } else { "lock|push|pop|release|persist" }
        Assert-AmeAccessibilityCleanup (($trace -join "|") -ceq $expected) "$Scenario bypassed actual facade finally"
        $record = ($saved.Output -split "`r?`n" | Where-Object {
            $_.StartsWith("AME_WINDOWS_ACCESSIBILITY_COMPLETION ")
        }).Substring("AME_WINDOWS_ACCESSIBILITY_COMPLETION ".Length) | ConvertFrom-Json
        if ($Scenario -ceq "push") {
            Assert-AmeAccessibilityCleanup ($caught.Message -ceq "push failed") "setup error lost"
        } elseif ($Scenario -ceq "pop") {
            Assert-AmeAccessibilityCleanup ($caught.Message -ceq "pop failed") "cleanup-only error hidden"
        } else {
            Assert-AmeAccessibilityCleanup ($caught.Message -ceq "Windows AccessibilityBridge rejected a semantics update") "run failure lost to outer cleanup"
            Assert-AmeAccessibilityCleanup (($record.cleanupFailures.stage -join "|") -ceq "pop-location|release-tool-lock") "outer failures not persisted"
        }
    } $scenario $runBody
}

Write-Host "Windows accessibility compiler-free cleanup guardrails passed."
