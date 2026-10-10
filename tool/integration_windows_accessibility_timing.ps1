function New-AmeWindowsUiaTiming {
    return [pscustomobject]@{
        Stage = "loading-assemblies"
        StartedMilliseconds = 0L
        StageMilliseconds = [ordered]@{}
        EvidenceWriteMilliseconds = 0L
    }
}

function Set-AmeWindowsUiaTimingStage {
    param(
        [Parameter(Mandatory = $true)] [object]$Timing,
        [Parameter(Mandatory = $true)] [string]$Stage,
        [Parameter(Mandatory = $true)] [long]$ElapsedMilliseconds
    )

    if ($ElapsedMilliseconds -lt $Timing.StartedMilliseconds) {
        throw "Windows UIA timing must be monotonic"
    }
    $previous = $Timing.Stage
    if (-not $Timing.StageMilliseconds.Contains($previous)) {
        $Timing.StageMilliseconds[$previous] = 0L
    }
    $Timing.StageMilliseconds[$previous] += $ElapsedMilliseconds - $Timing.StartedMilliseconds
    $Timing.Stage = $Stage
    $Timing.StartedMilliseconds = $ElapsedMilliseconds
}

function Assert-AmeWindowsUiaTimingRecord {
    param([Parameter(Mandatory = $true)] [object]$Record)

    $properties = $Record.PSObject.Properties.Name
    if ($properties -notcontains "stageMilliseconds") { return }
    if ($Record.stageMilliseconds -isnot [pscustomobject]) {
        throw "Windows UIA stage timing must be a bounded object"
    }
    $stages = @($Record.stageMilliseconds.PSObject.Properties)
    if ($stages.Count -gt 10) { throw "Windows UIA stage timing exceeds the fixed roster" }
    foreach ($stage in $stages) {
        if ($stage.Name -cnotin @(
            "loading-assemblies", "loading-uia-types", "loading-uia-client", "locating-window",
            "activating-cache", "finding-elements", "disposing-cache", "reading-properties",
            "asserting-contract", "complete"
        )) { throw "Windows UIA stage timing has an unknown stage" }
        if (($stage.Value -isnot [int] -and $stage.Value -isnot [long]) -or
            $stage.Value -lt 0 -or $stage.Value -gt [int]::MaxValue) {
            throw "Windows UIA stage timing has an invalid duration"
        }
    }
    if ($properties -notcontains "evidenceWriteMilliseconds" -or
        ($Record.evidenceWriteMilliseconds -isnot [int] -and
            $Record.evidenceWriteMilliseconds -isnot [long]) -or
        $Record.evidenceWriteMilliseconds -lt 0 -or
        $Record.evidenceWriteMilliseconds -gt [int]::MaxValue) {
        throw "Windows UIA evidence timing has an invalid duration"
    }
}
