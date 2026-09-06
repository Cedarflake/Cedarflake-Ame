$ErrorActionPreference = "Stop"
. (Join-Path $PSScriptRoot "quality_common.ps1")
. (Join-Path $PSScriptRoot "integration_windows_accessibility_process.ps1")
. (Join-Path $PSScriptRoot "integration_windows_accessibility_evidence.ps1")

$repositoryRoot = Get-AmeRepositoryRoot
$buildRoot = [System.IO.Path]::GetFullPath((Join-Path $repositoryRoot "build"))
$scratchRoot = [System.IO.Path]::GetFullPath(
    (Join-Path $buildRoot "windows-accessibility-guardrails-$PID")
)
$buildPrefix = "$buildRoot$([System.IO.Path]::DirectorySeparatorChar)"
if (-not $scratchRoot.StartsWith(
    $buildPrefix,
    [System.StringComparison]::OrdinalIgnoreCase
)) {
    throw "Accessibility guardrail storage must remain inside build"
}

$canary = Join-Path $PSScriptRoot "integration_test_windows_accessibility.ps1"
$cleanOutput = Join-Path $scratchRoot "clean.log"
$invalidOutput = Join-Path $scratchRoot "invalid.log"
$cleanProbeTranscript = Join-Path $scratchRoot "clean-probe.log"
$incompleteProbeTranscript = Join-Path $scratchRoot "incomplete-probe.log"
$outOfOrderProbeTranscript = Join-Path $scratchRoot "out-of-order-probe.log"
$canaryOutput = Join-Path $scratchRoot "canary.log"
$runnerPath = Join-Path $repositoryRoot "build\windows\x64\runner\Debug\cedarflake_ame.exe"
$utf8 = [System.Text.UTF8Encoding]::new($false)

function Assert-AmeAccessibilityFailureEvidence {
    param(
        [Parameter(Mandatory = $true)]
        [System.Exception]$Failure,
        [System.Exception]$CleanupFailure
    )

    $staleMarker = "previous-success-must-not-survive"
    [System.IO.File]::WriteAllText($canaryOutput, "$staleMarker`n", $utf8)
    $currentOutput = "current-run Flutter stderr"
    $currentPhase = "AME_WINDOWS_UIA_PHASE phase=application-ready result=ok"
    $observedFailure = $null
    try {
        Complete-AmeWindowsAccessibilityRun `
            -OutputPath $canaryOutput `
            -CapturedOutput $currentOutput `
            -ProbeTranscript $currentPhase `
            -RunFailure $Failure `
            -CleanupFailure $CleanupFailure
    } catch {
        $observedFailure = $_.Exception
    }
    if ($null -eq $observedFailure -or $observedFailure.Message -cne $Failure.Message) {
        throw "Accessibility completion did not preserve the original run failure"
    }
    $evidence = [System.IO.File]::ReadAllText($canaryOutput)
    if (
        $evidence.Contains($staleMarker) -or
        -not $evidence.Contains($currentOutput) -or
        -not $evidence.Contains($currentPhase)
    ) {
        throw "Accessibility failure did not replace stale success with current evidence"
    }
    $completionPrefix = "AME_WINDOWS_ACCESSIBILITY_COMPLETION "
    $completionLines = @(
        $evidence -split "`r?`n" | Where-Object { $_.StartsWith($completionPrefix) }
    )
    if ($completionLines.Count -ne 1) {
        throw "Accessibility failure did not retain one completion record"
    }
    $completion = $completionLines[0].Substring($completionPrefix.Length) | ConvertFrom-Json
    $expectedCleanup = if ($null -ne $CleanupFailure) { $CleanupFailure.Message }
    if (
        $completion.runFailure -cne $Failure.Message -or
        $completion.cleanupFailure -cne $expectedCleanup -or
        $completion.probeCleanupFailure -cne $Failure.Data["ameWindowsUiaProbeCleanupFailure"]
    ) {
        throw "Accessibility completion did not retain the run and cleanup failure metadata"
    }
    $expectedProgress = $Failure.Data["ameWindowsUiaProbeProgress"]
    if ($null -ne $expectedProgress -and (
        $completion.probeEvidenceStatus -cne "verified" -or
        $completion.probeProgress.stage -cne $expectedProgress.stage -or
        $completion.probeProgress.lastMismatch -cne $expectedProgress.lastMismatch
    )) {
        throw "Accessibility completion lost the verified probe stage or last mismatch"
    }
}

try {
    New-Item -ItemType Directory -Path $scratchRoot -Force | Out-Null
    $activationTypeBeforeImport = "AmeWindowsAccessibilityActivation" -as [type]
    . (Join-Path $PSScriptRoot "integration_windows_accessibility_activation.ps1")
    if (("AmeWindowsAccessibilityActivation" -as [type]) -ne $activationTypeBeforeImport) {
        throw "Importing accessibility activation compiled native interop eagerly"
    }
    Initialize-AmeWindowsAccessibilityActivation
    $invalidProcessRejected = $false
    try {
        Get-AmeWindowsAccessibilityActivationTargets -TargetProcessId 0 | Out-Null
    } catch {
        $invalidProcessRejected = $_.Exception.ToString().Contains("targetProcessId")
    }
    if (-not $invalidProcessRejected) {
        throw "Accessibility activation accepted an invalid process identity"
    }
    $invalidWindowRejected = $false
    try {
        Enable-AmeWindowsAccessibilityNativeView -TargetProcessId $PID -NativeViewHandle ([IntPtr]::Zero)
    } catch {
        $invalidWindowRejected = $_.Exception.ToString().Contains("target no longer belongs")
    }
    if (-not $invalidWindowRejected) {
        throw "Accessibility activation accepted an unowned window handle"
    }
    $runnerProcessIdsBefore = @(
        Get-Process -Name "cedarflake_ame" -ErrorAction SilentlyContinue |
            Where-Object {
                $_.Path -and
                $_.Path.Equals(
                    $runnerPath,
                    [System.StringComparison]::OrdinalIgnoreCase
                )
            } |
            Select-Object -ExpandProperty Id
    )
    [System.IO.File]::WriteAllText($cleanOutput, "All tests passed.`n", $utf8)
    $probePhases = @(
        "native-semantics-ready"
        "application-ready"
        "source-menu-open"
        "source-menu-closed"
        "square-range-loading"
        "viewer-open"
        "viewer-menu-open"
        "viewer-menu-closed"
        "settings-menu-open"
        "settings-menu-closed"
    )
    $probeTranscript = @(
        foreach ($phase in $probePhases) {
            "AME_WINDOWS_UIA_PHASE phase=$phase result=ok"
        }
    ) -join "`n"
    [System.IO.File]::WriteAllText(
        $cleanProbeTranscript,
        "$probeTranscript`n",
        $utf8
    )
    [System.IO.File]::WriteAllText(
        $incompleteProbeTranscript,
        "AME_WINDOWS_UIA_PHASE phase=application-ready result=ok`n",
        $utf8
    )
    $outOfOrderPhases = @($probePhases)
    $outOfOrderPhases[2] = "source-menu-closed"
    $outOfOrderPhases[3] = "source-menu-open"
    $outOfOrderTranscript = @(
        foreach ($phase in $outOfOrderPhases) {
            "AME_WINDOWS_UIA_PHASE phase=$phase result=ok"
        }
    ) -join "`n"
    [System.IO.File]::WriteAllText(
        $outOfOrderProbeTranscript,
        "$outOfOrderTranscript`n",
        $utf8
    )
    & $canary `
        -CapturedOutputPath $cleanOutput `
        -CapturedProbeTranscriptPath $cleanProbeTranscript `
        -OutputPath $canaryOutput
    $runnerProcessIdsAfter = @(
        Get-Process -Name "cedarflake_ame" -ErrorAction SilentlyContinue |
            Where-Object {
                $_.Path -and
                $_.Path.Equals(
                    $runnerPath,
                    [System.StringComparison]::OrdinalIgnoreCase
                )
            } |
            Select-Object -ExpandProperty Id
    )
    if (Compare-Object $runnerProcessIdsBefore $runnerProcessIdsAfter) {
        throw "Captured-output validation changed runner process ownership"
    }

    $missingProbeRejected = $false
    try {
        & $canary `
            -CapturedOutputPath $cleanOutput `
            -OutputPath $canaryOutput
    } catch {
        $missingProbeRejected = $_.Exception.Message -match `
            "requires both app output"
    }
    if (-not $missingProbeRejected) {
        throw "Windows accessibility canary accepted output without native UIA evidence"
    }

    $incompleteProbeRejected = $false
    try {
        & $canary `
            -CapturedOutputPath $cleanOutput `
            -CapturedProbeTranscriptPath $incompleteProbeTranscript `
            -OutputPath $canaryOutput
    } catch {
        $incompleteProbeRejected = $_.Exception.Message -match `
            "probe transcript is incomplete"
    }
    if (-not $incompleteProbeRejected) {
        throw "Windows accessibility canary accepted an incomplete native UIA sequence"
    }

    $missingActivationTranscript = @(
        foreach ($phase in $probePhases | Select-Object -Skip 1) {
            "AME_WINDOWS_UIA_PHASE phase=$phase result=ok"
        }
    ) -join "`n"
    [System.IO.File]::WriteAllText(
        $incompleteProbeTranscript, "$missingActivationTranscript`n", $utf8
    )
    $missingActivationRejected = $false
    try {
        & $canary `
            -CapturedOutputPath $cleanOutput `
            -CapturedProbeTranscriptPath $incompleteProbeTranscript `
            -OutputPath $canaryOutput
    } catch {
        $missingActivationRejected = $_.Exception.Message -match `
            "probe transcript is incomplete"
    }
    if (-not $missingActivationRejected) {
        throw "Windows accessibility canary accepted tree phases without native activation"
    }

    $outOfOrderProbeRejected = $false
    try {
        & $canary `
            -CapturedOutputPath $cleanOutput `
            -CapturedProbeTranscriptPath $outOfOrderProbeTranscript `
            -OutputPath $canaryOutput
    } catch {
        $outOfOrderProbeRejected = $_.Exception.Message -match `
            "was not the expected"
    }
    if (-not $outOfOrderProbeRejected) {
        throw "Windows accessibility canary accepted an out-of-order native UIA sequence"
    }

    [System.IO.File]::WriteAllText(
        $invalidOutput,
        "[ERROR] Failed to update ui::AXTree, error: node error`n",
        $utf8
    )
    $invalidOutputRejected = $false
    try {
        & $canary `
            -CapturedOutputPath $invalidOutput `
            -CapturedProbeTranscriptPath $cleanProbeTranscript `
            -OutputPath $canaryOutput
    } catch {
        $invalidOutputRejected = $_.Exception.Message -match `
            "AccessibilityBridge rejected"
    }
    if (-not $invalidOutputRejected) {
        throw "Windows accessibility canary accepted an AXTree failure"
    }
    Assert-AmeAccessibilityFailureEvidence `
        -Failure ([System.InvalidOperationException]::new("controlled UIA phase failure")) `
        -CleanupFailure ([System.IO.IOException]::new("controlled scratch cleanup failure"))
    $probeCleanupFixture = [System.TimeoutException]::new("controlled probe timeout")
    $probeCleanupFixture.Data["ameWindowsUiaProbeCleanupFailure"] = "controlled probe cleanup timeout"
    Assert-AmeAccessibilityFailureEvidence -Failure $probeCleanupFixture

    $canarySource = [System.IO.File]::ReadAllText($canary)
    $probeSource = [System.IO.File]::ReadAllText(
        (Join-Path $PSScriptRoot "integration_windows_accessibility_probe.ps1")
    )
    $processSource = [System.IO.File]::ReadAllText(
        (Join-Path $PSScriptRoot "integration_windows_accessibility_process.ps1")
    )
    if ($probeSource -match 'AddSeconds\(8\)' -or $probeSource -match '\$deadline') {
        throw "The native probe introduced a second deadline owner"
    }
    if ($probeSource.Contains('$element.Current')) {
        throw "Native snapshot reads must not issue per-property cross-process requests"
    }
    $completeCanarySource = $canarySource + $probeSource + $processSource
    foreach ($requiredToken in @(
        "UIAutomationClient",
        "TreeScope]::Children",
        "TreeScope]::Subtree",
        "CacheRequest]::new()",
        "TreeScope]::Element",
        "RawViewCondition",
        "AutomationElementMode]::None",
        '$element.Cached',
        "AmeWindowsAccessibilityProcessJob",
        "JobObjectLimitKillOnJobClose",
        "CreateSuspended",
        "Invoke-AmeWindowsAccessibilityProbe",
        "Complete-AmeWindowsAccessibilityRun",
        "TimeoutSeconds = 900",
        "CEDARFLAKE_AME_WINDOWS_UIA_PROBE_DIRECTORY",
        "CEDARFLAKE_AME_WINDOWS_UIA_PROBE_TOKEN"
    )) {
        if (-not $completeCanarySource.Contains($requiredToken)) {
            throw "Windows accessibility canary lost required native contract '$requiredToken'"
        }
    }

    $probeFixture = Join-Path $scratchRoot "probe.ps1"
    $successProbeResult = Join-Path $scratchRoot "probe-success.json"
    $failedProbeResult = Join-Path $scratchRoot "probe-failed.json"
    $invalidProbeResult = Join-Path $scratchRoot "probe-invalid.json"
    $blockedProbeResult = Join-Path $scratchRoot "probe-blocked.json"
    $progressFixture = Join-Path $scratchRoot "progress.json"
    foreach ($stage in @(
        "loading-assemblies", "locating-window", "finding-elements",
        "reading-properties", "asserting-contract"
    )) {
        Write-AmeWindowsUiaProbeRecord `
            -ResultPath $progressFixture -Token "progress-fixture" -Phase "application-ready" `
            -TargetProcessId $PID -ProbeProcessId $PID -Stage $stage -Attempt 1 `
            -WindowCount 1 -ElementCount 3 -LastMismatch "controlled previous mismatch"
        $progress = Read-AmeWindowsUiaProbeRecord `
            -ResultPath $progressFixture -Token "progress-fixture" -Phase "application-ready" `
            -TargetProcessId $PID -ProbeProcessId $PID
        if ($progress.stage -cne $stage -or $progress.lastMismatch -cne "controlled previous mismatch") {
            throw "Native progress evidence did not round-trip its stage and mismatch"
        }
    }
    $probeFixtureSource = @'
param([int]$TargetProcessId, [string]$Phase, [string]$ResultPath, [string]$Token)
$ErrorActionPreference = "Stop"
if ([System.Threading.Thread]::CurrentThread.GetApartmentState() -ne 'MTA') {
    throw "The owned probe process did not establish an MTA thread"
}
. '__AME_PROBE_EVIDENCE__'
$recordArguments = @{
    ResultPath = $ResultPath
    Token = $Token
    Phase = $Phase
    TargetProcessId = $TargetProcessId
    ProbeProcessId = $PID
}
if ($Phase -ceq "blocked-fixture") {
    $hostExecutable = (Get-Process -Id $PID).Path
    $child = Start-Process -FilePath $hostExecutable -WindowStyle Hidden -PassThru `
        -ArgumentList '-NoLogo -NoProfile -NonInteractive -Command "Start-Sleep -Seconds 30"'
    [System.IO.File]::WriteAllText(
        "$ResultPath.pids",
        (@{ probeProcessId = $PID; descendantProcessId = $child.Id } | ConvertTo-Json),
        [System.Text.UTF8Encoding]::new($false)
    )
    Write-AmeWindowsUiaProbeRecord @recordArguments `
        -Stage "asserting-contract" -Attempt 1 -WindowCount 1 -ElementCount 3 `
        -LastMismatch "controlled expected native button is missing"
    Write-AmeWindowsUiaProbeRecord @recordArguments `
        -Stage "reading-properties" -Attempt 2 -WindowCount 1 -ElementCount 7 `
        -LastMismatch "controlled expected native button is missing"
    [System.IO.File]::WriteAllText(
        [System.IO.Path]::ChangeExtension($ResultPath, "tmp"),
        '{',
        [System.Text.UTF8Encoding]::new($false)
    )
    Start-Sleep -Seconds 30
} else {
    $failure = $null
    if ($Phase -ceq "failure-fixture") {
        $failure = "controlled native phase failure"
    }
    if ($Phase -ceq "invalid-fixture") {
        $recordArguments.Token = "wrong-owner"
    }
    Write-AmeWindowsUiaProbeRecord @recordArguments `
        -Status "complete" -Stage "complete" -Attempt 1 -Failure $failure
}
'@
    $probeFixtureSource = $probeFixtureSource.Replace(
        "__AME_PROBE_EVIDENCE__",
        (Join-Path $PSScriptRoot "integration_windows_accessibility_evidence.ps1").Replace("'", "''")
    )
    [System.IO.File]::WriteAllText($probeFixture, $probeFixtureSource, $utf8)
    Invoke-AmeWindowsAccessibilityProbe `
        -ProbeScriptPath $probeFixture `
        -TargetProcessId $PID `
        -Phase "success-fixture" `
        -ResultPath $successProbeResult `
        -Token "owned-probe-fixture" `
        -Timeout ([TimeSpan]::FromSeconds(5))

    foreach ($case in @(
        @{
            Phase = "failure-fixture"
            Result = $failedProbeResult
            Message = "Windows UIA probe 'failure-fixture' failed: controlled native phase failure"
            EvidenceStatus = "verified"
        }
        @{
            Phase = "invalid-fixture"
            Result = $invalidProbeResult
            Message = "Windows UIA probe evidence does not match its owned request"
            EvidenceStatus = "invalid"
        }
    )) {
        $observedFailure = $null
        try {
            Invoke-AmeWindowsAccessibilityProbe `
                -ProbeScriptPath $probeFixture -TargetProcessId $PID `
                -Phase $case.Phase -ResultPath $case.Result -Token "owned-probe-fixture" `
                -Timeout ([TimeSpan]::FromSeconds(5))
        } catch {
            $observedFailure = $_.Exception
        }
        if (
            $null -eq $observedFailure -or $observedFailure.Message -cne $case.Message -or
            $observedFailure.Data["ameWindowsUiaProbeEvidenceStatus"] -cne $case.EvidenceStatus
        ) {
            throw "The native probe did not preserve failure or reject unowned evidence"
        }
    }

    $blockedProbeRejected = $false
    $blockedProbeFailure = $null
    $probeClock = [System.Diagnostics.Stopwatch]::StartNew()
    try {
        Invoke-AmeWindowsAccessibilityProbe `
            -ProbeScriptPath $probeFixture `
            -TargetProcessId $PID `
            -Phase "blocked-fixture" `
            -ResultPath $blockedProbeResult `
            -Token "owned-probe-fixture" `
            -Timeout ([TimeSpan]::FromSeconds(5))
    } catch {
        $blockedProbeFailure = $_.Exception
        $blockedProbeRejected = $_.Exception.Message -ceq (
            "Windows UIA probe 'blocked-fixture' exceeded its parent deadline"
        )
    } finally {
        $probeClock.Stop()
    }
    if (-not $blockedProbeRejected -or $probeClock.Elapsed.TotalSeconds -gt 11) {
        throw "A blocked native probe escaped the parent deadline and cleanup bound"
    }
    $blockedProgress = $blockedProbeFailure.Data["ameWindowsUiaProbeProgress"]
    if (
        $blockedProbeFailure.Data["ameWindowsUiaProbeEvidenceStatus"] -cne "verified" -or
        $null -eq $blockedProgress -or
        $blockedProgress.status -cne "progress" -or
        $blockedProgress.stage -cne "reading-properties" -or
        $blockedProgress.attempt -ne 2 -or
        $blockedProgress.elementCount -ne 7 -or
        $blockedProgress.lastMismatch -cne "controlled expected native button is missing"
    ) {
        throw "The parent deadline lost the last complete progress record or accepted a partial draft"
    }
    if (-not (Test-Path -LiteralPath $blockedProbeResult -PathType Leaf)) {
        throw "The blocking probe fixture never established its owned descendant"
    }
    $blockedProcesses = Get-Content -LiteralPath "$blockedProbeResult.pids" -Raw -Encoding UTF8 |
        ConvertFrom-Json
    $ownedProcessIds = @(
        [int]$blockedProcesses.probeProcessId
        [int]$blockedProcesses.descendantProcessId
    )
    if (@($ownedProcessIds | Where-Object { $_ -le 0 -or $_ -eq $PID }).Count -ne 0) {
        throw "The blocking probe fixture reported an invalid process identity"
    }
    $cleanupDeadline = [DateTime]::UtcNow.AddSeconds(5)
    do {
        $remainingProcesses = @(
            Get-Process -Id $ownedProcessIds -ErrorAction SilentlyContinue
        )
        if ($remainingProcesses.Count -eq 0) {
            break
        }
        Start-Sleep -Milliseconds 25
    } while ([DateTime]::UtcNow -lt $cleanupDeadline)
    if ($remainingProcesses.Count -ne 0) {
        throw "The parent deadline left a probe or its owned descendant running"
    }
    Assert-AmeAccessibilityFailureEvidence -Failure $blockedProbeFailure
} finally {
    if (Test-Path -LiteralPath $scratchRoot) {
        Remove-Item -LiteralPath $scratchRoot -Recurse -Force
    }
}
