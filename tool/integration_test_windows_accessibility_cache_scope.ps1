[CmdletBinding()]
param()

$ErrorActionPreference = "Stop"
. (Join-Path $PSScriptRoot "integration_windows_accessibility_cleanup.ps1")

function Assert-AmeAccessibilityCacheScope {
    param([bool]$Condition, [string]$Message)
    if (-not $Condition) { throw "Accessibility cache-scope regression: $Message" }
}

function New-AmeAccessibilityCacheScopeFixture {
    param([bool]$FailDispose = $false)
    $trace = [Collections.Generic.List[string]]::new()
    $active = [pscustomobject]@{ Trace = $trace; FailDispose = $FailDispose }
    $active | Add-Member ScriptMethod Dispose {
        $this.Trace.Add("dispose")
        if ($this.FailDispose) { throw [IO.IOException]::new("dispose failure") }
    }
    $request = [pscustomobject]@{ Trace = $trace; Active = $active }
    $request | Add-Member ScriptMethod Activate {
        $this.Trace.Add("activate")
        return $this.Active
    }
    return [pscustomobject]@{ Trace = $trace; Request = $request }
}

foreach ($failProgress in @($false, $true)) {
    foreach ($failDispose in @($false, $true)) {
        $fixture = New-AmeAccessibilityCacheScopeFixture -FailDispose $failDispose
        $readFailure = [InvalidOperationException]::new("original UIA read failure")
        $caught = $null
        try {
            Invoke-AmeWindowsAccessibilityCacheScope -CacheRequest $fixture.Request `
                -Read {
                    $fixture.Trace.Add("read")
                    throw $readFailure
                } `
                -BeforeDispose {
                    $fixture.Trace.Add("dispose-progress")
                    if ($failProgress) { throw [IO.IOException]::new("progress write failure") }
                } | Out-Null
        } catch { $caught = $_.Exception }
        Assert-AmeAccessibilityCacheScope ([object]::ReferenceEquals($caught, $readFailure)) "cleanup replaced the original read error"
        Assert-AmeAccessibilityCacheScope (($fixture.Trace -join "|") -ceq "activate|read|dispose-progress|dispose") "failed read skipped or repeated release"
        $expectedStages = @(
            if ($failProgress) { "cache-dispose-progress" }
            if ($failDispose) { "dispose-cache" }
        )
        $records = @($caught.Data["ameWindowsUiaCacheCleanupFailures"])
        if ($expectedStages.Count -eq 0) {
            Assert-AmeAccessibilityCacheScope (-not $caught.Data.Contains("ameWindowsUiaCacheCleanupFailures")) "invented cleanup failure"
        } else {
            Assert-AmeAccessibilityCacheScope (($records.stage -join "|") -ceq ($expectedStages -join "|")) "cleanup stages were lost or reordered"
        }
    }
}

foreach ($count in @(0, 1, 2)) {
    $fixture = New-AmeAccessibilityCacheScopeFixture
    $collection = [Collections.Generic.List[object]]::new()
    for ($index = 0; $index -lt $count; $index += 1) { $collection.Add([pscustomobject]@{ Index = $index }) }
    $actual = Invoke-AmeWindowsAccessibilityCacheScope -CacheRequest $fixture.Request `
        -Read { ,$collection } -BeforeDispose { $fixture.Trace.Add("dispose-progress") }
    Assert-AmeAccessibilityCacheScope ([object]::ReferenceEquals($actual, $collection)) "successful collection was enumerated or replaced"
    Assert-AmeAccessibilityCacheScope (($fixture.Trace -join "|") -ceq "activate|dispose-progress|dispose") "successful read did not release once"
}

& {
    $fixture = New-AmeAccessibilityCacheScopeFixture -FailDispose $true
    $progressFailure = [IO.IOException]::new("first cleanup failure")
    $caught = $null
    try {
        Invoke-AmeWindowsAccessibilityCacheScope -CacheRequest $fixture.Request `
            -Read { "successful read" } -BeforeDispose { throw $progressFailure } | Out-Null
    } catch { $caught = $_.Exception }
    Assert-AmeAccessibilityCacheScope ([object]::ReferenceEquals($caught, $progressFailure)) "second cleanup failure replaced the first"
    Assert-AmeAccessibilityCacheScope (($caught.Data["ameWindowsUiaCacheCleanupFailures"].stage -join "|") -ceq "cache-dispose-progress|dispose-cache") "compound cleanup evidence was lost"
    Assert-AmeAccessibilityCacheScope ($fixture.Trace[-1] -ceq "dispose") "progress error prevented disposal"
}

Write-Host "Windows accessibility compiler-free cache-scope guardrails passed."
