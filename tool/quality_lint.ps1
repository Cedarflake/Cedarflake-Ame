$ErrorActionPreference = "Stop"
. (Join-Path $PSScriptRoot "quality_common.ps1")

$repositoryRoot = Get-AmeRepositoryRoot
$toolchain = Get-AmeToolchain
$toolLock = Enter-AmeRepositoryToolLock

Push-Location $repositoryRoot
try {
    $powerShellFiles = @(Get-ChildItem -LiteralPath "tool" -Filter "*.ps1" -File)
    Assert-AmeToolScriptNames (
        $powerShellFiles | Select-Object -ExpandProperty Name
    )
    $workflowFiles = @(
        Get-ChildItem -LiteralPath ".github\workflows" -File |
            Where-Object { $_.Extension -in @(".yml", ".yaml") }
    )
    Assert-AmeWorkflowNames (
        $workflowFiles | Select-Object -ExpandProperty Name
    )
    $powerShellPaths = $powerShellFiles |
        Select-Object -ExpandProperty FullName
    Invoke-AmePowerShellSyntaxCheck $powerShellPaths
    & (Join-Path $PSScriptRoot "quality_test_naming_contract.ps1")
    & (Join-Path $PSScriptRoot "quality_test_hosted_parallel_contract.ps1")
    & (Join-Path $PSScriptRoot "integration_test_windows_accessibility_guardrails.ps1")
    & (Join-Path $PSScriptRoot "release_test_version_validation.ps1")
    & (Join-Path $PSScriptRoot "release_test_portable_archive.ps1")
    Invoke-AmeJsonSyntaxCheck @(
        (Join-Path $repositoryRoot ".vscode\extensions.json"),
        (Join-Path $repositoryRoot ".vscode\settings.json")
    )
    & (Join-Path $PSScriptRoot "quality_format.ps1") -Check
    Invoke-AmeChecked $toolchain.Cargo @(
        "clippy",
        "--locked",
        "--manifest-path",
        "rust\Cargo.toml",
        "--all-targets",
        "--all-features",
        "--",
        "-D",
        "warnings"
    )
    Invoke-AmeChecked $toolchain.Dart @(
        "analyze",
        "--fatal-infos",
        "--fatal-warnings"
    )
} finally {
    Pop-Location
    Exit-AmeRepositoryToolLock $toolLock
}
