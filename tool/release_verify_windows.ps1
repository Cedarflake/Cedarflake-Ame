$ErrorActionPreference = "Stop"
. (Join-Path $PSScriptRoot "quality_common.ps1")

$repositoryRoot = Get-AmeRepositoryRoot
$toolchain = Get-AmeToolchain
$buildRoot = Join-Path $repositoryRoot "build"
$releaseRoot = Join-Path $buildRoot "windows\x64\runner\Release"
$releaseExecutable = Join-Path $releaseRoot "cedarflake_ame.exe"
$releaseLibrary = Join-Path $releaseRoot "rust_lib_cedarflake_ame.dll"
$scratchRoot = Join-Path $buildRoot "release-bridge-smoke-$PID"
$resolvedBuildRoot = [System.IO.Path]::GetFullPath($buildRoot)
$resolvedScratchRoot = [System.IO.Path]::GetFullPath($scratchRoot)
$releaseLibraryAlias = Join-Path (
    $resolvedScratchRoot
) "rust_lib_cedarflake_ame_release_smoke.dll"
$flutterExecutable = $toolchain.Flutter

if (-not $resolvedScratchRoot.StartsWith(
    "$resolvedBuildRoot$([System.IO.Path]::DirectorySeparatorChar)",
    [System.StringComparison]::OrdinalIgnoreCase
)) {
    throw "Release bridge scratch directory must remain inside the repository build directory"
}

$previousLibraryPath = $env:CEDARFLAKE_AME_TEST_LIBRARY_PATH
$releaseProcess = $null
$toolLock = Enter-AmeRepositoryToolLock

Push-Location $repositoryRoot
try {
    & $flutterExecutable build windows --release
    if ($LASTEXITCODE -ne 0) {
        throw "Windows release build failed with exit code $LASTEXITCODE"
    }

    if (-not (Test-Path -LiteralPath $releaseLibrary -PathType Leaf)) {
        throw "Windows release Rust library was not packaged"
    }

    $latestRustSource = Get-ChildItem -LiteralPath "rust\src" -Recurse -Filter "*.rs" -File |
        Sort-Object LastWriteTimeUtc -Descending |
        Select-Object -First 1
    $packagedLibrary = Get-Item -LiteralPath $releaseLibrary
    if ($packagedLibrary.LastWriteTimeUtc -lt $latestRustSource.LastWriteTimeUtc) {
        throw "Packaged Rust library is older than the current Rust bridge sources"
    }

    $releaseStartInfo = [System.Diagnostics.ProcessStartInfo]::new()
    $releaseStartInfo.FileName = $releaseExecutable
    $releaseStartInfo.WorkingDirectory = $repositoryRoot
    $releaseStartInfo.UseShellExecute = $false
    $releaseStartInfo.CreateNoWindow = $true
    $releaseStartInfo.WindowStyle = [System.Diagnostics.ProcessWindowStyle]::Hidden
    $releaseProcess = [System.Diagnostics.Process]::Start($releaseStartInfo)
    if ($null -eq $releaseProcess) {
        throw "Windows release application process was not created"
    }
    $loadedRustLibrary = $null
    for ($attempt = 0; $attempt -lt 40; $attempt++) {
        Start-Sleep -Milliseconds 250
        $releaseProcess.Refresh()
        if ($releaseProcess.HasExited) {
            throw "Windows release application exited during startup"
        }
        $loadedRustLibrary = $releaseProcess.Modules |
            Where-Object { $_.ModuleName -eq "rust_lib_cedarflake_ame.dll" } |
            Select-Object -First 1
        if ($null -ne $loadedRustLibrary) {
            break
        }
    }
    if ($null -eq $loadedRustLibrary) {
        throw "Windows release application did not load the Rust library"
    }
    $resolvedLoadedLibrary = [System.IO.Path]::GetFullPath($loadedRustLibrary.FileName)
    $resolvedReleaseLibrary = [System.IO.Path]::GetFullPath($releaseLibrary)
    if (-not $resolvedLoadedLibrary.Equals(
        $resolvedReleaseLibrary,
        [System.StringComparison]::OrdinalIgnoreCase
    )) {
        throw "Windows release application loaded Rust from $resolvedLoadedLibrary"
    }
    Stop-Process -Id $releaseProcess.Id -Force
    for ($attempt = 0; $attempt -lt 40; $attempt++) {
        if ($null -eq (Get-Process -Id $releaseProcess.Id -ErrorAction SilentlyContinue)) {
            break
        }
        Start-Sleep -Milliseconds 250
    }
    if ($null -ne (Get-Process -Id $releaseProcess.Id -ErrorAction SilentlyContinue)) {
        throw "Windows release verification process did not exit"
    }
    $releaseProcess = $null

    New-Item -ItemType Directory -Path $resolvedScratchRoot -Force | Out-Null
    Copy-Item -LiteralPath $releaseLibrary -Destination $releaseLibraryAlias -Force
    $env:CEDARFLAKE_AME_TEST_LIBRARY_PATH = $releaseLibraryAlias

    & $flutterExecutable test integration_test\release_bridge_smoke_test.dart -d windows
    if ($LASTEXITCODE -ne 0) {
        throw "Windows release bridge smoke test failed with exit code $LASTEXITCODE"
    }
} finally {
    Pop-Location
    $env:CEDARFLAKE_AME_TEST_LIBRARY_PATH = $previousLibraryPath
    if ($null -ne $releaseProcess -and -not $releaseProcess.HasExited) {
        Stop-Process -Id $releaseProcess.Id -Force -ErrorAction SilentlyContinue
    }
    if (Test-Path -LiteralPath $resolvedScratchRoot) {
        Remove-Item -LiteralPath $resolvedScratchRoot -Recurse -Force
    }
    Exit-AmeRepositoryToolLock $toolLock
}
