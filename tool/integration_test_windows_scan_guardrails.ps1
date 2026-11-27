$ErrorActionPreference = "Stop"
. (Join-Path $PSScriptRoot "quality_common.ps1")

$nativeTypeBefore = "AmeWindowsAccessibilityProcessJob" -as [type]
. (Join-Path $PSScriptRoot "integration_windows_scan_run.ps1")
if (("AmeWindowsAccessibilityProcessJob" -as [type]) -ne $nativeTypeBefore) {
    throw "Importing Windows scan lifecycle helpers compiled native code"
}

function Assert-AmeScanGuardrail {
    param([bool]$Condition, [string]$Message)
    if (-not $Condition) { throw $Message }
}

$failure = [InvalidOperationException]::new("current-run-failure")
$failed = New-AmeWindowsScanOutcome
$failed.RunFailure = $failure
$failed.ExitCode = 1
$restored = [pscustomobject]@{ environment = $false; toolLock = $false }
Invoke-AmeWindowsScanCleanup $failed "close-owned-job" { throw "cleanup-close-failure" }
Invoke-AmeWindowsScanCleanup $failed "restore-environment" { $restored.environment = $true }
Invoke-AmeWindowsScanCleanup $failed "release-tool-lock" { $restored.toolLock = $true }
$observed = $null
try { Complete-AmeWindowsScanRun $failed } catch { $observed = $_.Exception }
Assert-AmeScanGuardrail ([object]::ReferenceEquals($observed, $failure)) "Cleanup replaced the original failure"
Assert-AmeScanGuardrail ($restored.environment -and $restored.toolLock) "A cleanup failure interrupted later restoration"
$completion = Get-AmeWindowsScanCompletion $failed
Assert-AmeScanGuardrail ($completion.status -ceq "failed") "Failed execution was reported as passed"
Assert-AmeScanGuardrail ($completion.runFailure -ceq "current-run-failure") "Completion lost the current failure"
Assert-AmeScanGuardrail ($completion.cleanupFailures.Count -eq 1) "Completion lost its separate cleanup failure"
Assert-AmeScanGuardrail ($completion.cleanupFailures[0].stage -ceq "close-owned-job") "Cleanup phase identity was lost"

$cleanupOnly = New-AmeWindowsScanOutcome
$cleanupOnly.ExitCode = 0
Invoke-AmeWindowsScanCleanup $cleanupOnly "close-owned-job" { throw "cleanup-only" }
$observed = $null
try { Complete-AmeWindowsScanRun $cleanupOnly } catch { $observed = $_.Exception }
Assert-AmeScanGuardrail ($null -ne $observed -and $observed.Message -ceq "cleanup-only") "Cleanup-only failure passed"
$missing = New-AmeWindowsScanOutcome
$observed = $null
try { Complete-AmeWindowsScanRun $missing } catch { $observed = $_.Exception }
Assert-AmeScanGuardrail ($null -ne $observed) "Missing process result passed"

$sharedClock = [pscustomobject]@{ Elapsed = [TimeSpan]::FromSeconds(1799) }
$sharedProcess = [pscustomobject]@{ HasExited = $true }
$sharedJob = [pscustomobject]@{ PrimaryExitCode = 0 }
$exitCode = Wait-AmeWindowsScanProcess $sharedProcess $sharedJob $sharedClock 1800
Assert-AmeScanGuardrail ($exitCode -eq 0) "The shared positional timeout path required scan progress"
$sharedJob.PrimaryExitCode = 7
$exitCode = Wait-AmeWindowsScanProcess $sharedProcess $sharedJob $sharedClock 1800
Assert-AmeScanGuardrail ($exitCode -eq 7) "The shared waiter lost a nonzero exit code"
$sharedClock.Elapsed = [TimeSpan]::FromSeconds(1800)
$observed = $null
try { Wait-AmeWindowsScanProcess $sharedProcess $sharedJob $sharedClock 1800 } catch { $observed = $_.Exception }
Assert-AmeScanGuardrail ($observed -is [TimeoutException]) "A completed shared process escaped its parent deadline"
$sharedProcess.HasExited = $false
$sharedProcess | Add-Member -MemberType ScriptMethod -Name WaitForExit -Value {
    throw "An expired shared process received another wait"
}
$observed = $null
try { Wait-AmeWindowsScanProcess $sharedProcess $sharedJob $sharedClock 1800 } catch { $observed = $_.Exception }
Assert-AmeScanGuardrail ($observed -is [TimeoutException]) "The shared waiter lost its active-process deadline"

$startedEvent = '[{"event":"test.startedProcess","params":{"vmServiceUri":"http://127.0.0.1:1234/"}}]' + "`n"
function New-AmeScanProgressFixture {
    param([string]$Text = "")
    New-AmeWindowsScanProgress ([IO.StringReader]::new($Text))
}

$clock = [pscustomobject]@{ Elapsed = [TimeSpan]::FromSeconds(60) }
$process = [pscustomobject]@{ HasExited = $false }
$job = [pscustomobject]@{ PrimaryExitCode = 0 }
$progress = New-AmeScanProgressFixture
$observed = $null
try { Wait-AmeWindowsScanTestProcess $process $job $clock $progress 60 900 } catch { $observed = $_.Exception }
Assert-AmeScanGuardrail (
    $observed -is [TimeoutException] -and $observed.Message.Contains("build/startup")
) "A build hang was not rejected by its own deadline"
$process.HasExited = $true
$progress = New-AmeScanProgressFixture $startedEvent
$observed = $null
try { Wait-AmeWindowsScanTestProcess $process $job $clock $progress 60 900 } catch { $observed = $_.Exception }
Assert-AmeScanGuardrail ($observed -is [TimeoutException]) "A process finishing after the deadline passed"
Assert-AmeScanGuardrail ($null -eq $progress.ExecutionStartedMilliseconds) "A late start event extended the build deadline"
$clock.Elapsed = [TimeSpan]::FromSeconds(59)
$progress = New-AmeScanProgressFixture $startedEvent
$exitCode = Wait-AmeWindowsScanTestProcess $process $job $clock $progress 60 900
Assert-AmeScanGuardrail ($exitCode -eq 0) "A timely owned process failed"
$progress = New-AmeScanProgressFixture
$observed = $null
try { Wait-AmeWindowsScanTestProcess $process $job $clock $progress 60 900 } catch { $observed = $_.Exception }
Assert-AmeScanGuardrail (
    $null -ne $observed -and $observed.Message.Contains("without its Flutter test.startedProcess event")
) "Zero exit without a started test process passed"
$job.PrimaryExitCode = 7
$exitCode = Wait-AmeWindowsScanTestProcess $process $job $clock $progress 60 900
Assert-AmeScanGuardrail ($exitCode -eq 7) "An early Flutter failure lost its exit code"
$job.PrimaryExitCode = 0
$advancing = [pscustomobject]@{
    HasExited = $false
    Clock = [pscustomobject]@{ Elapsed = [TimeSpan]::Zero }
    Waits = 0
}
$advancing | Add-Member -MemberType ScriptMethod -Name WaitForExit -Value {
    param([int]$Milliseconds)
    if ($Milliseconds -ne 100) { throw "Parent wait was not bounded" }
    $this.Waits += 1
    $this.Clock.Elapsed = [TimeSpan]::FromSeconds(60)
    return $false
}
$observed = $null
try {
    Wait-AmeWindowsScanTestProcess $advancing $job $advancing.Clock (New-AmeScanProgressFixture) 60 900
} catch { $observed = $_.Exception }
Assert-AmeScanGuardrail (
    $observed -is [TimeoutException] -and $advancing.Waits -eq 1
) "The parent did not enforce its deadline while the native process remained active"

$progress = New-AmeScanProgressFixture
Add-AmeWindowsScanOutput $progress $startedEvent ([TimeSpan]::FromSeconds(890))
$clock.Elapsed = [TimeSpan]::FromSeconds(1789)
$exitCode = Wait-AmeWindowsScanTestProcess $process $job $clock $progress 900 900
Assert-AmeScanGuardrail ($exitCode -eq 0) "Build time consumed the independent execution budget"
$clock.Elapsed = [TimeSpan]::FromSeconds(1790)
$observed = $null
try { Wait-AmeWindowsScanTestProcess $process $job $clock $progress 900 900 } catch { $observed = $_.Exception }
Assert-AmeScanGuardrail (
    $observed -is [TimeoutException] -and $observed.Message.Contains("900-second execution")
) "Execution exceeded its original 900-second bound"

$partial = New-AmeScanProgressFixture
$split = 20
Add-AmeWindowsScanOutput $partial $startedEvent.Substring(0, $split) ([TimeSpan]::FromSeconds(850))
Assert-AmeScanGuardrail ($null -eq $partial.ExecutionStartedMilliseconds) "An incomplete event started execution"
Add-AmeWindowsScanOutput $partial $startedEvent.Substring($split) ([TimeSpan]::FromSeconds(851))
Assert-AmeScanGuardrail ($partial.ExecutionStartedMilliseconds -eq 851000) "A fragmented event was lost"
$observed = $null
try { Add-AmeWindowsScanOutput $partial $startedEvent ([TimeSpan]::FromSeconds(852)) } catch { $observed = $_.Exception }
Assert-AmeScanGuardrail (
    $null -ne $observed -and $partial.ExecutionStartedMilliseconds -eq 851000
) "A duplicate event renewed the execution budget"

$noise = New-AmeScanProgressFixture
foreach ($line in @(
    '{"type":"testStart","test":{"name":"loading integration_test"}}'
    '{"type":"print","message":"test.startedProcess"}'
    '[{"event":"test.other"}]'
    '[not-json]'
)) {
    Add-AmeWindowsScanOutput $noise "$line`n" ([TimeSpan]::FromSeconds(1))
}
Assert-AmeScanGuardrail ($null -eq $noise.ExecutionStartedMilliseconds) "Loading or printed text replaced the device-start event"
$bounded = New-AmeScanProgressFixture (("x" * 70000) + "`n" + $startedEvent)
$drained = Read-AmeWindowsScanProgress $bounded ([TimeSpan]::FromSeconds(1))
Assert-AmeScanGuardrail (
    -not $drained -and $null -eq $bounded.ExecutionStartedMilliseconds -and
    $bounded.PendingLine.Length -le 16384
) "A progress poll exceeded its read or pending-line bound"
$drained = Read-AmeWindowsScanProgress $bounded ([TimeSpan]::FromSeconds(2))
Assert-AmeScanGuardrail (
    $drained -and $bounded.ExecutionStartedMilliseconds -eq 2000
) "Incremental reads lost the event after a discarded oversized line"

$retained = New-AmeWindowsScanOutcome
$retained.ExitCode = 0
$retained.StorageRoot = Join-Path (Get-AmeRepositoryRoot) "build\integration-storage-$($retained.RunId)"
$completion = Get-AmeWindowsScanCompletion $retained
Assert-AmeScanGuardrail ($completion.status -ceq "passed") "Retaining evidence was treated as a cleanup failure"
Assert-AmeScanGuardrail ($completion.storageDisposition -ceq "retained-evidence-no-owned-cleanup-proof") "Completion falsely claimed storage deletion"
$retained.RunFailure = $failure
$completion = Get-AmeWindowsScanCompletion $retained
Assert-AmeScanGuardrail ($completion.storageDisposition -ceq "retained-failure-evidence") "Failed-run evidence retention was lost"
Assert-AmeScanGuardrail ($retained.RunId -cne $failed.RunId) "Runs reused evidence identity"

$previousStorage = [Environment]::GetEnvironmentVariable("CEDARFLAKE_AME_TEST_STORAGE_ROOT", "Process")
try {
    [Environment]::SetEnvironmentVariable("CEDARFLAKE_AME_TEST_STORAGE_ROOT", "temporary-fixture-value", "Process")
    Restore-AmeWindowsScanEnvironment $null
    Assert-AmeScanGuardrail (
        $null -eq [Environment]::GetEnvironmentVariable("CEDARFLAKE_AME_TEST_STORAGE_ROOT", "Process")
    ) "Restoring an absent environment variable left an empty alias"
    Restore-AmeWindowsScanEnvironment "retained-original-value"
    Assert-AmeScanGuardrail (
        [Environment]::GetEnvironmentVariable("CEDARFLAKE_AME_TEST_STORAGE_ROOT", "Process") -ceq "retained-original-value"
    ) "Original environment value was not restored"
} finally {
    Restore-AmeWindowsScanEnvironment $previousStorage
}

& {
    $savedStorage = [Environment]::GetEnvironmentVariable("CEDARFLAKE_AME_TEST_STORAGE_ROOT", "Process")
    $setupFailure = [InvalidOperationException]::new("production-setup-failure")
    $lockSentinel = [object]::new()
    $calls = [pscustomobject]@{ setup = 0; releases = 0; releaseFails = $false }
    function Enter-AmeRepositoryToolLock { return $lockSentinel }
    function New-AmeWindowsScanStorage {
        param([string]$RepositoryRoot, [string]$RunId)
        $calls.setup += 1
        [Environment]::SetEnvironmentVariable("CEDARFLAKE_AME_TEST_STORAGE_ROOT", "mutated-during-setup", "Process")
        throw $setupFailure
    }
    function Exit-AmeRepositoryToolLock {
        param([object]$Mutex)
        Assert-AmeScanGuardrail ([object]::ReferenceEquals($Mutex, $lockSentinel)) "Production released an unowned lock"
        $calls.releases += 1
        if ($calls.releaseFails) { throw "production-lock-release-failure" }
    }
    try {
        foreach ($releaseFails in @($false, $true)) {
            $calls.setup = 0
            $calls.releases = 0
            $calls.releaseFails = $releaseFails
            $originalValue = if ($releaseFails) { $null } else { "original-production-value" }
            Restore-AmeWindowsScanEnvironment $originalValue
            $production = Invoke-AmeWindowsScanRun `
                -RepositoryRoot (Get-AmeRepositoryRoot) -FlutterPath "unused-flutter.bat" -TimeoutSeconds 60
            $actualValue = [Environment]::GetEnvironmentVariable("CEDARFLAKE_AME_TEST_STORAGE_ROOT", "Process")
            $environmentMatches = if ($null -eq $originalValue) { $null -eq $actualValue } else { $actualValue -ceq $originalValue }
            Assert-AmeScanGuardrail ($calls.setup -eq 1 -and $calls.releases -eq 1) "Production finally did not release its acquired lock"
            Assert-AmeScanGuardrail $environmentMatches "Production finally did not restore the original environment"
            Assert-AmeScanGuardrail (
                $null -eq $production.StorageRoot -and
                [object]::ReferenceEquals($production.RunFailure, $setupFailure)
            ) "Production lost its setup failure or published uncreated storage"
            $expectedCleanupCount = if ($releaseFails) { 1 } else { 0 }
            Assert-AmeScanGuardrail ($production.CleanupFailures.Count -eq $expectedCleanupCount) "Production cleanup failure was not retained independently"
            if ($releaseFails) {
                Assert-AmeScanGuardrail (
                    $production.CleanupFailures[0].stage -ceq "release-tool-lock"
                ) "Production lock cleanup lost its stage identity"
            }
            $observed = $null
            try { Complete-AmeWindowsScanRun $production } catch { $observed = $_.Exception }
            Assert-AmeScanGuardrail ([object]::ReferenceEquals($observed, $setupFailure)) "Production completion masked the original setup failure"
        }
    } finally {
        Restore-AmeWindowsScanEnvironment $savedStorage
    }
    Assert-AmeScanGuardrail (
        ("AmeWindowsAccessibilityProcessJob" -as [type]) -eq $nativeTypeBefore
    ) "Production setup-failure guardrails initialized native code"
}

Assert-AmeWindowsScanDirectoryChain (Get-AmeRepositoryRoot)
& {
    function Get-Item {
        [pscustomobject]@{ PSIsContainer = $true; Attributes = [IO.FileAttributes]::ReparsePoint }
    }
    $observed = $null
    try { Assert-AmeWindowsScanDirectoryChain (Get-AmeRepositoryRoot) } catch { $observed = $_.Exception }
    Assert-AmeScanGuardrail (
        $null -ne $observed -and $observed.Message.Contains("ordinary directory components")
    ) "Evidence creation accepted a reparse component"
    $failedPersistence = New-AmeWindowsScanOutcome
    $failedPersistence.RunFailure = $failure
    $failedPersistence.StorageRoot = Join-Path (Get-AmeRepositoryRoot) "build\absent-evidence-$($failedPersistence.RunId)"
    $observed = $null
    try { Complete-AmeWindowsScanRun $failedPersistence } catch { $observed = $_.Exception }
    Assert-AmeScanGuardrail (
        [object]::ReferenceEquals($observed, $failure) -and
        $failedPersistence.CleanupFailures.Count -eq 1 -and
        $failedPersistence.CleanupFailures[0].stage -ceq "persist-completion"
    ) "Failed evidence persistence masked the original run failure"
}

$paths = @(
    (Join-Path $PSScriptRoot "integration_test_windows.ps1")
    (Join-Path $PSScriptRoot "integration_windows_scan_run.ps1")
    (Join-Path $PSScriptRoot "integration_windows_scan_progress.ps1")
)
Invoke-AmePowerShellSyntaxCheck $paths
$commands = @(
    foreach ($path in $paths) {
        $ast = [Management.Automation.Language.Parser]::ParseFile($path, [ref]$null, [ref]$null)
        $ast.FindAll({ param($node) $node -is [Management.Automation.Language.CommandAst] }, $true)
    }
)
foreach ($command in $commands) {
    Assert-AmeScanGuardrail (
        $command.GetCommandName() -notin @("Stop-Process", "Remove-Item", "Push-Location", "Pop-Location", "Add-Type", "Invoke-AmeChecked")
    ) "The scan facade bypassed process ownership, evidence retention, or unchanged working-directory policy"
    if ($command.GetCommandName() -ceq "New-Item") {
        Assert-AmeScanGuardrail (
            @($command.CommandElements | Where-Object {
                $_ -is [Management.Automation.Language.CommandParameterAst] -and $_.ParameterName -ieq "Force"
            }).Count -eq 0
        ) "Evidence creation adopted existing storage"
    }
}
Write-Output "windows_scan_lifecycle_guardrails_passed"
