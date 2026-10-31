$ErrorActionPreference = "Stop"
. (Join-Path $PSScriptRoot "integration_windows_accessibility_session.ps1")

$buildRoot = [IO.Path]::GetFullPath((Join-Path (Split-Path -Parent $PSScriptRoot) "build"))
$scratchRoot = [IO.Path]::GetFullPath((Join-Path $buildRoot ("uia-session-" + [guid]::NewGuid().ToString("N"))))
if (-not $scratchRoot.StartsWith($buildRoot + "\", [StringComparison]::OrdinalIgnoreCase) -or
    ((Get-Item -LiteralPath $buildRoot).Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0) {
    throw "Windows UIA session fixture escaped owned build storage"
}
$null = New-Item -ItemType Directory -Path $scratchRoot
$canRetireStorage = $false
$session = $null
try {
    $realRoot = Join-Path $scratchRoot "real"
    $null = New-Item -ItemType Directory -Path $realRoot
    $session = Start-AmeWindowsUiaSession -ScratchRoot $realRoot -Token ([guid]::NewGuid().ToString("N"))
    $ownedProcessId = $session.Process.Id
    $timings = @()
    foreach ($sequence in 0..1) {
        $result = Invoke-AmeWindowsUiaSessionPhase -Session $session -TargetProcessId $PID `
            -Phase "runtime-preparation" -ResultPath (Join-Path $realRoot "prepared-$sequence.json") `
            -Timeout ([TimeSpan]::FromSeconds(8))
        if ($result.probeProcessId -ne $ownedProcessId -or $session.Process.HasExited -or
            $result.attempt -ne 0 -or $result.windowCount -ne 0 -or $result.elementCount -ne 0) {
            throw "UIA preparation did not retain its owned process independently of native assertions"
        }
        foreach ($assembly in @("UIAutomationTypes", "UIAutomationClient")) {
            foreach ($stage in @("entered", "loaded", "verified")) {
                if ($result.assemblyProgress.$assembly.$stage -ne $true) {
                    throw "A retained UIA session lost a verified assembly milestone"
                }
            }
        }
        $timings += $result.elapsedMilliseconds
    }
    Complete-AmeWindowsUiaSession $session
    if ($session.Cleanup.ProcessExited -ne $true -or $session.Cleanup.JobClosed -ne $true) {
        throw "A completed UIA session retained its owned process"
    }
    $session = $null

    $fixtureSource = @'
param([string]$ScratchRoot, [string]$Token)
$ErrorActionPreference = "Stop"
. '__PROTOCOL__'
. '__EVIDENCE__'
$sequence = 0
while ($true) {
    $path = Join-Path $ScratchRoot ("uia-command-{0:D2}.json" -f $sequence)
    if (-not [IO.File]::Exists($path)) { Start-Sleep -Milliseconds 10; continue }
    $request = Read-AmeWindowsUiaSessionRequest -Path $path -Token $Token -Sequence $sequence
    $resultPath = Join-Path $ScratchRoot $request.resultFile
    $name = Get-AmeWindowsUiaSessionCompletionName -ResultPath $resultPath -Token $Token `
        -Phase $request.phase -TargetProcessId $request.targetProcessId
    $completion = [Threading.EventWaitHandle]::OpenExisting($name)
    try {
        $record = @{
            ResultPath = $resultPath; Token = $Token; Phase = $request.phase
            TargetProcessId = $request.targetProcessId; ProbeProcessId = $PID
            Status = "complete"; Stage = "complete"
        }
        switch ($request.resultFile) {
            "wrong-owner.json" { $record.Token = "unowned" }
            "failed.json" { $record.Failure = "controlled native assertion failure" }
            "partial.json" { $record.Status = "progress"; $record.Stage = "finding-elements" }
        }
        Write-AmeWindowsUiaProbeRecord @record
        if ($request.resultFile -ceq "blocked.json") { Start-Sleep -Seconds 30 }
        $null = $completion.Set()
    } finally { $completion.Dispose() }
    $sequence += 1
}
'@
    $fixtureSource = $fixtureSource.Replace("__PROTOCOL__", (Join-Path $PSScriptRoot "integration_windows_accessibility_session_protocol.ps1").Replace("'", "''"))
    $fixtureSource = $fixtureSource.Replace("__EVIDENCE__", (Join-Path $PSScriptRoot "integration_windows_accessibility_evidence.ps1").Replace("'", "''"))
    $fixturePath = Join-Path $scratchRoot "worker.ps1"
    [IO.File]::WriteAllText($fixturePath, $fixtureSource, [Text.UTF8Encoding]::new($false))
    foreach ($case in @(
        @{ Name = "wrong-owner"; Message = "does not match its owned request" },
        @{ Name = "failed"; Message = "controlled native assertion failure" },
        @{ Name = "partial"; Message = "produced no complete result" },
        @{ Name = "blocked"; Message = "exceeded its parent deadline" }
    )) {
        $caseRoot = Join-Path $scratchRoot $case.Name
        $null = New-Item -ItemType Directory -Path $caseRoot
        $session = Start-AmeWindowsUiaSession -ScratchRoot $caseRoot `
            -Token ([guid]::NewGuid().ToString("N")) -WorkerScriptPath $fixturePath
        $failure = $null
        $clock = [Diagnostics.Stopwatch]::StartNew()
        try {
            Invoke-AmeWindowsUiaSessionPhase -Session $session -TargetProcessId $PID `
                -Phase "application-ready" -ResultPath (Join-Path $caseRoot "$($case.Name).json") `
                -Timeout ([TimeSpan]::FromSeconds(3)) | Out-Null
        } catch { $failure = $_.Exception }
        if ($null -eq $failure -or -not $failure.Message.Contains($case.Message) -or
            $session.Cleanup.ProcessExited -ne $true -or $session.Cleanup.JobClosed -ne $true -or
            $clock.Elapsed.TotalSeconds -gt 9) {
            throw "UIA session did not reject and retire '$($case.Name)': $failure"
        }
        if ($case.Name -ceq "blocked" -and $failure.Data["ameWindowsUiaProbeProgress"].status -cne "complete") {
            throw "The timeout must reject a complete file whose invocation has not returned"
        }
        $session = $null
    }

    $validRequest = [pscustomobject]@{
        format = "ame-windows-uia-session-v1"; token = "owner"; sequence = 1
        phase = "application-ready"; targetProcessId = $PID; resultFile = "valid.json"
    }
    foreach ($invalid in @(
        @{ Field = "token"; Value = "other" }, @{ Field = "sequence"; Value = 0 },
        @{ Field = "phase"; Value = "unknown" }, @{ Field = "resultFile"; Value = "../escape.json" },
        @{ Field = "targetProcessId"; Value = 0 }
    )) {
        $request = $validRequest | ConvertTo-Json | ConvertFrom-Json
        $request.($invalid.Field) = $invalid.Value
        $rejected = $false
        try { Assert-AmeWindowsUiaSessionRequest -Request $request -Token "owner" -Sequence 1 } catch { $rejected = $true }
        if (-not $rejected) { throw "UIA session accepted an invalid $($invalid.Field)" }
    }
    $canRetireStorage = $true
    Write-Host "Windows UIA session passed: retained loader $($timings -join '/') ms, invalid results and blocked invocation retired"
} finally {
    if ($null -ne $session) { Stop-AmeWindowsUiaSession $session }
    if ($canRetireStorage) {
        $entries = @((Get-Item -LiteralPath $scratchRoot)) + @(Get-ChildItem -LiteralPath $scratchRoot -Recurse -Force)
        if (@($entries | Where-Object { ($_.Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0 }).Count -ne 0) {
            throw "UIA session fixture contains a reparse point"
        }
        Remove-Item -LiteralPath $scratchRoot -Recurse -Force
    }
}
