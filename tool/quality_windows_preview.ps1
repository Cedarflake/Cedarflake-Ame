[CmdletBinding()]
param([Parameter(Mandatory)][ValidateSet('x64', 'arm64')][string]$Architecture)

$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'quality_common.ps1')
. (Join-Path $PSScriptRoot 'quality_unsigned_windows_payload.ps1')

if ([Runtime.InteropServices.RuntimeInformation]::OSArchitecture.ToString() -ine $Architecture -or
    [Runtime.InteropServices.RuntimeInformation]::ProcessArchitecture.ToString() -ine $Architecture -or
    [Environment]::OSVersion.Platform -ne [PlatformID]::Win32NT) {
    throw "Run the preview check in native PowerShell on Windows $Architecture"
}
if ($env:RUNNER_ENVIRONMENT -ne 'github-hosted') {
    throw 'The Release startup check requires a disposable hosted user profile'
}
$root = Get-AmeRepositoryRoot
$toolchain = Get-AmeToolchain
$target = if ($Architecture -eq 'arm64') { 'aarch64-pc-windows-msvc' } else { 'x86_64-pc-windows-msvc' }
$bundle = Join-Path $root "build/windows/$Architecture/runner/Release"
$built = Join-Path $root "build/windows/$Architecture/plugins/rust_lib_cedarflake_ame/cargokit_build/$target/release"
$broker = Join-Path $built 'cedarflake_ame_journal_broker.exe'
$lock = Enter-AmeRepositoryToolLock
Push-Location $root
try {
    $commit = (& $toolchain.Git rev-parse --verify HEAD).Trim()
    if ($LASTEXITCODE -ne 0 -or $commit -cne $env:GITHUB_SHA) { throw 'Preview requires the workflow commit' }
    $changes = @(& $toolchain.Git status --porcelain)
    if ($LASTEXITCODE -ne 0 -or $changes.Count -ne 0) { throw 'Preview requires a clean checkout' }
    Invoke-AmeChecked $toolchain.Flutter @('--version')
    Invoke-AmeChecked $toolchain.Flutter @('pub', 'get', '--enforce-lockfile')
    Invoke-AmeChecked $toolchain.Flutter @('build', 'windows', '--release', '--no-pub')
    Assert-AmeUnsignedBuildPath $broker
    $bundledBroker = Join-Path $bundle 'cedarflake_ame_journal_broker.exe'
    Assert-AmeUnsignedBuildPath $bundledBroker
    Copy-Item -LiteralPath $broker -Destination $bundledBroker -Force
    $facts = Get-AmeUnsignedWindowsPayloadFacts -Architecture $Architecture -BundlePath $bundle `
        -BuiltLibrary (Join-Path $built 'rust_lib_cedarflake_ame.dll') `
        -DependencyFile (Join-Path $built 'rust_lib_cedarflake_ame.d') `
        -RequiredRustSource (Join-Path $root 'rust/src/lib.rs') -BuiltBroker $broker
    $crt = @(Get-AmeMsvcRuntimeCompanionFiles -BundlePath $bundle -Architecture $Architecture)
    foreach ($accelerator in @($false, $true)) {
        $manifest = Get-AmeClassificationRuntimeManifest -Architecture $Architecture -Accelerator:$accelerator
        Assert-AmeClassificationRuntimeDirectory (Join-Path $bundle $manifest.bundleDirectory) $manifest $crt
    }
    Write-Host "Verified $($facts.fileCount) Release files for $($facts.machine)"

    # The runner shows its window after Flutter's first frame.
    $app = Start-Process -FilePath (Join-Path $bundle 'cedarflake_ame.exe') -WorkingDirectory $bundle -PassThru
    try {
        $elapsed = [Diagnostics.Stopwatch]::StartNew()
        do {
            Start-Sleep -Milliseconds 250
            $app.Refresh()
            if ($app.HasExited) { throw "Ame exited before showing its window (exit $($app.ExitCode))" }
            if ($elapsed.Elapsed.TotalSeconds -ge 60) { throw 'Ame did not show its first window within 60 seconds' }
        } until ($app.MainWindowHandle -ne 0 -and $app.MainWindowTitle -eq 'Cedarflake Ame')
        Write-Host "$Architecture desktop window appeared in $($elapsed.ElapsedMilliseconds) ms"
        if (-not $app.CloseMainWindow() -or -not $app.WaitForExit(30000) -or $app.ExitCode -ne 0) {
            throw 'The application did not exit cleanly after closing its window'
        }
    } finally {
        if (-not $app.HasExited) { $app.Kill(); $app.WaitForExit() }
        $app.Dispose()
    }
    $aliasPath = Join-Path $env:RUNNER_TEMP "ame-$Architecture-release-smoke-$PID.dll"
    $previousLibrary = $env:CEDARFLAKE_AME_TEST_LIBRARY_PATH
    Copy-Item -LiteralPath (Join-Path $bundle 'rust_lib_cedarflake_ame.dll') -Destination $aliasPath -ErrorAction Stop
    try {
        $env:CEDARFLAKE_AME_TEST_LIBRARY_PATH = $aliasPath
        Invoke-AmeChecked $toolchain.Flutter @('test', 'integration_test/release_bridge_smoke_test.dart', '-d', 'windows', '--no-pub')
    } finally {
        $env:CEDARFLAKE_AME_TEST_LIBRARY_PATH = $previousLibrary
        Remove-Item -LiteralPath $aliasPath
    }
    $evidence = [ordered]@{
        format = 'cedarflake-ame-windows-preview-v1'; status = 'passed'; sourceCommit = $commit
        dirtyWorktree = $false; windowClosed = $true; bridgeSmoke = 'passed_catalog_read'; payload = $facts
    }
    $evidence | ConvertTo-Json -Depth 8 | Set-Content -Encoding utf8 "build/windows/$Architecture/preview-evidence.json"
} finally {
    Pop-Location
    Exit-AmeRepositoryToolLock $lock
}
