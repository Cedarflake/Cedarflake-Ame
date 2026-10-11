$ErrorActionPreference = "Stop"
. (Join-Path $PSScriptRoot "quality_common.ps1")
. (Join-Path $PSScriptRoot "acceptance_r2c_change_driven_reliability_common.ps1")
. (Join-Path $PSScriptRoot "acceptance_r2c_fixture_cleanup.ps1")
. (Join-Path $PSScriptRoot "acceptance_r2c_worker_report_guardrail.ps1")

function Assert-WorkerReport {
    param([bool]$Condition, [string]$Message)
    if (-not $Condition) { throw "R2c-R worker-report regression: $Message" }
}

function Assert-WorkerReportRefusal {
    param([scriptblock]$Action, [string]$Expected)
    $failure = $null
    try { & $Action } catch { $failure = $_.Exception }
    Assert-WorkerReport ($null -ne $failure) "missing refusal: $Expected"
    Assert-WorkerReport ($failure.Message.Contains($Expected)) "wrong refusal: $($failure.Message)"
}

$cargo = (Get-AmeToolchain).Cargo
$sourceBudgetState = New-AmeR2cRDeletionAuditState
try {
    for ($index = 0; $index -lt 9; $index += 1) {
        Add-AmeR2cRDeletionAuditSource -State $sourceBudgetState -SourceText "# source $index" `
            -Label "source $index" -SourcePath $null -IsRoot ($index -eq 0) -Depth 0 | Out-Null
    }
    Assert-WorkerReport ($sourceBudgetState.Sources.Count -eq 9) "nine-source closure was not admitted"
    Assert-WorkerReportRefusal {
        Add-AmeR2cRDeletionAuditSource -State $sourceBudgetState -SourceText "# rejected tenth source" `
            -Label "source 10" -SourcePath $null -IsRoot $false -Depth 0 | Out-Null
    } "budget=source-count limit=9 actual=10"
} finally { Close-AmeR2cRDeletionAuditState -State $sourceBudgetState }
$buildArguments = @(Get-AmeR2cRTestBuildArguments)
Assert-AmeR2cRCargoInvocation -Kind CargoTestBuild -FileName $cargo -Arguments $buildArguments
foreach ($case in @(Get-AmeR2cRCaseDefinitions)) {
    Assert-AmeR2cRCargoInvocation -Kind CargoExactTest -FileName $cargo `
        -Arguments @(Get-AmeR2cRCaseArguments -Case $case)
}
$invalidBuilds = @(
    @{ Arguments = @($buildArguments | Where-Object { $_ -cne "--no-run" }) },
    @{ Arguments = @($buildArguments + "--release") },
    @{ Arguments = @("test", "--locked", "--manifest-path", "other\Cargo.toml", "--lib", "--no-run", "--jobs", "1") },
    @{ Arguments = @("test", "--locked", "--manifest-path", "rust\Cargo.toml", "--lib", "--no-run", "--jobs", "2") }
)
foreach ($invalid in $invalidBuilds) {
    Assert-WorkerReportRefusal {
        Assert-AmeR2cRCargoInvocation -Kind CargoTestBuild -FileName $cargo -Arguments $invalid.Arguments
    } "non-matrix Cargo command"
}
Assert-WorkerReportRefusal {
    Assert-AmeR2cRCargoInvocation -Kind CargoExactTest -FileName $cargo -Arguments $buildArguments
} "non-matrix Cargo command"
Assert-WorkerReportRefusal {
    Assert-AmeR2cRCargoInvocation -Kind CargoTestBuild -FileName ($cargo + ".wrong") -Arguments $buildArguments
} "rejected the Cargo executable"
Assert-WorkerReportRefusal {
    Assert-AmeR2cRCargoInvocation -Kind CargoTestBuild -FileName $cargo -Arguments $buildArguments -Detached
} "cannot detach"

$cargoOwnerText = [IO.File]::ReadAllText((Join-Path $PSScriptRoot "acceptance_r2c_cargo_invocation.ps1"), [Text.Encoding]::UTF8)
$tokens = $null
$parseErrors = $null
$cargoOwnerAst = [Management.Automation.Language.Parser]::ParseInput($cargoOwnerText, [ref]$tokens, [ref]$parseErrors)
$boundaryFunctions = @{}
foreach ($definition in @($cargoOwnerAst.FindAll({
    param($node)
    $node -is [Management.Automation.Language.FunctionDefinitionAst]
}, $true))) {
    $boundaryFunctions[$definition.Name] = @([pscustomobject]@{ Ast = $definition })
}
Assert-AmeR2cRCargoBoundaryContract -State ([pscustomobject]@{ Functions = $boundaryFunctions })
foreach ($name in @("Get-AmeR2cRTestBuildArguments", "Assert-AmeR2cRCargoInvocation")) {
    $changed = @{} + $boundaryFunctions
    $changed[$name] = @([pscustomobject]@{ Ast = [pscustomobject]@{ Extent = [pscustomobject]@{ Text = "function $name { }" } } })
    Assert-WorkerReportRefusal {
        Assert-AmeR2cRCargoBoundaryContract -State ([pscustomobject]@{ Functions = $changed })
    } "Cargo boundary item $name changed"
    $changed.Remove($name)
    Assert-WorkerReportRefusal {
        Assert-AmeR2cRCargoBoundaryContract -State ([pscustomobject]@{ Functions = $changed })
    } "Cargo boundary item $name was missing or ambiguous"
}

$passingLines = @("test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 42 filtered out; finished in 0.00s")
Assert-AmeR2cRWorkerReportPhase -Phase test -Result ([pscustomobject]@{
    TimedOut = $false; ExitCode = 0; Lines = $passingLines
})
foreach ($summary in @(
    "test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 43 filtered out; finished in 0.00s",
    "test result: ok. 1 passed; 0 failed; 1 ignored; 0 measured; 41 filtered out; finished in 0.00s",
    "test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 42 filtered out; finished in 0.00s"
)) {
    Assert-WorkerReportRefusal {
        Assert-AmeR2cRWorkerReportPhase -Phase test -Result ([pscustomobject]@{
            TimedOut = $false; ExitCode = 0; Lines = @($summary)
        })
    } "worker-report test output tail:"
}
$tailFailure = $null
try {
    Assert-AmeR2cRWorkerReportPhase -Phase test -Result ([pscustomobject]@{
        TimedOut = $true; ExitCode = -1
        Lines = @("excluded-first-line") + @(1..30 | ForEach-Object { "line-$_ " + ("x" * 600) })
    })
} catch { $tailFailure = $_.Exception }
Assert-WorkerReport ($null -ne $tailFailure -and $tailFailure.Message.Contains("five-minute deadline")) "test timeout was lost"
Assert-WorkerReport ($tailFailure.Message.Contains("line-30 ") -and
    -not $tailFailure.Message.Contains("line-6 ") -and
    -not $tailFailure.Message.Contains("excluded-first-line") -and
    $tailFailure.Message.Length -lt 13000) "failure output exceeded its tail bounds"
$tailLines = @($tailFailure.Message -split "`n" | Where-Object { $_ -match '^line-[0-9]+ ' })
Assert-WorkerReport ($tailLines.Count -eq 24 -and
    @($tailLines | Where-Object { $_.Length -gt 512 }).Count -eq 0) "truncation marker exceeded the per-line bound"

$preservedEnvironment = @{}
foreach ($name in @("CARGO_BUILD_JOBS", "RUST_TEST_THREADS")) {
    $preservedEnvironment[$name] = [Environment]::GetEnvironmentVariable($name, "Process")
}
try {
    foreach ($scenario in @("pass", "build-failure", "build-timeout", "test-timeout", "invalid-result", "cleanup-failure", "cleanup-throw", "combined-failure")) {
        & {
            [Environment]::SetEnvironmentVariable("CARGO_BUILD_JOBS", "7", "Process")
            [Environment]::SetEnvironmentVariable("RUST_TEST_THREADS", [System.Management.Automation.Language.NullString]::Value, "Process")
            $events = [Collections.Generic.List[string]]::new()
            function Enter-AmeRepositoryToolLock { $events.Add("lock"); return [pscustomobject]@{ Owned = $true } }
            function Exit-AmeRepositoryToolLock {
                param($Mutex)
                Assert-WorkerReport $Mutex.Owned "wrong lock was released"
                $events.Add("unlock")
                if ($scenario -ceq "combined-failure") { throw "lock-release-failure" }
            }
            function New-AmeR2cRGuardrailDisposableRoot {
                param($AnchorPath, $Nonce)
                $events.Add("fixture")
                return [pscustomobject]@{ Path = (Join-Path $AnchorPath "unused-owned-fixture") }
            }
            function Close-AmeR2cRGuardrailFixture {
                param($Cleanup, $Label, $Fixture)
                $events.Add("cleanup")
                if ($scenario -ceq "cleanup-throw") { throw "unexpected-cleanup-failure" }
                if ($scenario -cin @("cleanup-failure", "combined-failure")) {
                    $Cleanup.Failures.Add([pscustomobject]@{
                        stage = "fixture"; exception = [InvalidOperationException]::new("fixture-cleanup-failure")
                    })
                }
            }
            function Invoke-AmeR2cROwnedProcess {
                param($ExecutableKind, $FileName, $Arguments, $WorkingDirectory, $OutputPath, $TimeoutMilliseconds)
                $events.Add($ExecutableKind)
                Assert-WorkerReport ([Environment]::GetEnvironmentVariable("CARGO_BUILD_JOBS", "Process") -ceq "1" -and
                    [Environment]::GetEnvironmentVariable("RUST_TEST_THREADS", "Process") -ceq "1") "child concurrency was not bounded"
                Assert-AmeR2cRCargoInvocation -Kind $ExecutableKind -FileName $FileName -Arguments $Arguments
                if ($ExecutableKind -ceq "CargoTestBuild") {
                    Assert-WorkerReport ($TimeoutMilliseconds -eq 900000) "preparation deadline changed"
                    return [pscustomobject]@{
                        TimedOut = ($scenario -ceq "build-timeout")
                        ExitCode = $(if ($scenario -ceq "build-failure") { 101 } else { 0 })
                        Lines = @("compiler-detail")
                    }
                }
                Assert-WorkerReport ($TimeoutMilliseconds -eq 300000) "test deadline changed"
                return [pscustomobject]@{
                    TimedOut = ($scenario -cin @("test-timeout", "combined-failure"))
                    ExitCode = 0
                    Lines = $(if ($scenario -ceq "invalid-result") { @("missing-summary") } else { $passingLines })
                }
            }
            $failure = $null
            try { Invoke-AmeR2cRWorkerReportGuardrail -AnchorPath $PSScriptRoot | Out-Null } catch { $failure = $_.Exception }
            $expectedEvents = "lock,fixture,CargoTestBuild,CargoExactTest,cleanup,unlock"
            if ($scenario -cin @("build-failure", "build-timeout")) {
                $expectedEvents = "lock,fixture,CargoTestBuild,cleanup,unlock"
            }
            Assert-WorkerReport (($events -join ',') -ceq $expectedEvents) "$scenario lost ordered preparation or independent cleanup"
            Assert-WorkerReport ([Environment]::GetEnvironmentVariable("CARGO_BUILD_JOBS", "Process") -ceq "7" -and
                $null -eq [Environment]::GetEnvironmentVariable("RUST_TEST_THREADS", "Process")) "$scenario did not restore environment"
            if ($scenario -ceq "pass") {
                Assert-WorkerReport ($null -eq $failure) "successful exact test failed: $failure"
            } else {
                Assert-WorkerReport ($null -ne $failure) "$scenario was accepted"
                $expected = switch ($scenario) {
                    "build-failure" { "preparation failed with exit code 101" }
                    "build-timeout" { "preparation exceeded its fifteen-minute deadline" }
                    "test-timeout" { "test exceeded its five-minute deadline" }
                    "invalid-result" { "produced no test result" }
                    "cleanup-failure" { "fixture-cleanup-failure" }
                    "cleanup-throw" { "unexpected-cleanup-failure" }
                    "combined-failure" { "test exceeded its five-minute deadline" }
                }
                Assert-WorkerReport ($failure.Message.Contains($expected)) "$scenario lost its primary failure: $($failure.Message)"
                if ($scenario -ceq "combined-failure") {
                    Assert-WorkerReport ($failure.Data["ameR2cRFixtureCleanupFailures"].Count -eq 2) "secondary cleanup failures were lost"
                }
            }
        }
    }
} finally {
    foreach ($name in $preservedEnvironment.Keys) {
        $value = $preservedEnvironment[$name]
        if ($null -eq $value) { $value = [System.Management.Automation.Language.NullString]::Value }
        [Environment]::SetEnvironmentVariable($name, $value, "Process")
    }
}
Write-Output "r2c_r_worker_report_guardrail_boundaries_passed"
