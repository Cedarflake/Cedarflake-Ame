$ErrorActionPreference = "Stop"

$repositoryRoot = Split-Path -Parent $PSScriptRoot
$buildRoot = Join-Path $repositoryRoot "build"
$storageRoot = Join-Path $buildRoot "integration-storage-$PID"
$resolvedBuildRoot = [System.IO.Path]::GetFullPath($buildRoot)
$resolvedStorageRoot = [System.IO.Path]::GetFullPath($storageRoot)

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
    & flutter test integration_test\scan_workflow_test.dart -d windows
    if ($LASTEXITCODE -ne 0) {
        throw "Windows integration test failed with exit code $LASTEXITCODE"
    }
} finally {
    Pop-Location
    $env:CEDARFLAKE_AME_TEST_STORAGE_ROOT = $previousStorageRoot
    if (Test-Path -LiteralPath $resolvedStorageRoot) {
        Remove-Item -LiteralPath $resolvedStorageRoot -Recurse -Force
    }
}
