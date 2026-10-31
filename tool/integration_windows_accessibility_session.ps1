. (Join-Path $PSScriptRoot "integration_windows_accessibility_process.ps1")
. (Join-Path $PSScriptRoot "integration_windows_accessibility_session_protocol.ps1")

function Start-AmeWindowsUiaSession {
    param(
        [string]$ScratchRoot,
        [string]$Token,
        [string]$WorkerScriptPath = (Join-Path $PSScriptRoot "integration_windows_accessibility_session_worker.ps1")
    )

    foreach ($argument in @($ScratchRoot, $Token, $WorkerScriptPath)) {
        if ($argument.Contains('"') -or $argument.IndexOf([char]0) -ge 0) {
            throw "Windows UIA session argument cannot be quoted safely"
        }
    }
    Initialize-AmeWindowsAccessibilityProcessJob
    $session = [pscustomobject]@{
        ScratchRoot = [IO.Path]::GetFullPath($ScratchRoot)
        Token = $Token
        Sequence = 0
        StartupClock = [Diagnostics.Stopwatch]::StartNew()
        Job = $null
        Process = $null
        Cleanup = (New-AmeWindowsAccessibilityCleanup)
    }
    try {
        $session.Job = [AmeWindowsAccessibilityProcessJob]::new()
        $arguments = (
            '-NoLogo -NoProfile -NonInteractive -Mta -ExecutionPolicy Bypass ' +
            '-File "{0}" -ScratchRoot "{1}" -Token "{2}"'
        ) -f $WorkerScriptPath, $session.ScratchRoot, $Token
        $session.Process = $session.Job.Start((Get-Process -Id $PID).Path, $arguments, $PSScriptRoot)
        return $session
    } catch {
        $failure = $_.Exception
        Stop-AmeWindowsUiaSession -Session $session
        throw $failure
    }
}

function Stop-AmeWindowsUiaSession {
    param([AllowNull()] [object]$Session)

    if ($null -eq $Session) { return }
    Close-AmeWindowsAccessibilityProcess -Cleanup $Session.Cleanup -Job $Session.Job -Process $Session.Process
    $Session.Job = $null
    $Session.Process = $null
}

function Complete-AmeWindowsUiaSession {
    param([AllowNull()] [object]$Session)

    if ($null -eq $Session) { return }
    $exitedEarly = $null -ne $Session.Process -and $Session.Process.HasExited
    Stop-AmeWindowsUiaSession -Session $Session
    if ($exitedEarly) { throw "Windows UIA session exited before completion" }
    $failure = Get-AmeWindowsAccessibilityCleanupFailure $Session.Cleanup
    if ($null -ne $failure) { throw $failure }
}

function Invoke-AmeWindowsUiaSessionPhase {
    param(
        [object]$Session,
        [int]$TargetProcessId,
        [string]$Phase,
        [string]$ResultPath,
        [TimeSpan]$Timeout
    )

    if ($null -eq $Session.Process -or $Session.Process.HasExited) {
        throw "Windows UIA session exited before its next phase"
    }
    if ($Timeout -le [TimeSpan]::Zero) { throw "Windows UIA phase has no remaining parent deadline" }
    $resolvedResult = [IO.Path]::GetFullPath($ResultPath)
    if (-not [IO.Path]::GetDirectoryName($resolvedResult).Equals($Session.ScratchRoot, [StringComparison]::OrdinalIgnoreCase) -or
        [IO.File]::Exists($resolvedResult) -or [IO.File]::Exists([IO.Path]::ChangeExtension($resolvedResult, "tmp"))) {
        throw "Windows UIA phase result must use fresh session storage"
    }
    $request = [pscustomobject]@{
        format = "ame-windows-uia-session-v1"
        token = $Session.Token
        sequence = $Session.Sequence
        targetProcessId = $TargetProcessId
        phase = $Phase
        resultFile = [IO.Path]::GetFileName($resolvedResult)
    }
    Assert-AmeWindowsUiaSessionRequest -Request $request -Token $Session.Token -Sequence $Session.Sequence
    $clock = if ($Session.Sequence -eq 0) { $Session.StartupClock } else { [Diagnostics.Stopwatch]::StartNew() }
    $probeProcessId = $Session.Process.Id
    $progress = $null
    $completion = $null
    $snapshot = $null
    $failure = $null
    $result = $null
    try {
        $progress = New-AmeWindowsUiaAssemblyProgress -ResultPath $resolvedResult -Token $Session.Token `
            -Phase $Phase -TargetProcessId $TargetProcessId -Mode "Create"
        $name = Get-AmeWindowsUiaSessionCompletionName -ResultPath $resolvedResult -Token $Session.Token `
            -Phase $Phase -TargetProcessId $TargetProcessId
        $created = $false
        $completion = [Threading.EventWaitHandle]::new(
            $false, [Threading.EventResetMode]::ManualReset, $name, [ref]$created
        )
        if (-not $created) { throw "Windows UIA phase completion already has an owner" }
        Write-AmeWindowsUiaSessionRequest -ScratchRoot $Session.ScratchRoot -Request $request
        while ($true) {
            if ($Session.Process.HasExited) { throw "Windows UIA session exited during '$Phase'" }
            $remaining = $Timeout - $clock.Elapsed
            if ($remaining -le [TimeSpan]::Zero) { throw "Windows UIA probe '$Phase' exceeded its parent deadline" }
            $wait = [int][Math]::Min(50, [Math]::Max(1, [Math]::Ceiling($remaining.TotalMilliseconds)))
            if ($completion.WaitOne($wait)) { break }
        }
        if ($clock.Elapsed -gt $Timeout) { throw "Windows UIA probe '$Phase' exceeded its parent deadline" }
        if ($Session.Process.HasExited) { throw "Windows UIA session exited during '$Phase'" }
        $result = Read-AmeWindowsUiaProbeRecord -ResultPath $resolvedResult -Token $Session.Token `
            -Phase $Phase -TargetProcessId $TargetProcessId -ProbeProcessId $probeProcessId
        if ($result.status -cne "complete") { throw "Windows UIA probe '$Phase' produced no complete result" }
        if ($null -ne $result.failure) { throw "Windows UIA probe '$Phase' failed: $($result.failure)" }
        $snapshot = Get-AmeWindowsUiaAssemblyProgress $progress
    } catch {
        $failure = $_.Exception
        $failure.Data["ameWindowsUiaProbeParentElapsedMilliseconds"] = $clock.ElapsedMilliseconds
        Stop-AmeWindowsUiaSession -Session $Session
        $snapshot = Complete-AmeWindowsUiaAssemblyProgress -Progress $progress -Cleanup $Session.Cleanup
        $progress = $null
    } finally {
        $clock.Stop()
        Invoke-AmeWindowsAccessibilityCleanupStep $Session.Cleanup "close-phase-assembly-markers" {
            Close-AmeWindowsUiaAssemblyProgress $progress
        }
        Invoke-AmeWindowsAccessibilityCleanupStep $Session.Cleanup "close-phase-completion" {
            if ($null -ne $completion) { $completion.Dispose() }
        }
    }
    if ($null -eq $failure) { $failure = Get-AmeWindowsAccessibilityCleanupFailure $Session.Cleanup }
    if ($null -ne $failure) {
        Stop-AmeWindowsUiaSession -Session $Session
        Complete-AmeWindowsUiaProbe -ResultPath $resolvedResult -Token $Session.Token -Phase $Phase `
            -TargetProcessId $TargetProcessId -ProbeProcessId $probeProcessId -ProbeFailure $failure `
            -Cleanup $Session.Cleanup -AssemblyProgress $snapshot
    }
    $Session.Sequence += 1
    $result | Add-Member -NotePropertyName "assemblyProgress" -NotePropertyValue $snapshot
    return $result
}
