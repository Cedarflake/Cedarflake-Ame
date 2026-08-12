$ErrorActionPreference = "Stop"
. (Join-Path $PSScriptRoot "quality_common.ps1")

$repositoryRoot = Get-AmeRepositoryRoot
$buildRoot = [System.IO.Path]::GetFullPath((Join-Path $repositoryRoot "build"))
$scratchRoot = [System.IO.Path]::GetFullPath(
    (Join-Path $buildRoot "windows-accessibility-guardrails-$PID")
)
$buildPrefix = "$buildRoot$([System.IO.Path]::DirectorySeparatorChar)"
if (-not $scratchRoot.StartsWith(
    $buildPrefix,
    [System.StringComparison]::OrdinalIgnoreCase
)) {
    throw "Accessibility guardrail storage must remain inside build"
}

$canary = Join-Path $PSScriptRoot "integration_test_windows_accessibility.ps1"
$cleanOutput = Join-Path $scratchRoot "clean.log"
$invalidOutput = Join-Path $scratchRoot "invalid.log"
$canaryOutput = Join-Path $scratchRoot "canary.log"
$runnerPath = Join-Path $repositoryRoot "build\windows\x64\runner\Debug\cedarflake_ame.exe"
$utf8 = [System.Text.UTF8Encoding]::new($false)

try {
    New-Item -ItemType Directory -Path $scratchRoot -Force | Out-Null
    $runnerProcessIdsBefore = @(
        Get-Process -Name "cedarflake_ame" -ErrorAction SilentlyContinue |
            Where-Object {
                $_.Path -and
                $_.Path.Equals(
                    $runnerPath,
                    [System.StringComparison]::OrdinalIgnoreCase
                )
            } |
            Select-Object -ExpandProperty Id
    )
    [System.IO.File]::WriteAllText($cleanOutput, "All tests passed.`n", $utf8)
    & $canary -CapturedOutputPath $cleanOutput -OutputPath $canaryOutput
    $runnerProcessIdsAfter = @(
        Get-Process -Name "cedarflake_ame" -ErrorAction SilentlyContinue |
            Where-Object {
                $_.Path -and
                $_.Path.Equals(
                    $runnerPath,
                    [System.StringComparison]::OrdinalIgnoreCase
                )
            } |
            Select-Object -ExpandProperty Id
    )
    if (Compare-Object $runnerProcessIdsBefore $runnerProcessIdsAfter) {
        throw "Captured-output validation changed runner process ownership"
    }

    [System.IO.File]::WriteAllText(
        $invalidOutput,
        "[ERROR] Failed to update ui::AXTree, error: node error`n",
        $utf8
    )
    $invalidOutputRejected = $false
    try {
        & $canary -CapturedOutputPath $invalidOutput -OutputPath $canaryOutput
    } catch {
        $invalidOutputRejected = $_.Exception.Message -match `
            "AccessibilityBridge rejected"
    }
    if (-not $invalidOutputRejected) {
        throw "Windows accessibility canary accepted an AXTree failure"
    }
} finally {
    if (Test-Path -LiteralPath $scratchRoot) {
        Remove-Item -LiteralPath $scratchRoot -Recurse -Force
    }
}
