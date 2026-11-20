[CmdletBinding()]
param()

$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'quality_common.ps1')
. (Join-Path $PSScriptRoot 'quality_unsigned_windows_payload.ps1')

if ([Runtime.InteropServices.RuntimeInformation]::OSArchitecture -ne 'Arm64' -or
    [Runtime.InteropServices.RuntimeInformation]::ProcessArchitecture -ne 'Arm64' -or
    [Environment]::OSVersion.Platform -ne [PlatformID]::Win32NT) {
    throw 'Run the preview check in native PowerShell on Windows ARM64'
}
if ($env:RUNNER_ENVIRONMENT -ne 'github-hosted') {
    throw 'The Release startup check requires a disposable hosted user profile'
}
$root = Get-AmeRepositoryRoot
$toolchain = Get-AmeToolchain
$bundle = Join-Path $root 'build/windows/arm64/runner/Release'
$built = Join-Path $root 'build/windows/arm64/plugins/rust_lib_cedarflake_ame/cargokit_build/aarch64-pc-windows-msvc/release'
$broker = Join-Path $built 'cedarflake_ame_journal_broker.exe'
$lock = Enter-AmeRepositoryToolLock
Push-Location $root
try {
    Invoke-AmeChecked $toolchain.Flutter @('--version')
    Invoke-AmeChecked $toolchain.Flutter @('pub', 'get', '--enforce-lockfile')
    Invoke-AmeChecked $toolchain.Flutter @('build', 'windows', '--release', '--no-pub')
    Assert-AmeUnsignedBuildPath $broker
    $bundledBroker = Join-Path $bundle 'cedarflake_ame_journal_broker.exe'
    Assert-AmeUnsignedBuildPath $bundledBroker
    Copy-Item -LiteralPath $broker -Destination $bundledBroker -Force
    $facts = Get-AmeUnsignedWindowsPayloadFacts -Architecture arm64 -BundlePath $bundle `
        -BuiltLibrary (Join-Path $built 'rust_lib_cedarflake_ame.dll') `
        -DependencyFile (Join-Path $built 'rust_lib_cedarflake_ame.d') `
        -RequiredRustSource (Join-Path $root 'rust/src/lib.rs') -BuiltBroker $broker
    $crt = @(Get-AmeMsvcRuntimeCompanionFiles -BundlePath $bundle -Architecture arm64)
    foreach ($accelerator in @($false, $true)) {
        $manifest = Get-AmeClassificationRuntimeManifest -Architecture arm64 -Accelerator:$accelerator
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
        Write-Host "ARM64 desktop window appeared in $($elapsed.ElapsedMilliseconds) ms"
        if (-not $app.CloseMainWindow() -or -not $app.WaitForExit(30000) -or $app.ExitCode -ne 0) {
            throw 'The ARM64 application did not exit cleanly after closing its window'
        }
    } finally {
        if (-not $app.HasExited) { $app.Kill(); $app.WaitForExit() }
        $app.Dispose()
    }
} finally {
    Pop-Location
    Exit-AmeRepositoryToolLock $lock
}
