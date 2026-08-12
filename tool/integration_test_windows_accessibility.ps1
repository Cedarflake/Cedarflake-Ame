[CmdletBinding()]
param(
    [string]$CapturedOutputPath,
    [string]$OutputPath
)

$ErrorActionPreference = "Stop"
. (Join-Path $PSScriptRoot "quality_common.ps1")

$repositoryRoot = Get-AmeRepositoryRoot
$toolchain = Get-AmeToolchain
$toolLock = Enter-AmeRepositoryToolLock
if (-not $OutputPath) {
    $OutputPath = Join-Path `
        $repositoryRoot `
        "build\windows-accessibility-bridge-test.log"
}
$runnerPath = Join-Path $repositoryRoot "build\windows\x64\runner\Debug\cedarflake_ame.exe"
$resolvedRunnerPath = [System.IO.Path]::GetFullPath($runnerPath)
$arguments = @(
    "test"
    "integration_test\windows_accessibility_bridge_test.dart"
    "-d"
    "windows"
    "--no-pub"
)

Push-Location $repositoryRoot
$didLaunchRunner = $false
try {
    $runnerProcessIdsBefore = @(
        Get-Process -Name "cedarflake_ame" -ErrorAction SilentlyContinue |
            Where-Object {
                $_.Path -and
                $_.Path.Equals(
                    $resolvedRunnerPath,
                    [System.StringComparison]::OrdinalIgnoreCase
                )
            } |
            Select-Object -ExpandProperty Id
    )
    $testStartedAt = Get-Date
    if ($CapturedOutputPath) {
        $capturedOutput = Get-Content `
            -LiteralPath $CapturedOutputPath `
            -Raw `
            -Encoding UTF8
        $exitCode = 0
    } else {
        $didLaunchRunner = $true
        $outerErrorActionPreference = $ErrorActionPreference
        try {
            $ErrorActionPreference = "Continue"
            $capturedLines = @(
                & $toolchain.Flutter @arguments 2>&1 | ForEach-Object {
                    $line = $_.ToString()
                    Write-Host $line
                    $line
                }
            )
            $exitCode = $LASTEXITCODE
        } finally {
            $ErrorActionPreference = $outerErrorActionPreference
        }
        $capturedOutput = $capturedLines -join [Environment]::NewLine
    }
    $outputDirectory = Split-Path -Parent $OutputPath
    New-Item -ItemType Directory -Path $outputDirectory -Force | Out-Null
    Set-Content -LiteralPath $OutputPath -Value $capturedOutput -Encoding UTF8

    if ($exitCode -ne 0) {
        throw "Windows accessibility integration failed with exit code $exitCode"
    }
    if ($capturedOutput -match "Failed to update ui::AXTree") {
        throw "Windows AccessibilityBridge rejected a semantics update"
    }
} finally {
    if ($didLaunchRunner) {
        Get-Process -Name "cedarflake_ame" -ErrorAction SilentlyContinue |
            Where-Object {
                $_.Path -and
                $_.Path.Equals(
                    $resolvedRunnerPath,
                    [System.StringComparison]::OrdinalIgnoreCase
                ) -and
                $_.StartTime -ge $testStartedAt -and
                $runnerProcessIdsBefore -notcontains $_.Id
            } |
            Stop-Process -Force
    }
    Pop-Location
    Exit-AmeRepositoryToolLock $toolLock
}
