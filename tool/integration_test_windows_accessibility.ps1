[CmdletBinding(DefaultParameterSetName = "Live")]
param(
    [Parameter(ParameterSetName = "Captured")]
    [string]$CapturedOutputPath,
    [Parameter(ParameterSetName = "Captured")]
    [string]$CapturedProbeTranscriptPath,
    [string]$OutputPath,
    [Parameter(ParameterSetName = "Live")]
    [ValidateRange(60, 1800)]
    [int]$TimeoutSeconds = 900
)

$ErrorActionPreference = "Stop"
. (Join-Path $PSScriptRoot "quality_common.ps1")
. (Join-Path $PSScriptRoot "integration_windows_accessibility_process.ps1")
. (Join-Path $PSScriptRoot "integration_windows_accessibility_evidence.ps1")

$expectedProbePhases = @(
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
$probeProtocolPrefix = "AME_WINDOWS_UIA_PROBE_V1"
$probeTranscriptPrefix = "AME_WINDOWS_UIA_PHASE"
$probeDirectoryEnvironment = "CEDARFLAKE_AME_WINDOWS_UIA_PROBE_DIRECTORY"
$probeTokenEnvironment = "CEDARFLAKE_AME_WINDOWS_UIA_PROBE_TOKEN"
function Assert-AmeWindowsUiaProbeTranscript {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Transcript
    )

    $actual = @(
        $Transcript -split "`r?`n" |
            Where-Object { $_ -like "$probeTranscriptPrefix phase=*" }
    )
    $expected = @(
        foreach ($phase in $expectedProbePhases) {
            "$probeTranscriptPrefix phase=$phase result=ok"
        }
    )
    if ($actual.Count -ne $expected.Count) {
        throw (
            "Windows UIA probe transcript is incomplete: expected " +
            "$($expected.Count) phases, found $($actual.Count)"
        )
    }
    for ($index = 0; $index -lt $expected.Count; $index += 1) {
        if ($actual[$index] -cne $expected[$index]) {
            throw (
                "Windows UIA probe phase $($index + 1) was not the expected " +
                "'$($expected[$index])'"
            )
        }
    }
}

function Get-AmeWindowsAccessibilityRunnerProcessId {
    param(
        [Parameter(Mandatory = $true)]
        [string]$ResolvedRunnerPath,
        [Parameter(Mandatory = $true)]
        [AllowEmptyCollection()]
        [int[]]$PreexistingProcessIds,
        [Parameter(Mandatory = $true)]
        [DateTime]$StartedAt
    )

    $candidates = @(
        Get-Process -Name "cedarflake_ame" -ErrorAction SilentlyContinue |
            Where-Object {
                try {
                    $_.Path -and
                    $_.Path.Equals(
                        $ResolvedRunnerPath,
                        [System.StringComparison]::OrdinalIgnoreCase
                    ) -and
                    $_.StartTime -ge $StartedAt -and
                    $PreexistingProcessIds -notcontains $_.Id
                } catch {
                    $false
                }
            } |
            Sort-Object -Property StartTime -Descending
    )
    if ($candidates.Count -eq 0) {
        throw "The live Cedarflake Ame Windows runner was not found"
    }
    return [int]$candidates[0].Id
}

function Remove-AmeWindowsAccessibilityScratch {
    param(
        [Parameter(Mandatory = $true)]
        [string]$ScratchRoot,
        [Parameter(Mandatory = $true)]
        [string]$BuildRoot
    )

    $resolvedScratch = [System.IO.Path]::GetFullPath($ScratchRoot)
    $resolvedBuild = [System.IO.Path]::GetFullPath($BuildRoot)
    $buildPrefix = "$resolvedBuild$([System.IO.Path]::DirectorySeparatorChar)"
    if (-not $resolvedScratch.StartsWith(
        $buildPrefix,
        [System.StringComparison]::OrdinalIgnoreCase
    )) {
        throw "Windows accessibility scratch storage escaped build"
    }
    $ownedNames = [System.Collections.Generic.List[string]]::new()
    $ownedNames.Add("flutter.log")
    for ($sequence = 1; $sequence -le $expectedProbePhases.Count; $sequence += 1) {
        $suffix = $sequence.ToString("00")
        $ownedNames.Add("request-$suffix.tmp")
        $ownedNames.Add("request-$suffix.txt")
        $ownedNames.Add("ack-$suffix.tmp")
        $ownedNames.Add("ack-$suffix.txt")
        $ownedNames.Add("probe-$suffix.json")
        $ownedNames.Add("probe-$suffix.tmp")
    }
    foreach ($name in $ownedNames) {
        $path = Join-Path $resolvedScratch $name
        if (Test-Path -LiteralPath $path) {
            Remove-Item -LiteralPath $path -Force
        }
    }
    if (Test-Path -LiteralPath $resolvedScratch -PathType Container) {
        $unknown = @(Get-ChildItem -LiteralPath $resolvedScratch -Force)
        if ($unknown.Count -eq 0) {
            Remove-Item -LiteralPath $resolvedScratch -Force
        } else {
            Write-Warning (
                "Retained Windows accessibility scratch storage because it " +
                "contains unknown entries: $resolvedScratch"
            )
            return $false
        }
    }
    return $true
}

$repositoryRoot = Get-AmeRepositoryRoot
$toolchain = Get-AmeToolchain
if (-not $OutputPath) {
    $OutputPath = Join-Path `
        $repositoryRoot `
        "build\windows-accessibility-bridge-test.log"
}

$capturedOutputMode = -not [string]::IsNullOrWhiteSpace($CapturedOutputPath) -or
    -not [string]::IsNullOrWhiteSpace($CapturedProbeTranscriptPath)
if ($capturedOutputMode -and (
    [string]::IsNullOrWhiteSpace($CapturedOutputPath) -or
    [string]::IsNullOrWhiteSpace($CapturedProbeTranscriptPath)
)) {
    throw (
        "Captured accessibility validation requires both app output and a " +
        "native UIA probe transcript"
    )
}

$toolLock = $null
$locationPushed = $false
$capturedOutput = ""
$probeTranscript = ""
$exitCode = 0
$runFailure = $null
$cleanup = New-AmeWindowsAccessibilityCleanup
try {
    $toolLock = Enter-AmeRepositoryToolLock
    Push-Location $repositoryRoot
    $locationPushed = $true

    if ($capturedOutputMode) {
        $capturedOutput = Get-Content `
            -LiteralPath $CapturedOutputPath `
            -Raw `
            -Encoding UTF8
        $probeTranscript = Get-Content `
            -LiteralPath $CapturedProbeTranscriptPath `
            -Raw `
            -Encoding UTF8
    } else {
        if ($env:OS -ne "Windows_NT") {
            throw "Windows accessibility integration requires Windows"
        }
        Initialize-AmeWindowsAccessibilityProcessJob

        $buildRoot = [System.IO.Path]::GetFullPath(
            (Join-Path $repositoryRoot "build")
        )
        $scratchRoot = [System.IO.Path]::GetFullPath(
            (Join-Path $buildRoot "windows-accessibility-$PID-$([Guid]::NewGuid().ToString('N'))")
        )
        $buildPrefix = "$buildRoot$([System.IO.Path]::DirectorySeparatorChar)"
        if (-not $scratchRoot.StartsWith(
            $buildPrefix,
            [System.StringComparison]::OrdinalIgnoreCase
        )) {
            throw "Windows accessibility scratch storage must remain inside build"
        }
        New-Item -ItemType Directory -Path $scratchRoot | Out-Null
        $cleanup.ScratchPath = $scratchRoot
        $cleanup.ScratchDisposition = "retained"
        $cleanup.ScratchRetentionReason = "cleanup-not-reached"

        $processLogPath = Join-Path $scratchRoot "flutter.log"
        $runnerPath = Join-Path `
            $repositoryRoot `
            "build\windows\x64\runner\Debug\cedarflake_ame.exe"
        $resolvedRunnerPath = [System.IO.Path]::GetFullPath($runnerPath)
        $runnerProcessIdsBefore = @(
            Get-Process -Name "cedarflake_ame" -ErrorAction SilentlyContinue |
                Where-Object {
                    try {
                        $_.Path -and $_.Path.Equals(
                            $resolvedRunnerPath,
                            [System.StringComparison]::OrdinalIgnoreCase
                        )
                    } catch {
                        $false
                    }
                } |
                Select-Object -ExpandProperty Id
        )
        $probeToken = [Guid]::NewGuid().ToString("N")
        $previousProbeDirectory = [System.Environment]::GetEnvironmentVariable(
            $probeDirectoryEnvironment,
            "Process"
        )
        $previousProbeToken = [System.Environment]::GetEnvironmentVariable(
            $probeTokenEnvironment,
            "Process"
        )

        $cmd = Join-Path ([Environment]::GetFolderPath("System")) "cmd.exe"
        $flutterInvocation = (
            '"{0}" test "integration_test\windows_accessibility_bridge_test.dart" ' +
            '-d windows --no-pub 1>"{1}" 2>&1'
        ) -f $toolchain.Flutter, $processLogPath
        $cmdArguments = "/d /s /c `"$flutterInvocation`""
        $processJob = $null
        $process = $null
        $testStartedAt = Get-Date
        $validatedPhases = [System.Collections.Generic.List[string]]::new()
        try {
            [System.Environment]::SetEnvironmentVariable(
                $probeDirectoryEnvironment,
                $scratchRoot,
                "Process"
            )
            [System.Environment]::SetEnvironmentVariable(
                $probeTokenEnvironment,
                $probeToken,
                "Process"
            )
            $processJob = [AmeWindowsAccessibilityProcessJob]::new()
            $process = $processJob.Start($cmd, $cmdArguments, $repositoryRoot)
            $runClock = [System.Diagnostics.Stopwatch]::StartNew()
            while (-not $process.HasExited) {
                if ($runClock.Elapsed.TotalSeconds -ge $TimeoutSeconds) {
                    throw (
                        "Windows accessibility integration exceeded its " +
                        "$TimeoutSeconds-second wall-clock deadline"
                    )
                }
                $nextIndex = $validatedPhases.Count
                if ($nextIndex -lt $expectedProbePhases.Count) {
                    $sequence = ($nextIndex + 1).ToString("00")
                    $requestPath = Join-Path $scratchRoot "request-$sequence.txt"
                    if (Test-Path -LiteralPath $requestPath -PathType Leaf) {
                        $phase = $expectedProbePhases[$nextIndex]
                        $expectedRequest = (
                            "$probeProtocolPrefix|$probeToken|$sequence|$phase"
                        )
                        $request = [System.IO.File]::ReadAllText($requestPath).Trim()
                        if ($request -cne $expectedRequest) {
                            throw (
                                "Windows UIA probe request $sequence did not " +
                                "match the fail-closed phase contract"
                            )
                        }
                        $runnerProcessId = `
                            Get-AmeWindowsAccessibilityRunnerProcessId `
                                -ResolvedRunnerPath $resolvedRunnerPath `
                                -PreexistingProcessIds $runnerProcessIdsBefore `
                                -StartedAt $testStartedAt
                        $remainingSeconds = $TimeoutSeconds - $runClock.Elapsed.TotalSeconds
                        $probeTimeout = [TimeSpan]::FromSeconds(
                            [Math]::Min(8, [Math]::Max(0, $remainingSeconds))
                        )
                        Invoke-AmeWindowsAccessibilityProbe `
                            -ProbeScriptPath (Join-Path $PSScriptRoot "integration_windows_accessibility_probe.ps1") `
                            -TargetProcessId $runnerProcessId `
                            -Phase $phase `
                            -ResultPath (Join-Path $scratchRoot "probe-$sequence.json") `
                            -Token $probeToken `
                            -Timeout $probeTimeout
                        $acknowledgementPath = Join-Path `
                            $scratchRoot `
                            "ack-$sequence.txt"
                        $acknowledgementDraftPath = Join-Path `
                            $scratchRoot `
                            "ack-$sequence.tmp"
                        [System.IO.File]::WriteAllText(
                            $acknowledgementDraftPath,
                            "$probeProtocolPrefix|$probeToken|$sequence|ok`n",
                            [System.Text.UTF8Encoding]::new($false)
                        )
                        Move-Item `
                            -LiteralPath $acknowledgementDraftPath `
                            -Destination $acknowledgementPath
                        $validatedPhases.Add($phase)
                    }
                }
                Start-Sleep -Milliseconds 25
            }
            $process.WaitForExit()
            $exitCode = $processJob.PrimaryExitCode
        } catch {
            $runFailure = $_.Exception
        } finally {
            Complete-AmeWindowsAccessibilityCleanup `
                -Cleanup $cleanup -Job $processJob -Process $process `
                -EnvironmentValues ([ordered]@{
                    $probeDirectoryEnvironment = $previousProbeDirectory
                    $probeTokenEnvironment = $previousProbeToken
                }) `
                -CaptureOutput {
                    if (Test-Path -LiteralPath $processLogPath -PathType Leaf) {
                        [System.IO.File]::ReadAllText($processLogPath)
                    } else { "" }
                } `
                -CaptureTranscript {
                    @(
                        foreach ($phase in $validatedPhases) {
                            "$probeTranscriptPrefix phase=$phase result=ok"
                        }
                    ) -join [Environment]::NewLine
                } `
                -RemoveScratch {
                    Remove-AmeWindowsAccessibilityScratch `
                        -ScratchRoot $scratchRoot -BuildRoot $buildRoot
                }
            $capturedOutput = $cleanup.CapturedOutput
            $probeTranscript = $cleanup.ProbeTranscript
        }
    }

    if ($null -eq $runFailure -and $exitCode -eq 0) {
        if ($capturedOutput -match "Failed to update ui::AXTree") {
            throw "Windows AccessibilityBridge rejected a semantics update"
        }
        Assert-AmeWindowsUiaProbeTranscript -Transcript $probeTranscript
    }
} catch {
    if ($null -eq $runFailure) { $runFailure = $_.Exception }
} finally {
    if ($locationPushed) {
        Invoke-AmeWindowsAccessibilityCleanupStep $cleanup "pop-location" { Pop-Location }
    }
    if ($null -ne $toolLock) {
        Invoke-AmeWindowsAccessibilityCleanupStep $cleanup "release-tool-lock" {
            Exit-AmeRepositoryToolLock $toolLock
        }
    }
}
Complete-AmeWindowsAccessibilityRun `
    -OutputPath $OutputPath -CapturedOutput $capturedOutput `
    -ProbeTranscript $probeTranscript -ExitCode $exitCode `
    -RunFailure $runFailure -Cleanup $cleanup
