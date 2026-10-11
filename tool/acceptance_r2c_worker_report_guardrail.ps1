function Assert-AmeR2cRWorkerReportPhase {
    param(
        [Parameter(Mandatory = $true)][ValidateSet("build", "test")][string]$Phase,
        [Parameter(Mandatory = $true)][psobject]$Result
    )

    try {
        if ($Result.TimedOut) {
            if ($Phase -ceq "build") {
                throw "R2c-R worker-report preparation exceeded its fifteen-minute deadline"
            }
            throw "R2c-R worker-report tamper behavior test exceeded its five-minute deadline"
        }
        if ($Phase -ceq "build") {
            if ($Result.ExitCode -ne 0) {
                throw "R2c-R worker-report preparation failed with exit code $($Result.ExitCode)"
            }
        } else {
            Assert-AmeR2cRCaseResult -Label "worker-report-binding-tamper-lint" `
                -Lines $Result.Lines -ExitCode $Result.ExitCode
        }
    } catch {
        $tail = [Collections.Generic.List[string]]::new()
        $lines = @($Result.Lines)
        for ($index = [Math]::Max(0, $lines.Count - 24); $index -lt $lines.Count; $index += 1) {
            $line = [string]$lines[$index]
            if ($line.Length -gt 512) {
                $suffix = " [truncated]"
                $line = $line.Substring(0, 512 - $suffix.Length) + $suffix
            }
            $tail.Add($line)
        }
        throw [InvalidOperationException]::new(
            ($_.Exception.Message + "`nworker-report $Phase output tail:`n" + ($tail -join "`n")),
            $_.Exception
        )
    }
}

function Invoke-AmeR2cRWorkerReportGuardrail {
    param([Parameter(Mandatory = $true)][string]$AnchorPath)

    $tamperCase = @(Get-AmeR2cRCaseDefinitions |
        Where-Object { $_.Label -ceq "worker-report-binding-tamper" })
    if ($tamperCase.Count -ne 1 -or [bool]$tamperCase[0].Ignored) {
        throw "R2c-R guardrail lost the executable worker-report tamper case"
    }
    $previousEnvironment = @{}
    foreach ($name in @("CARGO_BUILD_JOBS", "RUST_TEST_THREADS")) {
        $previousEnvironment[$name] = [Environment]::GetEnvironmentVariable($name, "Process")
    }
    $fixture = $null
    $toolLock = $null
    $cleanup = New-AmeR2cRFixtureCleanup
    $primaryFailure = $null
    try {
        $toolLock = Enter-AmeRepositoryToolLock
        $fixture = New-AmeR2cRGuardrailDisposableRoot -AnchorPath $AnchorPath `
            -Nonce ([Guid]::NewGuid().ToString("N"))
        foreach ($name in $previousEnvironment.Keys) {
            [Environment]::SetEnvironmentVariable($name, "1", "Process")
        }
        $cargo = (Get-AmeToolchain).Cargo
        $repositoryRoot = Get-AmeRepositoryRoot
        Write-Output "r2c_r_worker_report_build_started deadline_ms=900000"
        $build = Invoke-AmeR2cROwnedProcess -ExecutableKind "CargoTestBuild" `
            -FileName $cargo -Arguments @(Get-AmeR2cRTestBuildArguments) `
            -WorkingDirectory $repositoryRoot `
            -OutputPath (Join-Path $fixture.Path "tamper-build.log") `
            -TimeoutMilliseconds 900000
        Assert-AmeR2cRWorkerReportPhase -Phase "build" -Result $build
        Write-Output "r2c_r_worker_report_build_passed"
        Write-Output "r2c_r_worker_report_test_started deadline_ms=300000"
        $result = Invoke-AmeR2cROwnedProcess -ExecutableKind "CargoExactTest" `
            -FileName $cargo -Arguments @(Get-AmeR2cRCaseArguments -Case $tamperCase[0]) `
            -WorkingDirectory $repositoryRoot `
            -OutputPath (Join-Path $fixture.Path "tamper-behavior.log") `
            -TimeoutMilliseconds 300000
        Assert-AmeR2cRWorkerReportPhase -Phase "test" -Result $result
        Write-Output "r2c_r_worker_report_test_passed"
    } catch {
        $primaryFailure = Get-AmeR2cRFixturePrimaryFailure -ErrorRecord $_
    } finally {
        foreach ($name in $previousEnvironment.Keys) {
            try {
                $value = $previousEnvironment[$name]
                if ($null -eq $value) { $value = [System.Management.Automation.Language.NullString]::Value }
                [Environment]::SetEnvironmentVariable($name, $value, "Process")
                $restored = [Environment]::GetEnvironmentVariable($name, "Process")
                if (($null -eq $previousEnvironment[$name]) -ne ($null -eq $restored) -or
                    ($null -ne $restored -and $previousEnvironment[$name] -cne $restored)) {
                    throw "R2c-R worker-report guardrail did not restore $name"
                }
            } catch {
                $cleanup.Failures.Add([pscustomobject]@{ stage = "environment-$name"; exception = $_.Exception })
            }
        }
        try {
            Close-AmeR2cRGuardrailFixture -Cleanup $cleanup -Label "worker-report" -Fixture $fixture
        } catch {
            $cleanup.Failures.Add([pscustomobject]@{ stage = "worker-report-fixture"; exception = $_.Exception })
        }
        if ($null -ne $toolLock) {
            try { Exit-AmeRepositoryToolLock $toolLock } catch {
                $cleanup.Failures.Add([pscustomobject]@{ stage = "worker-report-lock"; exception = $_.Exception })
            }
        }
    }
    Complete-AmeR2cRFixtureCleanup -Cleanup $cleanup -PrimaryFailure $primaryFailure
}
