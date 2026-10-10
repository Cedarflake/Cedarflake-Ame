Set-StrictMode -Version Latest
. (Join-Path $PSScriptRoot "integration_windows_accessibility_process.ps1")

function Assert-AmeWindowsScanDirectoryChain {
    param([Parameter(Mandatory = $true)] [string]$Path)

    $current = [IO.Path]::GetFullPath($Path)
    while ($current) {
        $entry = Get-Item -LiteralPath $current -Force
        if (-not $entry.PSIsContainer -or ($entry.Attributes -band [IO.FileAttributes]::ReparsePoint)) {
            throw "Windows scan evidence requires ordinary directory components: $current"
        }
        $parent = Split-Path -Parent $current
        if ($parent -eq $current) { break }
        $current = $parent
    }
}

function New-AmeWindowsScanStorage {
    param([string]$RepositoryRoot, [string]$RunId)

    if ($RunId -cnotmatch '^[a-f0-9]{32}$') { throw "Invalid Windows scan run identity" }
    Assert-AmeWindowsScanDirectoryChain $RepositoryRoot
    $build = Join-Path ([IO.Path]::GetFullPath($RepositoryRoot)) "build"
    if (-not (Test-Path -LiteralPath $build)) {
        $null = New-Item -ItemType Directory -Path $build
    }
    Assert-AmeWindowsScanDirectoryChain $build
    $storage = Join-Path $build "integration-storage-$RunId"
    if (Test-Path -LiteralPath $storage) { throw "Windows scan storage must be fresh" }
    $null = New-Item -ItemType Directory -Path $storage
    Assert-AmeWindowsScanDirectoryChain $storage
    return $storage
}

function New-AmeWindowsScanOutcome {
    [pscustomobject]@{
        RunId = [Guid]::NewGuid().ToString("N")
        StartedUtc = [DateTime]::UtcNow.ToString("o")
        StorageRoot = $null
        ExitCode = $null
        ElapsedMilliseconds = 0L
        RunFailure = $null
        CleanupFailures = [Collections.Generic.List[object]]::new()
    }
}

function Invoke-AmeWindowsScanCleanup {
    param([object]$Outcome, [string]$Stage, [scriptblock]$Action)

    try { & $Action } catch {
        $Outcome.CleanupFailures.Add([pscustomobject]@{
            stage = $Stage
            exception = $_.Exception
        })
    }
}

function Assert-AmeWindowsScanDeadline {
    param([TimeSpan]$Elapsed, [int]$TimeoutSeconds)

    if ($Elapsed.TotalSeconds -ge $TimeoutSeconds) {
        throw [TimeoutException]::new(
            "Windows scan integration exceeded its $TimeoutSeconds-second parent deadline"
        )
    }
}

function Wait-AmeWindowsScanProcess {
    param([object]$Process, [object]$Job, [object]$Clock, [int]$TimeoutSeconds)

    while (-not $Process.HasExited) {
        Assert-AmeWindowsScanDeadline $Clock.Elapsed $TimeoutSeconds
        $null = $Process.WaitForExit(100)
    }
    Assert-AmeWindowsScanDeadline $Clock.Elapsed $TimeoutSeconds
    return [int]$Job.PrimaryExitCode
}

function Restore-AmeWindowsScanEnvironment {
    param([AllowNull()] [object]$PreviousValue)

    $value = if ($null -eq $PreviousValue) {
        [System.Management.Automation.Language.NullString]::Value
    } else { $PreviousValue }
    [Environment]::SetEnvironmentVariable("CEDARFLAKE_AME_TEST_STORAGE_ROOT", $value, "Process")
}

function Invoke-AmeWindowsScanRun {
    param(
        [Parameter(Mandatory = $true)] [string]$RepositoryRoot,
        [Parameter(Mandatory = $true)] [string]$FlutterPath,
        [ValidateRange(60, 1800)] [int]$TimeoutSeconds = 900
    )

    $outcome = New-AmeWindowsScanOutcome
    $lock = $null
    $job = $null
    $process = $null
    $previousStorage = [Environment]::GetEnvironmentVariable("CEDARFLAKE_AME_TEST_STORAGE_ROOT", "Process")
    $clock = [Diagnostics.Stopwatch]::new()
    try {
        if ($env:OS -ne "Windows_NT") { throw "Windows scan integration requires Windows" }
        $lock = Enter-AmeRepositoryToolLock
        $outcome.StorageRoot = New-AmeWindowsScanStorage $RepositoryRoot $outcome.RunId
        Write-Host "Windows scan integration evidence: $($outcome.StorageRoot)"
        $stdout = Join-Path $outcome.StorageRoot "flutter.stdout.log"
        $stderr = Join-Path $outcome.StorageRoot "flutter.stderr.log"
        foreach ($path in @($FlutterPath, $stdout, $stderr)) {
            if ($path.IndexOfAny([char[]]@('"', '%', "`r", "`n", [char]0)) -ge 0) {
                throw "Windows scan command paths cannot be quoted safely"
            }
        }
        $utf8 = [Text.UTF8Encoding]::new($false)
        [IO.File]::WriteAllText($stdout, "", $utf8)
        [IO.File]::WriteAllText($stderr, "", $utf8)
        [Environment]::SetEnvironmentVariable(
            "CEDARFLAKE_AME_TEST_STORAGE_ROOT", $outcome.StorageRoot, "Process"
        )
        Initialize-AmeWindowsAccessibilityProcessJob
        $job = [AmeWindowsAccessibilityProcessJob]::new()
        $command = (
            '"{0}" test "integration_test\scan_workflow_test.dart" -d windows ' +
            '1>"{1}" 2>"{2}"'
        ) -f $FlutterPath, $stdout, $stderr
        $cmd = Join-Path ([Environment]::GetFolderPath("System")) "cmd.exe"
        $clock.Start()
        $process = $job.Start($cmd, "/d /v:off /s /c `"$command`"", $RepositoryRoot)
        $outcome.ExitCode = Wait-AmeWindowsScanProcess $process $job $clock $TimeoutSeconds
        if ($outcome.ExitCode -ne 0) {
            throw "Windows scan integration failed with exit code $($outcome.ExitCode)"
        }
    } catch {
        $outcome.RunFailure = $_.Exception
    } finally {
        if ($null -ne $job) {
            Invoke-AmeWindowsScanCleanup $outcome "close-owned-job" { $job.Dispose() }
        }
        if ($null -ne $process) {
            Invoke-AmeWindowsScanCleanup $outcome "wait-owned-process" {
                if (-not $process.WaitForExit(5000)) {
                    throw "Windows scan process did not exit after its owned Job Object closed"
                }
            }
            Invoke-AmeWindowsScanCleanup $outcome "dispose-owned-process" { $process.Dispose() }
        }
        Invoke-AmeWindowsScanCleanup $outcome "restore-environment" {
            Restore-AmeWindowsScanEnvironment $previousStorage
        }
        if ($null -ne $lock) {
            Invoke-AmeWindowsScanCleanup $outcome "release-tool-lock" { Exit-AmeRepositoryToolLock $lock }
        }
        $clock.Stop()
        $outcome.ElapsedMilliseconds = $clock.ElapsedMilliseconds
    }
    return $outcome
}

function Get-AmeWindowsScanCompletion {
    param([object]$Outcome)

    [ordered]@{
        format = "ame-windows-scan-integration-v1"
        runId = $Outcome.RunId
        startedUtc = $Outcome.StartedUtc
        completedUtc = [DateTime]::UtcNow.ToString("o")
        exitCode = $Outcome.ExitCode
        elapsedMilliseconds = $Outcome.ElapsedMilliseconds
        status = $(if ($Outcome.ExitCode -ne 0 -or $null -ne $Outcome.RunFailure -or $Outcome.CleanupFailures.Count -gt 0) { "failed" } else { "passed" })
        runFailure = $(if ($null -ne $Outcome.RunFailure) { $Outcome.RunFailure.Message } else { $null })
        cleanupFailures = @($Outcome.CleanupFailures | ForEach-Object {
            [ordered]@{ stage = $_.stage; message = $_.exception.Message }
        })
        storageRoot = $Outcome.StorageRoot
        storageDisposition = $(
            if ($null -eq $Outcome.StorageRoot) { "not-created" }
            elseif ($null -ne $Outcome.RunFailure) { "retained-failure-evidence" }
            else { "retained-evidence-no-owned-cleanup-proof" }
        )
    }
}

function Complete-AmeWindowsScanRun {
    param([Parameter(Mandatory = $true)] [object]$Outcome)

    if ($null -ne $Outcome.StorageRoot) {
        Invoke-AmeWindowsScanCleanup $Outcome "persist-completion" {
            Assert-AmeWindowsScanDirectoryChain $Outcome.StorageRoot
            $record = Get-AmeWindowsScanCompletion $Outcome | ConvertTo-Json -Depth 5
            [IO.File]::WriteAllText(
                (Join-Path $Outcome.StorageRoot "completion.json"),
                "$record`n", [Text.UTF8Encoding]::new($false)
            )
        }
    }
    $completion = Get-AmeWindowsScanCompletion $Outcome | ConvertTo-Json -Compress -Depth 5
    Write-Host "AME_WINDOWS_SCAN_COMPLETION $completion"
    # Retained evidence is not a cleanup failure. There is no identity-held proof
    # authorizing recursive deletion of catalog files or a replaced namespace.
    if ($null -ne $Outcome.RunFailure) { throw $Outcome.RunFailure }
    if ($Outcome.CleanupFailures.Count -gt 0) { throw $Outcome.CleanupFailures[0].exception }
    if ($Outcome.ExitCode -ne 0 -or $null -eq $Outcome.StorageRoot) {
        throw "Windows scan integration has no successful owned execution evidence"
    }
}
