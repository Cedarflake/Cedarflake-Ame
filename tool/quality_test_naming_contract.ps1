$ErrorActionPreference = "Stop"
. (Join-Path $PSScriptRoot "quality_common.ps1")

Assert-AmeToolScriptNames @(
    "quality_verify_daily.ps1",
    "release_verify_candidate.ps1"
)
Assert-AmeWorkflowNames @(
    "quality_ci.yml",
    "release_candidate_windows.yaml"
)

$invalidScriptRejected = $false
try {
    Assert-AmeToolScriptNames @("verify.ps1")
} catch {
    $invalidScriptRejected = $true
}
if (-not $invalidScriptRejected) {
    throw "Tool naming validation accepted a script without an ownership prefix"
}

$invalidWorkflowRejected = $false
try {
    Assert-AmeWorkflowNames @("ci.yml")
} catch {
    $invalidWorkflowRejected = $true
}
if (-not $invalidWorkflowRejected) {
    throw "Workflow naming validation accepted a workflow without an ownership prefix"
}
