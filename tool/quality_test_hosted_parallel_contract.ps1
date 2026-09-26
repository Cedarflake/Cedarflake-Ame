$ErrorActionPreference = "Stop"
. (Join-Path $PSScriptRoot "quality_common.ps1")

$repositoryRoot = Get-AmeRepositoryRoot
$dailyScriptPath = Join-Path $PSScriptRoot "quality_verify_daily.ps1"
$dailyCommand = Get-Command -Name $dailyScriptPath
$validateSet = @(
    $dailyCommand.Parameters["Component"].Attributes |
        Where-Object { $_ -is [System.Management.Automation.ValidateSetAttribute] } |
        Select-Object -ExpandProperty ValidValues
)
$expectedComponents = @(
    "all",
    "flutter",
    "static",
    "windows_accessibility",
    "windows_scan"
)
$actualComponents = @($validateSet | Sort-Object)
if (
    $actualComponents.Count -ne $expectedComponents.Count -or
    (Compare-Object $actualComponents $expectedComponents)
) {
    throw "Daily component contract does not match the hosted lane set"
}

$invalidComponentRejected = $false
try {
    & $dailyScriptPath -Component "unsupported"
} catch [System.Management.Automation.ParameterBindingException] {
    $invalidComponentRejected = $true
}
if (-not $invalidComponentRejected) {
    throw "Daily gate accepted an unsupported hosted component"
}

$dailyTokens = $null
$dailyErrors = $null
$dailyAst = [System.Management.Automation.Language.Parser]::ParseFile(
    $dailyScriptPath, [ref]$dailyTokens, [ref]$dailyErrors
)
$cargoCalls = @($dailyAst.FindAll({
    param($node)
    $node -is [System.Management.Automation.Language.CommandAst] -and
        $node.GetCommandName() -ceq "Invoke-AmeChecked" -and
        $node.CommandElements.Count -eq 3 -and
        $node.CommandElements[1].Extent.Text -ceq '$toolchain.Cargo'
}, $true))
if ($dailyErrors.Count -ne 0 -or $cargoCalls.Count -ne 1) {
    throw "Daily must own exactly one complete Rust test invocation"
}
$rustArguments = @($cargoCalls[0].CommandElements[2].SafeGetValue())
$expectedRustArguments = @(
    "test", "--locked", "--manifest-path", 'rust\Cargo.toml',
    "--all-targets", "--all-features", "--", "--test-threads=1"
)
if (
    $rustArguments.Count -ne $expectedRustArguments.Count -or
    ($rustArguments -join "`n") -cne ($expectedRustArguments -join "`n")
) {
    throw "Daily Rust tests must remain complete and case-serial with unchanged fixture concurrency"
}

$workflowPath = Join-Path $repositoryRoot ".github\workflows\quality_gate_windows.yml"
$workflow = Get-Content -LiteralPath $workflowPath -Raw -Encoding UTF8
$compatibilityStep = [regex]::Match(
    $workflow,
    '(?ms)^[ ]{6}- name: Run Windows PowerShell platform compatibility probes\r?\n' +
        '.*?(?=^[ ]{6}- name:|\z)'
)
if (-not $compatibilityStep.Success -or
    $compatibilityStep.Value -notmatch '(?m)^[ ]{8}shell:\s*powershell\s*$' -or
    $compatibilityStep.Value.IndexOf(
        "./tool/quality_test_windows_powershell_compatibility.ps1",
        [StringComparison]::Ordinal
    ) -lt 0) {
    throw "Hosted Windows PowerShell must run the compiler-free platform probe"
}
if ($compatibilityStep.Value.IndexOf(
        "acceptance_test_r2c_change_driven_reliability_guardrails.ps1",
        [StringComparison]::Ordinal
    ) -ge 0) {
    throw "Hosted Windows PowerShell must not initialize the complete R2c-R compiler guardrail"
}
if ($compatibilityStep.Value.IndexOf(
        "./tool/acceptance_test_windows_journal_broker_guardrails.ps1",
        [StringComparison]::Ordinal
    ) -lt 0) {
    throw "Hosted Windows PowerShell must exercise the .NET Framework secure-pipe fallback"
}

$lintPath = Join-Path $PSScriptRoot "quality_lint.ps1"
$lintSource = Get-Content -LiteralPath $lintPath -Raw -Encoding UTF8
if ($lintSource.IndexOf(
        '"acceptance_test_r2c_change_driven_reliability_guardrails.ps1"',
        [StringComparison]::Ordinal
    ) -lt 0) {
    throw "PowerShell 7 Daily lint must retain the complete R2c-R guardrail"
}
foreach ($component in $expectedComponents | Where-Object { $_ -ne "all" }) {
    if ($workflow -notmatch "component:\s*$component(?:\r?\n|$)") {
        throw "Hosted workflow is missing the '$component' daily lane"
    }
}
if ($workflow -notmatch "fail-fast:\s*false") {
    throw "Hosted daily lanes must report all failures instead of stopping early"
}
if (
    $workflow -notmatch (
        'quality_verify_daily\.ps1\s+-Component\s+' +
        '\$env:AME_DAILY_COMPONENT'
    )
) {
    throw "Hosted workflow does not invoke isolated daily components"
}
if ($workflow -notmatch "quality_windows:\s*\r?\n\s+name:\s*Windows Gate") {
    throw "Hosted workflow must retain the stable Windows Gate aggregation check"
}
