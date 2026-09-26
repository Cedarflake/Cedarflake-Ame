[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [string]$ExpectedBrokerPublisher,
    [Parameter(Mandatory = $true)]
    [string]$SignedBrokerBinaryPath,
    [Parameter(Mandatory = $true)]
    [string]$SignedApplicationBundlePath
)

$ErrorActionPreference = "Stop"
. (Join-Path $PSScriptRoot "quality_common.ps1")
. (Join-Path $PSScriptRoot "release_journal_broker_common.ps1")

$repositoryRoot = Get-AmeRepositoryRoot
$toolchain = Get-AmeToolchain
$buildRoot = Join-Path $repositoryRoot "build"
$releaseRoot = [System.IO.Path]::GetFullPath($SignedApplicationBundlePath)
$releaseExecutable = Join-Path $releaseRoot "cedarflake_ame.exe"
$releaseLibrary = Join-Path $releaseRoot "rust_lib_cedarflake_ame.dll"
$releaseBroker = Join-Path $releaseRoot (Get-AmeBrokerServicePlan).BinaryName
$scratchRoot = Join-Path $buildRoot "release-bridge-smoke-$PID"
$resolvedBuildRoot = [System.IO.Path]::GetFullPath($buildRoot)
$resolvedScratchRoot = [System.IO.Path]::GetFullPath($scratchRoot)
$releaseLibraryAlias = Join-Path (
    $resolvedScratchRoot
) "rust_lib_cedarflake_ame_release_smoke.dll"
$rustPluginBuildRoot = Join-Path (
    $buildRoot
) "windows\x64\plugins\rust_lib_cedarflake_ame\cargokit_build\x86_64-pc-windows-msvc\release"
$builtRustLibrary = Join-Path $rustPluginBuildRoot "rust_lib_cedarflake_ame.dll"
$rustDependencyFile = Join-Path $rustPluginBuildRoot "rust_lib_cedarflake_ame.d"
$flutterExecutable = $toolchain.Flutter

if (-not $resolvedScratchRoot.StartsWith(
    "$resolvedBuildRoot$([System.IO.Path]::DirectorySeparatorChar)",
    [System.StringComparison]::OrdinalIgnoreCase
)) {
    throw "Release bridge scratch directory must remain inside the repository build directory"
}

$previousLibraryPath = $env:CEDARFLAKE_AME_TEST_LIBRARY_PATH
$releaseProcess = $null
$secondaryProcess = $null
$replacementProcess = $null
$toolLock = Enter-AmeRepositoryToolLock

Push-Location $repositoryRoot
try {
    if ([string]::IsNullOrWhiteSpace($ExpectedBrokerPublisher) -or
        [string]::IsNullOrWhiteSpace($SignedBrokerBinaryPath) -or
        [string]::IsNullOrWhiteSpace($SignedApplicationBundlePath)) {
        throw "Windows release verification requires signed application, broker, and publisher inputs"
    }
    $bundleFacts = Assert-AmeApplicationBundleSource `
        -BundlePath $releaseRoot `
        -ExpectedPublisher $ExpectedBrokerPublisher
    if (-not $bundleFacts.ClientBinaryPath.Equals(
        $releaseExecutable,
        [System.StringComparison]::OrdinalIgnoreCase
    )) {
        throw "Signed application bundle resolved an unexpected client binary"
    }

    $builtBroker = [System.IO.Path]::GetFullPath($SignedBrokerBinaryPath)
    if (-not (Test-Path -LiteralPath $builtBroker -PathType Leaf)) {
        throw "The signed Windows x64 journal broker was not found"
    }
    Assert-AmeBrokerReleaseBinaryName -Path $builtBroker
    Assert-AmeBrokerBinary `
        -Path $builtBroker `
        -ExpectedPublisher $ExpectedBrokerPublisher | Out-Null
    if (-not (Test-Path -LiteralPath $releaseBroker -PathType Leaf)) {
        throw "The signed application bundle does not contain the journal broker"
    }
    $builtBrokerHash = (Get-FileHash -LiteralPath $builtBroker -Algorithm SHA256).Hash
    $releaseBrokerHash = (Get-FileHash -LiteralPath $releaseBroker -Algorithm SHA256).Hash
    if ($builtBrokerHash -cne $releaseBrokerHash) {
        throw "Packaged journal broker does not match the admitted x64 release binary"
    }
    Assert-AmeBrokerBinary `
        -Path $releaseBroker `
        -ExpectedPublisher $ExpectedBrokerPublisher | Out-Null

    if (-not (Test-Path -LiteralPath $releaseLibrary -PathType Leaf)) {
        throw "Windows release Rust library was not packaged"
    }

    if (-not (Test-Path -LiteralPath $builtRustLibrary -PathType Leaf)) {
        throw "Cargokit did not produce the Windows x64 Release Rust library"
    }
    if (-not (Test-Path -LiteralPath $rustDependencyFile -PathType Leaf)) {
        throw "Cargokit did not produce Rust release dependency evidence"
    }

    $dependencyText = Get-Content -LiteralPath $rustDependencyFile -Raw
    $dependencySeparator = $dependencyText.IndexOf(
        ": ",
        [System.StringComparison]::Ordinal
    )
    if ($dependencySeparator -lt 0) {
        throw "Rust release dependency evidence is malformed"
    }
    $dependencyList = $dependencyText.Substring($dependencySeparator + 2) -replace `
        '\\\r?\n',
        ''
    $rustReleaseDependencies = @(
        [regex]::Matches($dependencyList, '(?:\\ |[^\s])+') | ForEach-Object {
            $dependencyPath = $_.Value.Replace('\ ', ' ')
            if (-not (Test-Path -LiteralPath $dependencyPath -PathType Leaf)) {
                throw "Rust release dependency is missing: $dependencyPath"
            }
            Get-Item -LiteralPath $dependencyPath
        }
    )
    if ($rustReleaseDependencies.Count -eq 0) {
        throw "Rust release dependency evidence is empty"
    }
    $latestRustReleaseDependency = $rustReleaseDependencies |
        Sort-Object LastWriteTimeUtc -Descending |
        Select-Object -First 1
    $builtRustLibraryItem = Get-Item -LiteralPath $builtRustLibrary
    if ($builtRustLibraryItem.LastWriteTimeUtc -lt $latestRustReleaseDependency.LastWriteTimeUtc) {
        throw "Cargokit Rust library is older than its current release dependencies"
    }
    $builtRustLibraryHash = (Get-FileHash -LiteralPath $builtRustLibrary -Algorithm SHA256).Hash
    $packagedRustLibraryHash = (Get-FileHash -LiteralPath $releaseLibrary -Algorithm SHA256).Hash
    if ($packagedRustLibraryHash -ne $builtRustLibraryHash) {
        throw "Packaged Rust library does not match the current Cargokit release build"
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

    $secondaryProcess = [System.Diagnostics.Process]::Start($releaseStartInfo)
    if ($null -eq $secondaryProcess) {
        throw "Second Windows release application process was not created"
    }
    if (-not $secondaryProcess.WaitForExit(5000)) {
        throw "Second Windows release application did not reject the duplicate launch"
    }
    if ($secondaryProcess.ExitCode -ne 0) {
        throw "Second Windows release application exited with code $($secondaryProcess.ExitCode)"
    }
    $releaseProcess.Refresh()
    if ($releaseProcess.HasExited) {
        throw "Primary Windows release application exited during duplicate-launch verification"
    }
    $secondaryProcess.Dispose()
    $secondaryProcess = $null

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

    $replacementProcess = [System.Diagnostics.Process]::Start($releaseStartInfo)
    if ($null -eq $replacementProcess) {
        throw "Replacement Windows release application process was not created"
    }
    $replacementRustLibrary = $null
    for ($attempt = 0; $attempt -lt 40; $attempt++) {
        Start-Sleep -Milliseconds 250
        $replacementProcess.Refresh()
        if ($replacementProcess.HasExited) {
            throw "Replacement Windows release application exited during startup"
        }
        $replacementRustLibrary = $replacementProcess.Modules |
            Where-Object { $_.ModuleName -eq "rust_lib_cedarflake_ame.dll" } |
            Select-Object -First 1
        if ($null -ne $replacementRustLibrary) {
            break
        }
    }
    if ($null -eq $replacementRustLibrary) {
        throw "Replacement Windows release application did not load the Rust library"
    }
    $resolvedReplacementLibrary = [System.IO.Path]::GetFullPath(
        $replacementRustLibrary.FileName
    )
    if (-not $resolvedReplacementLibrary.Equals(
        $resolvedReleaseLibrary,
        [System.StringComparison]::OrdinalIgnoreCase
    )) {
        throw "Replacement Windows release application loaded Rust from $resolvedReplacementLibrary"
    }
    Stop-Process -Id $replacementProcess.Id -Force
    for ($attempt = 0; $attempt -lt 40; $attempt++) {
        if ($null -eq (Get-Process -Id $replacementProcess.Id -ErrorAction SilentlyContinue)) {
            break
        }
        Start-Sleep -Milliseconds 250
    }
    if ($null -ne (Get-Process -Id $replacementProcess.Id -ErrorAction SilentlyContinue)) {
        throw "Replacement Windows release verification process did not exit"
    }
    $replacementProcess = $null

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
    if ($null -ne $secondaryProcess -and -not $secondaryProcess.HasExited) {
        Stop-Process -Id $secondaryProcess.Id -Force -ErrorAction SilentlyContinue
    }
    if ($null -ne $replacementProcess -and -not $replacementProcess.HasExited) {
        Stop-Process -Id $replacementProcess.Id -Force -ErrorAction SilentlyContinue
    }
    if (Test-Path -LiteralPath $resolvedScratchRoot) {
        Remove-Item -LiteralPath $resolvedScratchRoot -Recurse -Force
    }
    Exit-AmeRepositoryToolLock $toolLock
}
