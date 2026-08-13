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

$workflowPath = Join-Path $repositoryRoot ".github\workflows\quality_gate_windows.yml"
$workflow = Get-Content -LiteralPath $workflowPath -Raw -Encoding UTF8
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
