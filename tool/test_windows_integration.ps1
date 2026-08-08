$ErrorActionPreference = "Stop"
. (Join-Path $PSScriptRoot "quality_common.ps1")

$repositoryRoot = Get-AmeRepositoryRoot
$toolchain = Get-AmeToolchain
$buildRoot = Join-Path $repositoryRoot "build"
$storageRoot = Join-Path $buildRoot "integration-storage-$PID"
$runnerPath = Join-Path $buildRoot "windows\x64\runner\Debug\cedarflake_ame.exe"
$resolvedBuildRoot = [System.IO.Path]::GetFullPath($buildRoot)
$resolvedStorageRoot = [System.IO.Path]::GetFullPath($storageRoot)
$resolvedRunnerPath = [System.IO.Path]::GetFullPath($runnerPath)
$testStartedAt = Get-Date

if (-not $resolvedStorageRoot.StartsWith(
    "$resolvedBuildRoot$([System.IO.Path]::DirectorySeparatorChar)",
    [System.StringComparison]::OrdinalIgnoreCase
)) {
    throw "Integration storage must remain inside the repository build directory"
}

New-Item -ItemType Directory -Path $resolvedStorageRoot -Force | Out-Null
$previousStorageRoot = $env:CEDARFLAKE_AME_TEST_STORAGE_ROOT
$env:CEDARFLAKE_AME_TEST_STORAGE_ROOT = $resolvedStorageRoot

Push-Location $repositoryRoot
try {
    Invoke-AmeChecked $toolchain.Flutter @(
        "test",
        "integration_test\scan_workflow_test.dart",
        "-d",
        "windows"
    )
} finally {
    Get-Process -Name "cedarflake_ame" -ErrorAction SilentlyContinue |
        Where-Object {
            $_.Path -and
            $_.Path.Equals(
                $resolvedRunnerPath,
                [System.StringComparison]::OrdinalIgnoreCase
            ) -and
            $_.StartTime -ge $testStartedAt
        } |
        Stop-Process -Force
    Pop-Location
    $env:CEDARFLAKE_AME_TEST_STORAGE_ROOT = $previousStorageRoot
    if (Test-Path -LiteralPath $resolvedStorageRoot) {
        Remove-Item -LiteralPath $resolvedStorageRoot -Recurse -Force
    }
}
