$ErrorActionPreference = "Stop"
. (Join-Path $PSScriptRoot "integration_windows_accessibility_process.ps1")
. (Join-Path $PSScriptRoot "integration_windows_accessibility_assemblies.ps1")

function Assert-AmeAssemblyProgressRefusal {
    param([scriptblock]$Action, [string]$Boundary)

    $rejected = $false
    try { & $Action | Out-Null } catch { $rejected = $true }
    if (-not $rejected) { throw "Windows UIA assembly progress accepted $Boundary" }
}

$repositoryRoot = Split-Path -Parent $PSScriptRoot
$buildRoot = [System.IO.Path]::GetFullPath((Join-Path $repositoryRoot "build"))
$scratchRoot = Join-Path $buildRoot ("uia-assembly-progress-" + [guid]::NewGuid().ToString("N"))
$request = @{
    ResultPath = Join-Path $scratchRoot "markers.json"
    Token = [guid]::NewGuid().ToString("N")
    Phase = "application-ready"
    TargetProcessId = $PID
}
$owner = $null
$child = $null
$collision = $null
$passed = $false
try {
    $owner = New-AmeWindowsUiaAssemblyProgress @request -Mode "Create"
    $child = New-AmeWindowsUiaAssemblyProgress @request -Mode "Open"
    $initial = Get-AmeWindowsUiaAssemblyProgress $owner
    foreach ($assembly in @("UIAutomationTypes", "UIAutomationClient")) {
        foreach ($stage in @("entered", "loaded", "verified")) {
            if ($initial.$assembly[$stage] -ne $false) {
                throw "New assembly progress inherited an earlier signal"
            }
        }
    }
    Import-AmeWindowsUiaAssembly -Name "UIAutomationTypes" -Progress $child
    Import-AmeWindowsUiaAssembly -Name "UIAutomationClient" -Progress $child
    Close-AmeWindowsUiaAssemblyProgress $child
    $child = $null
    $loaded = Get-AmeWindowsUiaAssemblyProgress $owner
    foreach ($assembly in @("UIAutomationTypes", "UIAutomationClient")) {
        foreach ($stage in @("entered", "loaded", "verified")) {
            if ($loaded.$assembly[$stage] -ne $true) {
                throw "Runtime loading did not publish its exact assembly milestone"
            }
        }
    }
    Assert-AmeAssemblyProgressRefusal {
        New-AmeWindowsUiaAssemblyProgress @request -Mode "Create"
    } "a second owner"
    $otherRequest = @{} + $request
    $otherRequest.Token = "different-owner"
    Assert-AmeAssemblyProgressRefusal {
        New-AmeWindowsUiaAssemblyProgress @otherRequest -Mode "Open"
    } "a different request token"
    if (-not (Get-AmeWindowsUiaAssemblyProgress $owner).UIAutomationClient.verified) {
        throw "A refused marker connection changed the original owner"
    }

    $unconfirmed = New-AmeWindowsAccessibilityCleanup
    $unconfirmed.ProcessExited = $false
    $unconfirmed.JobClosed = $true
    if ($null -ne (Complete-AmeWindowsUiaAssemblyProgress $owner $unconfirmed)) {
        throw "Assembly markers claimed a stable snapshot before owned exit"
    }
    if ($owner.Count -ne 0 -or $unconfirmed.Failures.Count -ne 0) {
        throw "Unconfirmed process exit skipped assembly marker retirement"
    }
    Assert-AmeAssemblyProgressRefusal {
        New-AmeWindowsUiaAssemblyProgress @request -Mode "Open"
    } "retired markers"
    $owner = $null

    $owner = New-AmeWindowsUiaAssemblyProgress @request -Mode "Create"
    $unconfirmedJob = New-AmeWindowsAccessibilityCleanup
    $unconfirmedJob.ProcessExited = $true
    $unconfirmedJob.JobClosed = $false
    if ($null -ne (Complete-AmeWindowsUiaAssemblyProgress $owner $unconfirmedJob)) {
        throw "Assembly markers claimed a stable snapshot before the owned process group closed"
    }
    if ($owner.Count -ne 0 -or $unconfirmedJob.Failures.Count -ne 0) {
        throw "Unconfirmed job closure skipped assembly marker retirement"
    }
    $owner = $null

    $names = Get-AmeWindowsUiaAssemblyProgressNames @request
    $collision = [System.Threading.EventWaitHandle]::new(
        $true, [System.Threading.EventResetMode]::ManualReset, $names["UIAutomationTypes-verified"]
    )
    Assert-AmeAssemblyProgressRefusal {
        New-AmeWindowsUiaAssemblyProgress @request -Mode "Create"
    } "partial marker ownership"
    if (-not $collision.WaitOne(0)) { throw "Partial creation reset an unrelated marker" }
    foreach ($key in @("UIAutomationTypes-entered", "UIAutomationTypes-loaded")) {
        Assert-AmeAssemblyProgressRefusal {
            $unexpected = [System.Threading.EventWaitHandle]::OpenExisting($names[$key])
            $unexpected.Dispose()
        } "a handle retained after partial acquisition"
    }
    $collision.Dispose()
    $collision = $null

    $owner = New-AmeWindowsUiaAssemblyProgress @request -Mode "Create"
    $owner["UIAutomationClient-loaded"].Dispose()
    $cleanup = New-AmeWindowsAccessibilityCleanup
    $cleanup.ProcessExited = $true
    $cleanup.JobClosed = $true
    if ($null -ne (Complete-AmeWindowsUiaAssemblyProgress $owner $cleanup)) {
        throw "A failed marker read produced a complete snapshot"
    }
    if ($cleanup.Failures.Count -ne 1 -or $cleanup.Failures[0].stage -cne "read-assembly-markers") {
        throw "Marker read failure was not preserved independently of cleanup"
    }
    if ($owner.Count -ne 0) { throw "A failed marker read skipped remaining handle retirement" }
    $owner = $null

    New-Item -ItemType Directory -Path $scratchRoot | Out-Null
    $fixture = Join-Path $scratchRoot "probe.ps1"
    $fixtureSource = @'
param([int]$TargetProcessId, [string]$Phase, [string]$ResultPath, [string]$Token)
$ErrorActionPreference = "Stop"
. '__AME_EVIDENCE__'
. '__AME_ASSEMBLIES__'
$progress = New-AmeWindowsUiaAssemblyProgress -ResultPath $ResultPath -Token $Token `
    -Phase $Phase -TargetProcessId $TargetProcessId -Mode "Open"
try {
    Import-AmeWindowsUiaAssembly -Name "UIAutomationTypes" -Progress $progress
    $record = @{
        ResultPath = $ResultPath; Token = $Token; Phase = $Phase
        TargetProcessId = $TargetProcessId; ProbeProcessId = $PID
    }
    Write-AmeWindowsUiaProbeRecord @record -Status "progress" -Stage "loading-uia-client"
    if ($Phase -ceq "before-loader") {
        Start-Sleep -Seconds 60
    } elseif ($Phase -ceq "inside-loader") {
        Set-AmeWindowsUiaAssemblyProgress -Progress $progress -Name "UIAutomationClient" -Stage "entered"
        Start-Sleep -Seconds 60
    } else {
        Import-AmeWindowsUiaAssembly -Name "UIAutomationClient" -Progress $progress
        Write-AmeWindowsUiaProbeRecord @record -Status "complete" -Stage "complete"
    }
} finally {
    Close-AmeWindowsUiaAssemblyProgress $progress
}
'@
    $fixtureSource = $fixtureSource.Replace(
        "__AME_EVIDENCE__", (Join-Path $PSScriptRoot "integration_windows_accessibility_evidence.ps1").Replace("'", "''")
    ).Replace(
        "__AME_ASSEMBLIES__", (Join-Path $PSScriptRoot "integration_windows_accessibility_assemblies.ps1").Replace("'", "''")
    )
    [System.IO.File]::WriteAllText($fixture, $fixtureSource, [System.Text.UTF8Encoding]::new($false))
    $success = Invoke-AmeWindowsAccessibilityProbe -ProbeScriptPath $fixture `
        -TargetProcessId $PID -Phase "successful-loader" -Token $request.Token `
        -ResultPath (Join-Path $scratchRoot "success.json") -Timeout ([TimeSpan]::FromSeconds(8))
    if (-not $success.assemblyProgress.UIAutomationClient.verified) {
        throw "Successful probe transcript omitted its post-exit assembly markers"
    }

    foreach ($phase in @("before-loader", "inside-loader")) {
        $failure = $null
        $clock = [System.Diagnostics.Stopwatch]::StartNew()
        try {
            Invoke-AmeWindowsAccessibilityProbe -ProbeScriptPath $fixture `
                -TargetProcessId $PID -Phase $phase -Token $request.Token `
                -ResultPath (Join-Path $scratchRoot "$phase.json") -Timeout ([TimeSpan]::FromSeconds(8))
        } catch {
            $failure = $_.Exception
        } finally {
            $clock.Stop()
        }
        if ($null -eq $failure -or $failure.Message -cne "Windows UIA probe '$phase' exceeded its parent deadline") {
            throw "Assembly marker attribution replaced the original deadline failure"
        }
        $record = $failure.Data["ameWindowsUiaProbeProgress"]
        $markers = $failure.Data["ameWindowsUiaAssemblyProgress"]
        if ($clock.Elapsed.TotalSeconds -gt 14 -or $record.stage -cne "loading-uia-client") {
            throw "Assembly marker fixture escaped its deadline or changed its persisted stage"
        }
        if ($null -eq $markers -or -not $markers.UIAutomationTypes.verified) {
            throw "Timed-out probe lost its completed assembly contract"
        }
        if (
            $markers.UIAutomationClient.entered -ne ($phase -ceq "inside-loader") -or
            $markers.UIAutomationClient.loaded -or $markers.UIAutomationClient.verified
        ) {
            throw "Identical persisted stages failed to distinguish the blocked operation"
        }
        if ($null -ne $failure.Data["ameWindowsUiaProbeCleanupFailure"]) {
            throw "Assembly attribution left an owned process or handle cleanup failure"
        }
        $completion = Get-AmeWindowsAccessibilityCompletionOutput -CapturedOutput "" `
            -ProbeTranscript "" -RunFailure $failure -Cleanup (New-AmeWindowsAccessibilityCleanup) `
            -EvidencePersisted $true
        $completionLine = @($completion -split "`r?`n" | Where-Object { $_.StartsWith("AME_WINDOWS_ACCESSIBILITY_COMPLETION ") })
        $decoded = $completionLine[0].Substring("AME_WINDOWS_ACCESSIBILITY_COMPLETION ".Length) | ConvertFrom-Json
        if ($decoded.probeAssemblyProgress.UIAutomationClient.entered -ne ($phase -ceq "inside-loader")) {
            throw "Completion output discarded the independent assembly markers"
        }
    }
    $passed = $true
    Write-Host "Windows UIA assembly progress ownership, loader milestones, timeout attribution and cleanup passed"
} finally {
    Close-AmeWindowsUiaAssemblyProgress $child
    Close-AmeWindowsUiaAssemblyProgress $owner
    if ($null -ne $collision) { $collision.Dispose() }
    $resolvedScratch = [System.IO.Path]::GetFullPath($scratchRoot)
    if (-not $resolvedScratch.StartsWith("$buildRoot\", [System.StringComparison]::OrdinalIgnoreCase)) {
        throw "Assembly progress fixture storage escaped build"
    }
    if ($passed -and (Test-Path -LiteralPath $resolvedScratch)) {
        Remove-Item -LiteralPath $resolvedScratch -Recurse -Force
    }
}
