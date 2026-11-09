$ErrorActionPreference = "Stop"
. (Join-Path $PSScriptRoot "acceptance_scratch_storage.ps1")

function Assert-ScratchResult {
    param([bool]$Condition, [string]$Message)
    if (-not $Condition) { throw $Message }
}

$name = "storage-check-" + [Guid]::NewGuid().ToString("N")
$parent = Join-Path (Get-AmeRepositoryRoot) "build\test\derived\temporary"
$expected = Join-Path $parent $name
$utf8 = [Text.UTF8Encoding]::new($false)

Invoke-AmeAcceptanceScratch -Name $name -Action {
    param($root)
    [IO.File]::WriteAllText((Join-Path $root "catalog.sqlite3"), "disposable", $utf8)
}
Assert-ScratchResult (-not (Test-Path -LiteralPath $expected)) "Successful acceptance left scratch behind"

$rejected = $false
try {
    Invoke-AmeAcceptanceScratch -Name $name -Action {
        param($root)
        [IO.File]::WriteAllText((Join-Path $root "partial.sqlite3"), "partial", $utf8)
        throw "expected acceptance failure"
    }
} catch {
    $rejected = $_.Exception.Message -ceq "expected acceptance failure"
}
Assert-ScratchResult $rejected "Acceptance failure was not preserved"
Assert-ScratchResult (-not (Test-Path -LiteralPath $expected)) "Failed acceptance left scratch behind"

foreach ($invalid in @("../sources", "r4-retained-profile\baseline", "")) {
    $rejected = $false
    $execution = @{ started = $false }
    try { Invoke-AmeAcceptanceScratch -Name $invalid -Action { $execution.started = $true } }
    catch { $rejected = $true }
    Assert-ScratchResult $rejected "Invalid scratch name was accepted"
    Assert-ScratchResult (-not $execution.started) "Invalid scratch action ran"
}

$held = $null
try {
    $rejected = $false
    try {
        Invoke-AmeAcceptanceScratch -Name $name -Action {
            param($root)
            [IO.File]::WriteAllText((Join-Path $root "busy.sqlite3"), "busy", $utf8)
            $script:held = [IO.File]::Open((Join-Path $root "busy.sqlite3"), [IO.FileMode]::Open, [IO.FileAccess]::Read, [IO.FileShare]::None)
        }
    } catch { $rejected = $true }
    Assert-ScratchResult $rejected "Cleanup removed an active file"
    Assert-ScratchResult (Test-Path -LiteralPath (Join-Path $expected "busy.sqlite3")) "Active file was lost"
} finally {
    if ($null -ne $held) { $held.Dispose() }
}
Invoke-AmeAcceptanceScratch -Name $name -Action {
    param($root)
    Assert-ScratchResult (-not (Test-Path -LiteralPath (Join-Path $root "busy.sqlite3"))) "Retired scratch was not reclaimed"
    $rejected = $false
    $execution = @{ started = $false }
    try { Invoke-AmeAcceptanceScratch -Name $name -Action { $execution.started = $true } }
    catch { $rejected = $true }
    Assert-ScratchResult $rejected "Concurrent scratch lease was accepted"
    Assert-ScratchResult (-not $execution.started) "Concurrent scratch action ran"
}
Assert-ScratchResult (-not (Test-Path -LiteralPath $expected)) "Recovered scratch was retained"

try {
    $rejected = $false
    try {
        Invoke-AmeAcceptanceScratch -Name $name -Action {
            param($root)
            $script:held = [IO.File]::Open((Join-Path $root "busy.sqlite3"), [IO.FileMode]::CreateNew, [IO.FileAccess]::Write, [IO.FileShare]::None)
            throw "expected acceptance failure"
        }
    } catch {
        $failure = $_.Exception
        $rejected = $failure -is [AggregateException] -and $failure.InnerExceptions.Count -eq 2
        Assert-ScratchResult $rejected "Combined action and cleanup failures lost their causes"
        foreach ($cause in $failure.InnerExceptions) {
            Assert-ScratchResult ($failure.Message.Contains($cause.Message)) "The CI diagnostic hides an inner failure"
        }
    }
    Assert-ScratchResult $rejected "A failed action and busy scratch were accepted"
} finally {
    if ($null -ne $held) { $held.Dispose() }
    Invoke-AmeAcceptanceScratch -Name $name -Action { param($root) }
}

$outside = Join-Path $parent ($name + "-outside")
$junction = Join-Path $expected "linked"
[IO.Directory]::CreateDirectory($outside) | Out-Null
[IO.File]::WriteAllText((Join-Path $outside "sentinel.txt"), "keep", $utf8)
try {
    $rejected = $false
    $execution = @{ linked = $false }
    try {
        Invoke-AmeAcceptanceScratch -Name $name -Action {
            param($root)
            New-Item -ItemType Junction -Path (Join-Path $root "linked") -Target $outside | Out-Null
            $execution.linked = $true
        }
    } catch { $rejected = $true }
    Assert-ScratchResult $execution.linked "Junction fixture was not created"
    Assert-ScratchResult $rejected "Cleanup followed a junction"
    Assert-ScratchResult ([IO.File]::ReadAllText((Join-Path $outside "sentinel.txt"), $utf8) -ceq "keep") "Junction target was changed"
} finally {
    if (Test-Path -LiteralPath $junction) { [IO.Directory]::Delete($junction) }
    Invoke-AmeAcceptanceScratch -Name $name -Action { param($root) }
    Remove-Item -LiteralPath (Join-Path $outside "sentinel.txt")
    [IO.Directory]::Delete($outside)
}

[IO.Directory]::CreateDirectory($expected) | Out-Null
[IO.File]::WriteAllText((Join-Path $expected "foreign.txt"), "keep", $utf8)
try {
    $rejected = $false
    $execution = @{ started = $false }
    try { Invoke-AmeAcceptanceScratch -Name $name -Action { $execution.started = $true } }
    catch { $rejected = $true }
    Assert-ScratchResult $rejected "Unowned directory was removed"
    Assert-ScratchResult (-not $execution.started) "Foreign directory was adopted"
    Assert-ScratchResult ([IO.File]::ReadAllText((Join-Path $expected "foreign.txt"), $utf8) -ceq "keep") "Foreign file was changed"
} finally {
    Assert-AmeAcceptanceScratchPath $expected | Out-Null
    Remove-Item -LiteralPath (Join-Path $expected "foreign.txt")
    [IO.Directory]::Delete($expected)
}

$previousIncremental = $env:CARGO_INCREMENTAL
$previousBuildIncremental = $env:CARGO_BUILD_INCREMENTAL
try {
    $env:CARGO_INCREMENTAL = $null
    $env:CARGO_BUILD_INCREMENTAL = $null
    Invoke-AmeAcceptanceScratch -Name ($name + "-compile") -Action {
        param($root)
        [IO.Directory]::CreateDirectory((Join-Path $root "src")) | Out-Null
        $manifest = "[package]`nname = 'scratch_storage_probe'`nversion = '0.0.0'`nedition = '2024'`n"
        [IO.File]::WriteAllText((Join-Path $root "Cargo.toml"), $manifest, $utf8)
        [IO.File]::WriteAllText((Join-Path $root "src/lib.rs"), "pub fn probe() -> u8 { 1 }`n", $utf8)
        $artifacts = Join-Path $root "artifacts"
        Invoke-AmeChecked (Get-AmeToolchain).Cargo @(
            "check", "--offline", "--manifest-path", (Join-Path $root "Cargo.toml"), "--target-dir", $artifacts
        )
        $incremental = Join-Path $artifacts "debug/incremental"
        $files = @()
        if (Test-Path -LiteralPath $incremental) { $files = @(Get-ChildItem -LiteralPath $incremental -File -Recurse -Force) }
        Assert-ScratchResult ($files.Count -eq 0) "Project configuration created an incremental compiler cache"
    }
} finally {
    $env:CARGO_INCREMENTAL = $previousIncremental
    $env:CARGO_BUILD_INCREMENTAL = $previousBuildIncremental
}
Assert-ScratchResult (-not (Test-Path -LiteralPath ($expected + "-compile"))) "Compiler probe retained its outputs"

Write-Output "acceptance_scratch_storage_passed"
