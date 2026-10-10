. (Join-Path $PSScriptRoot "integration_windows_engine_inputs.ps1")
. (Join-Path $PSScriptRoot "integration_windows_engine_results.ps1")
. (Join-Path $PSScriptRoot "integration_windows_scan_run.ps1")
. (Join-Path $PSScriptRoot "integration_windows_accessibility_cleanup.ps1")

function New-AmeWindowsEngineProcessJob {
    Initialize-AmeWindowsAccessibilityProcessJob
    return [AmeWindowsAccessibilityProcessJob]::new()
}

function Invoke-AmeWindowsEngineBuildStep {
    param([string]$Command, [string[]]$Arguments, [string]$RunRoot,
        [ValidateSet("pub", "kernel", "configure", "build")][string]$Step)

    $stdout = Join-Path $RunRoot "$Step.stdout.log"
    $stderr = Join-Path $RunRoot "$Step.stderr.log"
    foreach ($value in @($Command, $stdout, $stderr) + $Arguments) {
        if ($value.IndexOfAny([char[]]@('"', '%', "`r", "`n", "`0")) -ge 0) {
            throw "The engine build argument contains unsupported shell characters"
        }
    }
    $quoted = @(@($Command) + $Arguments | ForEach-Object { '"' + $_ + '"' }) -join ' '
    $line = $quoted + ' 1>"' + $stdout + '" 2>"' + $stderr + '"'
    $cmd = Join-Path ([Environment]::GetFolderPath("System")) "cmd.exe"
    $process = [Diagnostics.Process]::new()
    $process.StartInfo.FileName = $cmd
    $process.StartInfo.Arguments = '/d /v:off /s /c "' + $line + '"'
    $process.StartInfo.WorkingDirectory = (Get-Location).ProviderPath
    $process.StartInfo.UseShellExecute = $false
    $process.StartInfo.CreateNoWindow = $true
    $failure = $null
    $cleanup = New-AmeWindowsAccessibilityCleanup
    try {
        if (-not $process.Start()) { throw "The engine build process did not start" }
        $process.WaitForExit()
        if ($process.ExitCode -ne 0) { throw "$Command failed with exit code $($process.ExitCode)" }
    } catch {
        $failure = $_.Exception
    } finally {
        Invoke-AmeWindowsAccessibilityCleanupStep $cleanup "dispose-engine-build-process" { $process.Dispose() }
    }
    if ($null -eq $failure) { $failure = Get-AmeWindowsAccessibilityCleanupFailure $cleanup }
    if ($null -ne $failure) {
        $failure.Data["ameEngineBuildCleanupFailures"] = @(Get-AmeWindowsAccessibilityCleanupRecords $cleanup)
        throw $failure
    }
}

function Invoke-AmeWindowsEngineCases {
    param([string]$CTest, [string]$BuildRoot, [string]$RunRoot, [string]$JUnitPath)

    $stdout = Join-Path $RunRoot "native.stdout.log"
    $stderr = Join-Path $RunRoot "native.stderr.log"
    foreach ($path in @($CTest, $BuildRoot, $RunRoot, $JUnitPath, $stdout, $stderr)) {
        if ($path.IndexOfAny([char[]]@('"', '%', "`r", "`n", "`0")) -ge 0) {
            throw "The engine fixture command path contains unsupported shell characters"
        }
    }
    $job = $null
    $process = $null
    $failure = $null
    $exitCode = $null
    $processId = $null
    $cleanup = New-AmeWindowsAccessibilityCleanup
    $clock = [Diagnostics.Stopwatch]::StartNew()
    try {
        $job = New-AmeWindowsEngineProcessJob
        $command = '"' + $CTest + '" --test-dir "' + $BuildRoot +
            '" --build-config Debug --parallel 1 --no-tests=error --timeout 30' +
            ' --output-on-failure --output-junit "' + $JUnitPath +
            '" 1>"' + $stdout + '" 2>"' + $stderr + '"'
        $cmd = Join-Path ([Environment]::GetFolderPath("System")) "cmd.exe"
        $process = $job.Start($cmd, '/d /v:off /s /c "' + $command + '"', $RunRoot)
        $processId = $process.Id
        while (-not $process.WaitForExit(100)) {
            if ($clock.Elapsed.TotalSeconds -ge 75) {
                throw [TimeoutException]::new("Engine lifecycle cases exceeded the 75-second parent deadline")
            }
        }
        if ($clock.Elapsed.TotalSeconds -ge 75) {
            throw [TimeoutException]::new("Engine lifecycle cases exceeded the 75-second parent deadline")
        }
        $exitCode = [int]$job.PrimaryExitCode
        if ($exitCode -ne 0) { throw "Engine lifecycle CTest failed with exit code $exitCode" }
    } catch {
        $failure = $_.Exception
    } finally {
        Close-AmeWindowsAccessibilityProcess $cleanup $job $process
    }
    if ($null -eq $failure) { $failure = Get-AmeWindowsAccessibilityCleanupFailure $cleanup }
    $evidence = [ordered]@{
        processId = $processId
        primaryExitCode = $exitCode
        processExited = $cleanup.ProcessExited
        jobClosed = $cleanup.JobClosed
        elapsedMs = $clock.ElapsedMilliseconds
        stdoutPath = $stdout
        stderrPath = $stderr
        cleanupFailures = @(Get-AmeWindowsAccessibilityCleanupRecords $cleanup)
        failure = if ($null -ne $failure) { $failure.Message } else { $null }
    }
    try {
        [IO.File]::WriteAllText((Join-Path $RunRoot "native-completion.json"),
            ($evidence | ConvertTo-Json -Depth 5), [Text.UTF8Encoding]::new($false))
    } catch {
        if ($null -eq $failure) { $failure = $_.Exception }
        else { $failure.Data["ameEngineEvidenceFailure"] = $_.Exception.ToString() }
    }
    if ($null -ne $failure) {
        $failure.Data["ameEngineProcessEvidence"] = $evidence
        throw $failure
    }
    return [pscustomobject]$evidence
}

function Invoke-AmeWindowsEngineRetirement {
    param([Parameter(Mandatory = $true)][string]$RepositoryRoot, [string]$CMakePath = "")

    if ([Environment]::OSVersion.Platform -ne [PlatformID]::Win32NT -or
        [Runtime.InteropServices.RuntimeInformation]::OSArchitecture -ne 'X64' -or
        -not [Environment]::Is64BitProcess) { throw "The engine lifecycle gate requires Windows x64" }
    $toolchain = Get-AmeToolchain
    $inputs = Get-AmeWindowsEngineInputs $toolchain
    $tools = Get-AmeWindowsRunnerTools -RepositoryRoot $RepositoryRoot -CMakePath $CMakePath
    $source = Join-Path $RepositoryRoot "windows/runner/tests/engine_retirement"
    Assert-AmeWindowsEngineFixture $source
    Assert-AmeWindowsScanDirectoryChain $RepositoryRoot
    $build = Join-Path $RepositoryRoot "build"
    if (-not (Test-Path -LiteralPath $build)) { $null = New-Item -ItemType Directory -Path $build }
    Assert-AmeWindowsScanDirectoryChain $build
    $runRoot = Join-Path $build ("windows-engine-lifecycle-" + [Guid]::NewGuid().ToString("N"))
    if (Test-Path -LiteralPath $runRoot) { throw "Engine lifecycle storage must be fresh" }
    $null = New-Item -ItemType Directory -Path $runRoot
    Assert-AmeWindowsScanDirectoryChain $runRoot
    $package = Join-Path $runRoot "package"
    $assets = Join-Path $runRoot "flutter_assets"
    $nativeBuild = Join-Path $runRoot "native"
    $junit = Join-Path $runRoot "results.xml"
    $null = New-Item -ItemType Directory -Path (Join-Path $package "lib"), $assets
    Copy-Item -LiteralPath (Join-Path $source "pubspec.yaml") -Destination (Join-Path $package "pubspec.yaml")
    Copy-Item -LiteralPath (Join-Path $source "entrypoint.dart") -Destination (Join-Path $package "lib/main.dart")
    Write-Host "Engine lifecycle evidence (retained): $runRoot"
    $failure = $null
    $pushed = $false
    $native = $null
    try {
        Push-Location $package
        $pushed = $true
        $package = (Get-Location).ProviderPath
        Invoke-AmeWindowsEngineBuildStep $inputs.Dart @("pub", "get", "--offline") $runRoot "pub"
        Assert-AmeWindowsEnginePackage $package
        Invoke-AmeWindowsEngineBuildStep $toolchain.Flutter @(
            "--no-version-check", "--suppress-analytics", "assemble",
            "-dTargetPlatform=windows-x64", "-dBuildMode=debug", "-dTargetFile=lib/main.dart",
            "-dTrackWidgetCreation=false", "--resource-pool-size=1", "--output=build/assemble",
            "kernel_snapshot_program"
        ) $runRoot "kernel"
        $kernel = Get-AmeWindowsEngineKernel $package
        Copy-Item -LiteralPath $kernel -Destination (Join-Path $assets "kernel_blob.bin")
        Invoke-AmeWindowsEngineBuildStep $tools.CMake @(
            "-S", $source, "-B", $nativeBuild, "-A", "x64",
            "-DAME_FLUTTER_ENGINE_ROOT=$($inputs.EngineRoot.Replace('\', '/'))",
            "-DAME_FLUTTER_ASSETS_DIR=$($assets.Replace('\', '/'))",
            "-DAME_FLUTTER_ICU_PATH=$($inputs.IcuPath.Replace('\', '/'))"
        ) $runRoot "configure"
        Invoke-AmeWindowsEngineBuildStep $tools.CMake @(
            "--build", $nativeBuild, "--config", "Debug", "--parallel", "1",
            "--target", "ame_window_engine_lifecycle_test"
        ) $runRoot "build"
        if (Test-Path -LiteralPath $junit) { throw "Engine lifecycle evidence must not predate native execution" }
        $native = Invoke-AmeWindowsEngineCases $tools.CTest $nativeBuild $runRoot $junit
        if ($native.primaryExitCode -ne 0 -or $native.processExited -ne $true -or $native.jobClosed -ne $true) {
            throw "Engine lifecycle execution lacks owned process retirement evidence"
        }
        $report = Get-Item -LiteralPath $junit -ErrorAction Stop
        if ($report.PSIsContainer -or $report.Length -eq 0 -or $report.Length -gt 1MB) {
            throw "Engine lifecycle evidence is empty or exceeds its bound"
        }
        Assert-AmeWindowsEngineResults -JUnitXml ([IO.File]::ReadAllText($junit))
    } catch {
        $failure = $_.Exception
    } finally {
        if ($pushed) {
            try { Pop-Location } catch {
                if ($null -eq $failure) { $failure = $_.Exception }
                else { $failure.Data["ameEngineLocationFailure"] = $_.Exception.ToString() }
            }
        }
    }
    if ($null -ne $failure) {
        $failure.Data["ameEngineEvidenceRoot"] = $runRoot
        throw $failure
    }
    return [pscustomobject]@{
        status = "passed"
        cases = @("engine_lifecycle_explicit_destroy", "engine_lifecycle_scope_exit")
        engineStarted = $true
        engineRevision = $inputs.EngineRevision
        junitPath = $junit
        nativeProcess = $native
        retainedEvidenceRoot = $runRoot
    }
}
