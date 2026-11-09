$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'quality_common.ps1')
$scripts = @(Get-ChildItem -LiteralPath $PSScriptRoot -Filter '*.ps1' -File | Select-Object -ExpandProperty FullName)
Invoke-AmePowerShellSyntaxCheck $scripts
foreach ($check in @(
    'quality_test_ci_policy', 'release_test_version_validation',
    'release_test_installer_payload', 'release_test_classification_runtime',
    'release_test_classification_models', 'release_test_classification_vocabulary',
    'release_test_portable_archive', 'release_test_windows_publication',
    'release_test_journal_broker_installer_guardrails'
)) {
    & (Join-Path $PSScriptRoot "$check.ps1")
}
