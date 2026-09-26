[CmdletBinding()]
param(
    [AllowEmptyString()]
    [string]$SourceRoot,
    [AllowEmptyString()]
    [string]$LocalRoot,
    [AllowEmptyString()]
    [string]$CloudRoot,
    [AllowEmptyString()]
    [string]$SourceCatalogPath,
    [switch]$ValidationOnly,
    [switch]$GuardrailWorkspaceAnchor
)

$ErrorActionPreference = "Stop"
. (Join-Path $PSScriptRoot "quality_common.ps1")
. (Join-Path $PSScriptRoot "acceptance_r2c_change_driven_reliability_common.ps1")

$forbiddenParameters = @("SourceRoot", "LocalRoot", "CloudRoot", "SourceCatalogPath")
foreach ($name in $forbiddenParameters) {
    if ($PSBoundParameters.ContainsKey($name)) {
        [Console]::Out.WriteLine(
            "AME_R2C_R_REJECTION reason=caller-supplied-source"
        )
        [Console]::Out.Flush()
        throw (
            "R2c-R local reliability does not accept caller-supplied source roots, " +
            "catalogs, or external path aliases"
        )
    }
}

$forbiddenEnvironmentNames = @(Get-AmeR2cRForbiddenEnvironmentNames)
$forbiddenEnvironment = @(
    foreach ($name in $forbiddenEnvironmentNames) {
        if ($null -ne [Environment]::GetEnvironmentVariable($name, "Process")) {
            $name
        }
    }
)
if ($forbiddenEnvironment.Count -gt 0) {
    [Console]::Out.WriteLine(
        "AME_R2C_R_REJECTION reason=caller-supplied-environment"
    )
    [Console]::Out.Flush()
    throw (
        "R2c-R local reliability refuses caller-supplied environment aliases: " +
        ($forbiddenEnvironment -join ", ")
    )
}
if ($GuardrailWorkspaceAnchor -and -not $ValidationOnly) {
    [Console]::Out.WriteLine(
        "AME_R2C_R_REJECTION reason=workspace-requires-validation-only"
    )
    [Console]::Out.Flush()
    throw "R2c-R workspace guardrail storage is available only to ValidationOnly"
}
$repositoryRoot = Get-AmeRepositoryRoot
Initialize-AmeR2cRNativeTypes -RepositoryRoot $repositoryRoot
$version = Get-AmeR2cRWindowsVersionEvidence
$isAdministrator = Test-AmeR2cRCurrentProcessIsAdministrator
try {
    Assert-AmeR2cRExecutionContext `
        -IsWindowsPlatform ([bool]$version.IsWindows) `
        -OperatingSystemArchitecture (
            [Runtime.InteropServices.RuntimeInformation]::OSArchitecture
        ) `
        -ProcessArchitecture (
            [Runtime.InteropServices.RuntimeInformation]::ProcessArchitecture
        ) `
        -BuildNumber ([int]$version.BuildNumber) `
        -ApiBuildNumber ([int]$version.ApiBuildNumber) `
        -InstallationType ([string]$version.InstallationType) `
        -ProductType ([string]$version.ProductType) `
        -ProductSku ([uint32]$version.ProductSku) `
        -IsAdministrator $isAdministrator
} catch {
    [Console]::Out.WriteLine(
        "AME_R2C_R_REJECTION reason=execution-context"
    )
    [Console]::Out.Flush()
    throw
}

Assert-AmeR2cRCaseMatrix -RepositoryRoot $repositoryRoot
$cargo = (Get-AmeToolchain).Cargo
$testCases = @(Get-AmeR2cRCaseDefinitions)
$runNonce = [Guid]::NewGuid().ToString("N")
$fixture = $null
$toolLock = $null
$locationPushed = $false
$resultLine = $null
$runnerStarted = [Diagnostics.Stopwatch]::StartNew()
$totalTimeout = [TimeSpan]::FromHours(1)
$caseTimeout = [TimeSpan]::FromMinutes(10)
$environmentNames = @(
    $forbiddenEnvironmentNames +
    @("CARGO_BUILD_JOBS", "RUST_TEST_THREADS", "TEMP", "TMP") |
        Sort-Object -Unique
)
$previousEnvironment = @{}
foreach ($name in $environmentNames) {
    $previousEnvironment[$name] = [Environment]::GetEnvironmentVariable($name, "Process")
}

try {
    $fixture = if ($GuardrailWorkspaceAnchor) {
        New-AmeR2cRGuardrailDisposableRoot `
            -AnchorPath (Join-Path $repositoryRoot "tool") `
            -Nonce $runNonce
    } else {
        New-AmeR2cRDisposableRoot -Nonce $runNonce
    }
    [Environment]::SetEnvironmentVariable(
        "CEDARFLAKE_AME_R2C_R_OWNED_ROOT",
        [string]$fixture.Path,
        "Process"
    )
    [Environment]::SetEnvironmentVariable(
        "CEDARFLAKE_AME_R2C_R_RUN_NONCE",
        $runNonce,
        "Process"
    )
    [Environment]::SetEnvironmentVariable(
        "CEDARFLAKE_AME_R2C_R_RUNNER_PID",
        $PID.ToString([Globalization.CultureInfo]::InvariantCulture),
        "Process"
    )
    [Environment]::SetEnvironmentVariable("TEMP", [string]$fixture.Path, "Process")
    [Environment]::SetEnvironmentVariable("TMP", [string]$fixture.Path, "Process")
    [Environment]::SetEnvironmentVariable("CARGO_BUILD_JOBS", "1", "Process")
    [Environment]::SetEnvironmentVariable("RUST_TEST_THREADS", "1", "Process")

    $fixture.ValidateHeldIdentity()

    if ($ValidationOnly) {
        $storageMode = if ($GuardrailWorkspaceAnchor) {
            "workspace-guardrail"
        } else { "known-folder" }
        $resultLine = (
            "AME_R2C_R_VALIDATION status=passed cases=19 normal=15 ignored=4 " +
            "selectors=exact roots=internal-disposable root_physical=held-handle " +
            "external_paths=refused reparse=refused volume=fixed-ntfs " +
            "platform=windows11-x64 build=$($version.BuildNumber) " +
            "display_version=$($version.DisplayVersion) product_sku=$($version.ProductSku) " +
            "privilege=ordinary-user storage_mode=$storageMode " +
            "nonce=$runNonce runner_pid=$PID phase=validation"
        )
    } else {
        $toolLock = Enter-AmeRepositoryToolLock
        Push-Location $repositoryRoot
        $locationPushed = $true
        for ($caseIndex = 0; $caseIndex -lt $testCases.Count; $caseIndex++) {
            $testCase = $testCases[$caseIndex]
            $remaining = $totalTimeout - $runnerStarted.Elapsed
            if ($remaining -le [TimeSpan]::Zero) {
                throw "R2c-R public runner exceeded its one-hour wall-clock deadline"
            }
            $allowed = if ($remaining -lt $caseTimeout) { $remaining } else { $caseTimeout }
            $allowedMilliseconds = [int][Math]::Max(1, [Math]::Floor($allowed.TotalMilliseconds))
            $outputPath = Join-Path (
                [string]$fixture.Path
            ) ("case-{0:D2}-{1}.log" -f ($caseIndex + 1), $testCase.Label)
            Write-Output (
                "AME_R2C_R_CASE label=$($testCase.Label) status=started " +
                "nonce=$runNonce runner_pid=$PID phase=cargo"
            )
            $caseResult = Invoke-AmeR2cROwnedProcess `
                -ExecutableKind "CargoExactTest" `
                -FileName $cargo `
                -Arguments @(Get-AmeR2cRCaseArguments -Case $testCase) `
                -WorkingDirectory $repositoryRoot `
                -OutputPath $outputPath `
                -TimeoutMilliseconds $allowedMilliseconds
            foreach ($line in $caseResult.Lines) {
                Write-Output $line
            }
            if ($caseResult.TimedOut) {
                throw (
                    "R2c-R case '$($testCase.Label)' exceeded its parent wall-clock " +
                    "deadline; owned process tree $($caseResult.ProcessId) was terminated"
                )
            }
            Assert-AmeR2cRCaseResult `
                -Label $testCase.Label `
                -Lines $caseResult.Lines `
                -ExitCode $caseResult.ExitCode
            Write-Output (
                "AME_R2C_R_CASE label=$($testCase.Label) status=passed " +
                "nonce=$runNonce runner_pid=$PID child_pid=$($caseResult.ProcessId) phase=harvested"
            )
        }
        $storageMode = if ($GuardrailWorkspaceAnchor) {
            "workspace-guardrail"
        } else { "known-folder" }
        $resultLine = (
            "AME_R2C_R_REPORT status=passed cases=19 platform=windows11-x64 " +
            "build=$($version.BuildNumber) display_version=$($version.DisplayVersion) " +
            "installation_type=$($version.InstallationType) product_type=$($version.ProductType) " +
            "product_sku=$($version.ProductSku) privilege=ordinary-user " +
            "roots=internal-disposable root_physical=held-handle source_mutation=fixture-only " +
            "real_fsctl=not-used timeout=process-tree storage_mode=$storageMode " +
            "nonce=$runNonce runner_pid=$PID phase=complete"
        )
    }
} finally {
    if ($locationPushed) {
        Pop-Location
    }
    if ($null -ne $toolLock) {
        Exit-AmeRepositoryToolLock $toolLock
    }
    foreach ($name in $environmentNames) {
        if ($null -eq $previousEnvironment[$name]) {
            [Environment]::SetEnvironmentVariable(
                $name,
                [Management.Automation.Language.NullString]::Value,
                [EnvironmentVariableTarget]::Process
            )
        } else {
            [Environment]::SetEnvironmentVariable(
                $name,
                $previousEnvironment[$name],
                [EnvironmentVariableTarget]::Process
            )
        }
    }
    if ($null -ne $fixture) {
        Remove-AmeR2cRDisposableRoot -Fixture $fixture
    }
}
foreach ($name in $environmentNames) {
    $restoredEnvironment = [Environment]::GetEnvironmentVariable($name, "Process")
    if (($null -eq $previousEnvironment[$name]) -ne ($null -eq $restoredEnvironment) -or
        ($null -ne $previousEnvironment[$name] -and
            $previousEnvironment[$name] -cne $restoredEnvironment)) {
        throw "R2c-R runner did not restore process environment value $name"
    }
}

Write-Output $resultLine
