Set-StrictMode -Version Latest

function New-AmeWindowsScanProgress {
    param([Parameter(Mandatory = $true)] [IO.TextReader]$Reader)

    [pscustomobject]@{
        Reader = $Reader
        Buffer = [char[]]::new(4096)
        PendingLine = [Text.StringBuilder]::new()
        DiscardingLine = $false
        ExecutionStartedMilliseconds = $null
    }
}

function Test-AmeWindowsScanStartedEvent {
    param([string]$Line)

    if (-not $Line.TrimStart().StartsWith("[", [StringComparison]::Ordinal)) { return $false }
    try { $events = @(ConvertFrom-Json -InputObject $Line -ErrorAction Stop) } catch { return $false }
    if ($events.Count -ne 1 -or $null -eq $events[0]) { return $false }
    $event = $events[0].PSObject.Properties["event"]
    return $null -ne $event -and $event.Value -ceq "test.startedProcess"
}

function Add-AmeWindowsScanOutput {
    param([object]$Progress, [string]$Text, [TimeSpan]$Elapsed)

    foreach ($character in $Text.ToCharArray()) {
        if ($character -eq "`n") {
            if (-not $Progress.DiscardingLine -and
                (Test-AmeWindowsScanStartedEvent $Progress.PendingLine.ToString())) {
                if ($null -ne $Progress.ExecutionStartedMilliseconds) {
                    throw "Windows scan integration reported more than one started test process"
                }
                $Progress.ExecutionStartedMilliseconds = $Elapsed.TotalMilliseconds
            }
            $null = $Progress.PendingLine.Clear()
            $Progress.DiscardingLine = $false
        } elseif (-not $Progress.DiscardingLine) {
            if ($Progress.PendingLine.Length -ge 16384) {
                $null = $Progress.PendingLine.Clear()
                $Progress.DiscardingLine = $true
            } else {
                $null = $Progress.PendingLine.Append($character)
            }
        }
    }
}

function Read-AmeWindowsScanProgress {
    param([object]$Progress, [TimeSpan]$Elapsed)

    for ($chunk = 0; $chunk -lt 16; $chunk++) {
        $read = $Progress.Reader.Read($Progress.Buffer, 0, $Progress.Buffer.Length)
        if ($read -eq 0) { return $true }
        Add-AmeWindowsScanOutput $Progress ([string]::new($Progress.Buffer, 0, $read)) $Elapsed
    }
    return $false
}

function Assert-AmeWindowsScanPhaseDeadline {
    param([object]$Progress, [TimeSpan]$Elapsed, [int]$BuildTimeoutSeconds, [int]$TimeoutSeconds)

    if ($null -eq $Progress.ExecutionStartedMilliseconds) {
        $phase = "build/startup"
        $phaseSeconds = $Elapsed.TotalSeconds
        $limit = $BuildTimeoutSeconds
    } else {
        $phase = "execution"
        $phaseSeconds = ($Elapsed.TotalMilliseconds - $Progress.ExecutionStartedMilliseconds) / 1000
        $limit = $TimeoutSeconds
    }
    if ($phaseSeconds -ge $limit) {
        throw [TimeoutException]::new(
            "Windows scan integration exceeded its $limit-second $phase deadline"
        )
    }
}

function Wait-AmeWindowsScanTestProcess {
    param(
        [object]$Process, [object]$Job, [object]$Clock, [object]$Progress,
        [int]$BuildTimeoutSeconds, [int]$TimeoutSeconds
    )

    while ($true) {
        Assert-AmeWindowsScanPhaseDeadline $Progress $Clock.Elapsed $BuildTimeoutSeconds $TimeoutSeconds
        $outputDrained = Read-AmeWindowsScanProgress $Progress $Clock.Elapsed
        Assert-AmeWindowsScanPhaseDeadline $Progress $Clock.Elapsed $BuildTimeoutSeconds $TimeoutSeconds
        if ($Process.HasExited -and $outputDrained) { break }
        $null = $Process.WaitForExit(100)
    }
    $exitCode = [int]$Job.PrimaryExitCode
    if ($exitCode -eq 0 -and $null -eq $Progress.ExecutionStartedMilliseconds) {
        throw "Windows scan integration exited without its Flutter test.startedProcess event"
    }
    return $exitCode
}
