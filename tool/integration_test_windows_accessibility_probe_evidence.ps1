[CmdletBinding()]
param()

$ErrorActionPreference = "Stop"
. (Join-Path $PSScriptRoot "integration_windows_accessibility_evidence.ps1")

function Assert-AmeUiaProbeEvidence {
    param([bool]$Condition, [string]$Message)
    if (-not $Condition) { throw "Accessibility probe evidence regression: $Message" }
}

$scratchRoot = Join-Path ([IO.Path]::GetTempPath()) "ame-uia-evidence-$([Guid]::NewGuid().ToString('N'))"
$ownedFiles = [Collections.Generic.List[string]]::new()
$recordPath = Join-Path $scratchRoot "probe.json"
$completionPath = Join-Path $scratchRoot "completion.log"
$ownedFiles.Add($recordPath)
$ownedFiles.Add([IO.Path]::ChangeExtension($recordPath, "tmp"))
$ownedFiles.Add($completionPath)
$utf8 = [Text.UTF8Encoding]::new($false)
$diagnostic = "prior mismatch with a quote ' and newline`nsecond line"
$baseline = @{
    ResultPath = $recordPath; Token = "owned-token"; Phase = "application-ready"
    TargetProcessId = 101; ProbeProcessId = 202; Status = "complete"; Stage = "complete"
    Attempt = 3; WindowCount = 1; ElementCount = 179; ElapsedMilliseconds = 1632
    LastMismatch = $diagnostic; Failure = $null
}
$request = @{
    ResultPath = $recordPath; Token = "owned-token"; Phase = "application-ready"
    TargetProcessId = 101; ProbeProcessId = 202; ExitCode = 0
}

New-Item -ItemType Directory -Path $scratchRoot | Out-Null
try {
    foreach ($case in @(
        @{ Name = "token"; Change = @{ Token = "other" }; Evidence = "invalid" }
        @{ Name = "phase"; Change = @{ Phase = "other" }; Evidence = "invalid" }
        @{ Name = "target"; Change = @{ TargetProcessId = 999 }; Evidence = "invalid" }
        @{ Name = "probe"; Change = @{ ProbeProcessId = 999 }; Evidence = "invalid" }
        @{ Name = "partial"; Change = @{ Status = "progress"; Stage = "finding-elements" }; Evidence = "verified" }
        @{ Name = "failure"; Change = @{ Failure = "query failed" }; Evidence = "verified" }
        @{ Name = "exit"; Change = @{}; Evidence = "verified" }
        @{ Name = "cleanup"; Change = @{}; Evidence = "verified" }
        @{ Name = "missing"; Change = @{}; Evidence = "missing" }
    )) {
        $record = $baseline.Clone()
        foreach ($key in $case.Change.Keys) { $record[$key] = $case.Change[$key] }
        Write-AmeWindowsUiaProbeRecord @record
        if ($case.Name -ceq "missing") { Remove-Item -LiteralPath $recordPath }
        $arguments = $request.Clone()
        if ($case.Name -ceq "exit") { $arguments.ExitCode = 5 }
        $cleanup = New-AmeWindowsAccessibilityCleanup
        if ($case.Name -ceq "cleanup") {
            $cleanup.Failures.Add([pscustomobject]@{
                stage = "dispose-job"; exception = [IO.IOException]::new("cleanup failed")
            })
        }
        $emitted = [Collections.Generic.List[object]]::new()
        $caught = $null
        try {
            Complete-AmeWindowsUiaProbe @arguments -Cleanup $cleanup |
                ForEach-Object { $emitted.Add($_) }
        } catch { $caught = $_.Exception }
        Assert-AmeUiaProbeEvidence ($null -ne $caught) "$($case.Name) became success"
        Assert-AmeUiaProbeEvidence ($emitted.Count -eq 0) "$($case.Name) emitted a success record before failure"
        Assert-AmeUiaProbeEvidence ($caught.Data["ameWindowsUiaProbeEvidenceStatus"] -ceq $case.Evidence) "$($case.Name) lost failure evidence classification"
        if ($case.Evidence -ceq "verified") {
            Assert-AmeUiaProbeEvidence ($caught.Data["ameWindowsUiaProbeProgress"].elementCount -eq 179) "$($case.Name) lost its original progress"
        }
    }

    $original = [InvalidOperationException]::new("original parent deadline")
    $record = $baseline.Clone()
    $record.Token = "wrong-owner"
    Write-AmeWindowsUiaProbeRecord @record
    $cleanup = New-AmeWindowsAccessibilityCleanup
    $cleanup.Failures.Add([pscustomobject]@{
        stage = "dispose-job"; exception = [IO.IOException]::new("cleanup failed")
    })
    $caught = $null
    try {
        Complete-AmeWindowsUiaProbe @request -ProbeFailure $original -Cleanup $cleanup | Out-Null
    } catch { $caught = $_.Exception }
    Assert-AmeUiaProbeEvidence ([object]::ReferenceEquals($caught, $original)) "invalid evidence or cleanup replaced the original failure"
    Assert-AmeUiaProbeEvidence ($caught.Data["ameWindowsUiaProbeEvidenceStatus"] -ceq "invalid") "original failure lost invalid evidence status"
    Assert-AmeUiaProbeEvidence ($caught.Data["ameWindowsUiaProbeCleanupFailures"].Count -eq 1) "original failure lost cleanup detail"

    Write-AmeWindowsUiaProbeRecord @baseline
    $returned = @(Complete-AmeWindowsUiaProbe @request -Cleanup (New-AmeWindowsAccessibilityCleanup))
    Assert-AmeUiaProbeEvidence ($returned.Count -eq 1) "success emitted multiple objects"
    $result = $returned[0]
    Assert-AmeUiaProbeEvidence ($result -is [pscustomobject]) "success changed the record type"
    $line = Format-AmeWindowsUiaProbeTranscript -Record $result
    $prefix = "AME_WINDOWS_UIA_PROBE_RESULT "
    Assert-AmeUiaProbeEvidence (($line -split "`r?`n").Count -eq 1) "diagnostic text injected transcript lines"
    $roundTrip = $line.Substring($prefix.Length) | ConvertFrom-Json
    foreach ($field in @("format", "token", "phase", "targetProcessId", "probeProcessId", "status", "stage", "attempt", "windowCount", "elementCount", "elapsedMilliseconds", "lastMismatch", "failure")) {
        Assert-AmeUiaProbeEvidence ($roundTrip.$field -ceq $result.$field) "success lost $field"
    }

    $mainPath = Join-Path $PSScriptRoot "integration_test_windows_accessibility.ps1"
    $mainAst = [Management.Automation.Language.Parser]::ParseFile($mainPath, [ref]$null, [ref]$null)
    $phaseAssignment = @($mainAst.EndBlock.Statements | Where-Object {
        $_ -is [Management.Automation.Language.AssignmentStatementAst] -and
        $_.Left.Extent.Text -ceq '$expectedProbePhases'
    })
    Assert-AmeUiaProbeEvidence ($phaseAssignment.Count -eq 1) "phase roster missing"
    . ([scriptblock]::Create($phaseAssignment[0].Extent.Text))
    Assert-AmeUiaProbeEvidence ($expectedProbePhases.Count -eq 10) "phase roster weakened"
    $assertion = @($mainAst.FindAll({
        param($node)
        $node -is [Management.Automation.Language.FunctionDefinitionAst] -and
        $node.Name -ceq "Assert-AmeWindowsUiaProbeTranscript"
    }, $false))
    Assert-AmeUiaProbeEvidence ($assertion.Count -eq 1) "transcript assertion missing"
    $assertTranscript = $assertion[0].Body.GetScriptBlock()
    $cleanupCall = @($mainAst.FindAll({
        param($node)
        $node -is [Management.Automation.Language.CommandAst] -and
        $node.GetCommandName() -ceq "Complete-AmeWindowsAccessibilityCleanup"
    }, $true))
    Assert-AmeUiaProbeEvidence ($cleanupCall.Count -eq 1) "facade cleanup call is ambiguous"
    $captureTranscript = $null
    $elements = $cleanupCall[0].CommandElements
    for ($index = 0; $index -lt $elements.Count - 1; $index += 1) {
        if ($elements[$index] -is [Management.Automation.Language.CommandParameterAst] -and
            $elements[$index].ParameterName -ceq "CaptureTranscript") {
            $captureTranscript = $elements[$index + 1].ScriptBlock.GetScriptBlock()
        }
    }
    Assert-AmeUiaProbeEvidence ($null -ne $captureTranscript) "real transcript capture is missing"
    $probeTranscriptPrefix = "AME_WINDOWS_UIA_PHASE"
    $validatedPhases = [Collections.Generic.List[string]]::new()
    foreach ($phase in $expectedProbePhases) { $validatedPhases.Add($phase) }
    $probeResultTranscript = [Collections.Generic.List[string]]::new()
    foreach ($count in @(0, 1, 2)) {
        if ($count -gt 0) {
            $record = $baseline.Clone()
            $record.Phase = $expectedProbePhases[$count - 1]
            $record.ElementCount += $count
            Write-AmeWindowsUiaProbeRecord @record
            $arguments = $request.Clone()
            $arguments.Phase = $record.Phase
            $nextResult = Complete-AmeWindowsUiaProbe @arguments -Cleanup (New-AmeWindowsAccessibilityCleanup)
            $probeResultTranscript.Add((Format-AmeWindowsUiaProbeTranscript -Record $nextResult))
        }
        $transcript = & $captureTranscript
        & $assertTranscript -Transcript $transcript
        $resultLines = @($transcript -split "`r?`n" | Where-Object { $_.StartsWith($prefix) })
        Assert-AmeUiaProbeEvidence ($resultLines.Count -eq $count) "capture dropped or duplicated result records"
        Assert-AmeUiaProbeEvidence (($resultLines -join "`n") -ceq ($probeResultTranscript -join "`n")) "capture reordered or repeated different records"
        Assert-AmeUiaProbeEvidence ($probeResultTranscript.Count -eq $count) "capture changed its input collection"
    }
    $incomplete = $transcript.Replace("$probeTranscriptPrefix phase=settings-menu-closed result=ok", "")
    $caught = $null
    try { & $assertTranscript -Transcript $incomplete } catch { $caught = $_.Exception }
    Assert-AmeUiaProbeEvidence ($null -ne $caught) "success metrics substituted for a missing phase"

    Complete-AmeWindowsAccessibilityRun -OutputPath $completionPath `
        -CapturedOutput "controlled Flutter output" -ProbeTranscript $transcript
    $persisted = [IO.File]::ReadAllText($completionPath)
    $resultLines = @($persisted -split "`r?`n" | Where-Object { $_.StartsWith($prefix) })
    Assert-AmeUiaProbeEvidence ($resultLines.Count -eq 2) "persistent completion lost successful query metrics"
    Assert-AmeUiaProbeEvidence (($resultLines -join "`n") -ceq ($probeResultTranscript -join "`n")) "persistent completion changed query evidence"
} finally {
    foreach ($path in $ownedFiles) {
        if (Test-Path -LiteralPath $path -PathType Leaf) { Remove-Item -LiteralPath $path }
    }
    if (@(Get-ChildItem -LiteralPath $scratchRoot -Force).Count -eq 0) {
        Remove-Item -LiteralPath $scratchRoot
    }
}

Write-Host "Windows accessibility compiler-free probe evidence guardrails passed."
