param(
    [Parameter(Mandatory = $true)]
    [string]$ActionlintPath
)

$ErrorActionPreference = "Stop"
. (Join-Path $PSScriptRoot "quality_common.ps1")

$repositoryRoot = Get-AmeRepositoryRoot
if (-not (Test-Path -LiteralPath $ActionlintPath -PathType Leaf)) {
    throw "actionlint executable was not found at '$ActionlintPath'"
}
$resolvedActionlintPath = (Resolve-Path -LiteralPath $ActionlintPath).Path

Push-Location $repositoryRoot
try {
    $workflowFiles = @(
        Get-ChildItem -LiteralPath ".github\workflows" -File |
            Where-Object { $_.Extension -in @(".yml", ".yaml") }
    )
    Assert-AmeWorkflowNames (
        $workflowFiles | Select-Object -ExpandProperty Name
    )
    Invoke-AmeChecked $resolvedActionlintPath (
        $workflowFiles | Select-Object -ExpandProperty FullName
    )
} finally {
    Pop-Location
}
