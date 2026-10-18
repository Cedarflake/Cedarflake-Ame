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
    "rust_catalog",
    "rust_catalog_history",
    "rust_other",
    "rust_sync_scan",
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

$rustScriptPath = Join-Path $PSScriptRoot "quality_test_rust.ps1"
$rustTokens = $null
$rustErrors = $null
$rustAst = [System.Management.Automation.Language.Parser]::ParseFile(
    $rustScriptPath, [ref]$rustTokens, [ref]$rustErrors
)
$cargoArguments = @($rustAst.FindAll({
    param($node)
    $node -is [System.Management.Automation.Language.AssignmentStatementAst] -and
        $node.Left.Extent.Text -ceq '$cargoArguments'
}, $true))
if ($rustErrors.Count -ne 0 -or $cargoArguments.Count -ne 1) {
    throw "Rust test owner must retain one complete Cargo target and feature selection"
}
$argumentArrays = @($cargoArguments[0].Right.FindAll({
    param($node)
    $node -is [System.Management.Automation.Language.ArrayExpressionAst]
}, $true))
if ($argumentArrays.Count -ne 1 -or
    $argumentArrays[0].Extent.Text -cne $cargoArguments[0].Right.Extent.Text) {
    throw "Rust target and feature selection must be one literal argument array"
}
$rustArguments = @($argumentArrays[0].SafeGetValue())
$expectedRustArguments = @(
    "test", "--locked", "--manifest-path", 'rust\Cargo.toml',
    "--all-targets", "--all-features", "--"
)
if (
    $rustArguments.Count -ne $expectedRustArguments.Count -or
    ($rustArguments -join "`n") -cne ($expectedRustArguments -join "`n")
) {
    throw "Rust tests must retain every target and feature in both complete and partitioned runs"
}

$workflowPath = Join-Path $repositoryRoot ".github\workflows\quality_gate_daily_windows.yml"
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
$hostedComponents = @([regex]::Matches(
    $workflow, '(?m)^          - component: ([a-z_]+)\r?$'
) | ForEach-Object { $_.Groups[1].Value } | Sort-Object)
$expectedHostedComponents = @($expectedComponents | Where-Object { $_ -ne "all" })
if (($hostedComponents -join ",") -cne ($expectedHostedComponents -join ",")) {
    throw "Hosted daily components must each run exactly once on a separate runner"
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
$gate = Get-Content -LiteralPath (Join-Path $repositoryRoot `
    ".github\workflows\quality_gate_windows.yml") -Raw -Encoding UTF8
if ($gate -notmatch "quality_windows:\s*\r?\n\s+name:\s*Windows Gate") {
    throw "Hosted workflow must retain the stable Windows Gate aggregation check"
}
foreach ($required in @(
    "    uses: ./.github/workflows/quality_gate_daily_windows.yml",
    '      checkout_ref: ${{ inputs.checkout_ref || github.sha }}',
    '      base_revision: ${{ inputs.base_revision }}'
)) {
    if (-not $gate.Contains($required)) {
        throw "Hosted daily owner lost source or revision binding: $required"
    }
}

foreach ($entrypoint in @($dailyScriptPath, $rustScriptPath)) {
    $tokens = $null
    $errors = $null
    $ast = [System.Management.Automation.Language.Parser]::ParseFile(
        $entrypoint, [ref]$tokens, [ref]$errors
    )
    if ($errors.Count -ne 0) { throw "Cannot parse serial entrypoint: $entrypoint" }
    $selection = @($ast.ParamBlock.Parameters | Where-Object {
        $_.Name.VariablePath.UserPath -in @("Component", "Partition")
    })
    if ($selection.Count -ne 1 -or $selection[0].DefaultValue.SafeGetValue() -cne "all") {
        throw "Local quality entrypoints must default to the complete serial suite: $entrypoint"
    }
    $commands = @($ast.FindAll({
        param($node)
        $node -is [System.Management.Automation.Language.CommandAst]
    }, $true))
    foreach ($command in $commands) {
        if ($command.GetCommandName() -in @(
            "Start-Job", "Start-ThreadJob", "Start-Process", "Invoke-Command"
        ) -or $command.CommandElements.Extent.Text -contains "-Parallel") {
            throw "Local quality entrypoints must not dispatch concurrent work: $entrypoint"
        }
    }
    $lockEntries = @($commands | Where-Object {
        $_.GetCommandName() -eq "Enter-AmeRepositoryToolLock"
    })
    $lockExits = @($commands | Where-Object {
        $_.GetCommandName() -eq "Exit-AmeRepositoryToolLock"
    })
    $finallyBlocks = @($ast.FindAll({
        param($node)
        $node -is [System.Management.Automation.Language.TryStatementAst] -and
            $node.Finally.Extent.Text.Contains("Exit-AmeRepositoryToolLock")
    }, $true))
    if ($lockEntries.Count -ne 1 -or $lockExits.Count -ne 1 -or $finallyBlocks.Count -ne 1) {
        throw "Local quality entrypoints must retain lock acquisition and final release: $entrypoint"
    }
}

Write-Output "Hosted coverage and aggregation wiring verified; local entrypoints retain serial locks"
