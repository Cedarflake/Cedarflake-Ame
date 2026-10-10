$ErrorActionPreference = "Stop"
. (Join-Path $PSScriptRoot "quality_common.ps1")
. (Join-Path $PSScriptRoot "integration_windows_runner_lifecycle.ps1")
. (Join-Path $PSScriptRoot "integration_windows_engine_retirement.ps1")

function Assert-EngineRejection {
    param([scriptblock]$Action, [string]$Reason)
    $rejected = $false
    try { & $Action | Out-Null } catch { $rejected = $true }
    if (-not $rejected) { throw "Engine lifecycle guard accepted $Reason" }
}

$caseXml = @(foreach ($phase in @("explicit_destroy", "scope_exit")) {
    '<testcase name="engine_lifecycle_' + $phase + '" classname="engine_lifecycle_' + $phase + '" status="run"><system-out>' +
        "PASS: engine_lifecycle_$phase engine_started=1 child_destroy_notifications=1 scope_completed=1 " +
        'hwnd_retired=1 access_violations=0 plugins_registered=0 com_released_after_scope=1' +
        '</system-out></testcase>'
})
$validXml = '<testsuite name="(empty)" tests="2" failures="0" disabled="0" skipped="0">' +
    ($caseXml -join '') + '</testsuite>'
Assert-AmeWindowsEngineResults $validXml
foreach ($invalid in @(
    $validXml.Replace('tests="2"', 'tests="0"'),
    $validXml.Replace('failures="0"', 'failures="1"'),
    $validXml.Replace('disabled="0"', 'disabled="1"'),
    $validXml.Replace('skipped="0"', 'skipped="1"'),
    $validXml.Replace('<testsuite ', '<testsuite errors="1" '),
    $validXml.Replace($caseXml[1], ''),
    $validXml.Replace('engine_lifecycle_scope_exit', 'engine_lifecycle_explicit_destroy'),
    $validXml.Replace('engine_lifecycle_scope_exit', 'unexpected'),
    $validXml.Replace('status="run"', 'status="notrun"'),
    $validXml.Replace('<system-out>', '<skipped/><system-out>'),
    $validXml.Replace('<system-out>', '<failure/><system-out>'),
    $validXml.Replace('<system-out>', '<error/><system-out>'),
    $validXml.Replace('PASS: ', "PASS: duplicate`nPASS: "),
    ('<!DOCTYPE testsuite [<!ENTITY x "data">]>' + $validXml)
) + @(foreach ($field in @('engine_started', 'child_destroy_notifications', 'scope_completed',
    'hwnd_retired', 'com_released_after_scope')) { $validXml.Replace("$field=1", "$field=0") }
) + @($validXml.Replace('access_violations=0', 'access_violations=1'),
    $validXml.Replace('plugins_registered=0', 'plugins_registered=1'))) {
    Assert-EngineRejection { Assert-AmeWindowsEngineResults $invalid } "incomplete native evidence"
}

$fixture = Join-Path ([IO.Path]::GetTempPath()) ("ame-engine-guard-" + [Guid]::NewGuid().ToString('N'))
$repo = Join-Path $fixture "repo"
$sdk = Join-Path $fixture "sdk"
$cache = Join-Path $sdk "bin/cache"
$source = Join-Path $repo "windows/runner/tests/engine_retirement"
function Write-EngineFixtureFile {
    param([string]$Path, [string]$Text = "fixture")
    $null = [IO.Directory]::CreateDirectory((Split-Path -Parent $Path))
    [IO.File]::WriteAllText($Path, $Text, [Text.UTF8Encoding]::new($false))
}
$actualSource = Join-Path (Get-AmeRepositoryRoot) "windows/runner/tests/engine_retirement"
$required = @(
    "engine.stamp", "engine-dart-sdk.stamp", "flutter_sdk.stamp", "windows-sdk.stamp",
    "engine_stamp.stamp", "flutter_tools.stamp", "flutter_tools.snapshot",
    "dart-sdk/bin/dart.exe", "dart-sdk/bin/dartaotruntime.exe",
    "dart-sdk/bin/snapshots/frontend_server_aot.dart.snapshot",
    "artifacts/engine/common/flutter_patched_sdk/platform_strong.dill",
    "artifacts/engine/windows-x64/flutter_windows.dll",
    "artifacts/engine/windows-x64/flutter_windows.dll.lib",
    "artifacts/engine/windows-x64/flutter_windows.h",
    "artifacts/engine/windows-x64/icudtl.dat",
    "artifacts/engine/windows-x64/cpp_client_wrapper/flutter_engine.cc",
    "artifacts/engine/windows-x64/cpp_client_wrapper/flutter_view_controller.cc"
)
try {
    foreach ($relative in $required) {
        $content = if ($relative.EndsWith('.stamp')) { 'a' * 40 } else { 'fixture' }
        Write-EngineFixtureFile (Join-Path $cache $relative) $content
    }
    Write-EngineFixtureFile (Join-Path $sdk 'bin/flutter.bat')
    foreach ($name in @('pubspec.yaml', 'entrypoint.dart')) {
        Write-EngineFixtureFile (Join-Path $source $name) ([IO.File]::ReadAllText((Join-Path $actualSource $name)))
    }
    Push-Location $repo
    try { $repo = (Get-Location).ProviderPath } finally { Pop-Location }
    $chain = [pscustomobject]@{ Flutter = Join-Path $sdk 'bin/flutter.bat'; Dart = Join-Path $cache 'dart-sdk/bin/dart.exe' }
    Get-AmeWindowsEngineInputs $chain | Out-Null
    $wrapperChain = [pscustomobject]@{ Flutter = $chain.Flutter; Dart = Join-Path $sdk 'bin/dart.bat' }
    if ((Get-AmeWindowsEngineInputs $wrapperChain).Dart -cne $chain.Dart) {
        throw 'The engine fixture must select the prepared Dart executable from its Flutter SDK'
    }
    foreach ($relative in $required) {
        $path = Join-Path $cache $relative
        $bytes = [IO.File]::ReadAllBytes($path)
        Remove-Item -LiteralPath $path
        Assert-EngineRejection { Get-AmeWindowsEngineInputs $chain } "missing prepared artifact $relative"
        [IO.File]::WriteAllBytes($path, $bytes)
    }
    $stampPath = Join-Path $cache 'windows-sdk.stamp'
    Write-EngineFixtureFile $stampPath ('b' * 40)
    Assert-EngineRejection { Get-AmeWindowsEngineInputs $chain } "mismatched SDK stamps"
    Write-EngineFixtureFile $stampPath ('a' * 40)
    $pubspecPath = Join-Path $source 'pubspec.yaml'
    $pubspec = [IO.File]::ReadAllText($pubspecPath)
    Write-EngineFixtureFile $pubspecPath ($pubspec + "`ndependencies:`n  flutter:`n    sdk: flutter`n")
    Assert-EngineRejection { Assert-AmeWindowsEngineFixture $source } "business dependencies"
    Write-EngineFixtureFile $pubspecPath $pubspec
    $entryPath = Join-Path $source 'entrypoint.dart'
    Write-EngineFixtureFile $entryPath "import 'package:flutter/widgets.dart'; void main() {}"
    Assert-EngineRejection { Assert-AmeWindowsEngineFixture $source } "non-no-op entrypoint"
    Write-EngineFixtureFile $entryPath 'void main() {}'

    & {
        $calls = [Collections.Generic.List[object]]::new()
        $reports = [Collections.Generic.List[string]]::new()
        $mode = 'valid'
        $failureStep = 0
        $nativeFailure = [InvalidOperationException]::new('controlled engine build failure')
        function Get-AmeToolchain { $chain }
        function Get-AmeWindowsRunnerTools { [pscustomobject]@{ CMake = 'cmake.exe'; CTest = 'ctest.exe' } }
        function Invoke-AmeWindowsEngineBuildStep {
            param([string]$Command, [string[]]$Arguments, [string]$RunRoot, [string]$Step)
            $calls.Add([pscustomobject]@{ Command = $Command; Arguments = $Arguments; Directory = (Get-Location).Path })
            if ($calls.Count -eq $failureStep) { throw $nativeFailure }
            if ($calls.Count -eq 1) {
                $packages = @(@{ name = 'ame_engine_lifecycle_fixture'; rootUri = '../'; packageUri = 'lib/' })
                if ($mode -ceq 'dependency') { $packages += @{ name = 'unexpected'; rootUri = 'outside'; packageUri = 'lib/' } }
                Write-EngineFixtureFile (Join-Path (Get-Location).Path '.dart_tool/package_config.json') (
                    @{ configVersion = 2; packages = $packages } | ConvertTo-Json -Depth 5)
            } elseif ($calls.Count -eq 2) {
                $directory = Join-Path (Get-Location).Path '.dart_tool/flutter_build/current'
                $kernel = Join-Path $directory 'app.dill'
                Write-EngineFixtureFile $kernel ('x' * 600)
                $outputs = @($kernel, $kernel)
                if ($mode -ceq 'foreign-output') { $outputs += (Join-Path $fixture 'other.dill') }
                Write-EngineFixtureFile (Join-Path $directory 'kernel_snapshot_program.stamp') (
                    @{ outputs = $outputs } | ConvertTo-Json -Depth 3)
                if ($mode -ceq 'old-report') {
                    Write-EngineFixtureFile (Join-Path (Split-Path -Parent (Get-Location).Path) 'results.xml') $validXml
                }
            }
        }
        function Invoke-AmeWindowsEngineCases {
            param([string]$CTest, [string]$BuildRoot, [string]$RunRoot, [string]$JUnitPath)
            $reports.Add($JUnitPath)
            if ($mode -cne 'missing-report') { Write-EngineFixtureFile $JUnitPath $validXml }
            [pscustomobject]@{ primaryExitCode = 0; processExited = $mode -cne 'unretired'; jobClosed = $true }
        }
        $before = (Get-Location).Path
        $result = Invoke-AmeWindowsEngineRetirement $repo
        if ($result.status -cne 'passed' -or $calls.Count -ne 4 -or $reports.Count -ne 1 -or
            (Get-Location).Path -cne $before) { throw 'Real engine workflow did not settle once and restore its location' }
        $package = $calls[0].Directory
        if (-not $package.StartsWith((Join-Path $repo 'build/windows-engine-lifecycle-'), [StringComparison]::OrdinalIgnoreCase) -or
            $package -ceq $source -or ($calls[0].Arguments -join ',') -cne 'pub,get,--offline' -or
            -not $calls[1].Arguments.Contains('-dBuildMode=debug') -or
            -not $calls[1].Arguments.Contains('-dTargetFile=lib/main.dart') -or
            -not $calls[1].Arguments.Contains('kernel_snapshot_program') -or
            -not $calls[3].Arguments.Contains('Debug') -or
            @($calls[2].Arguments | Where-Object { $_.StartsWith('-D') -and $_.Contains('\') }).Count -ne 0) {
            throw 'The real workflow escaped its no-op Debug package or emitted CMake path escapes'
        }
        foreach ($scenario in @('dependency', 'foreign-output', 'old-report', 'missing-report', 'unretired')) {
            $calls.Clear()
            $mode = $scenario
            Assert-EngineRejection { Invoke-AmeWindowsEngineRetirement $repo } $scenario
            if ((Get-Location).Path -cne $before) { throw 'Failure did not restore the caller location' }
        }
        $mode = 'valid'
        foreach ($step in @(1, 2, 3, 4)) {
            $calls.Clear()
            $failureStep = $step
            $observed = $null
            try { Invoke-AmeWindowsEngineRetirement $repo | Out-Null } catch { $observed = $_.Exception }
            if (-not [object]::ReferenceEquals($observed, $nativeFailure) -or $calls.Count -ne $step) {
                throw 'The engine workflow replaced a command failure or continued after it'
            }
        }
        if (@($reports | Select-Object -Unique).Count -ne $reports.Count) {
            throw 'Engine workflow reused native evidence from an earlier invocation'
        }
    }
    $commandRoot = Join-Path $fixture 'command with spaces & literal'
    $commandPath = Join-Path $commandRoot 'controlled.cmd'
    foreach ($code in @(0, 7)) {
        Write-EngineFixtureFile $commandPath ("@echo off`r`necho controlled stdout`r`necho controlled stderr 1>&2`r`nexit /b $code`r`n")
        $observed = $null
        try { Invoke-AmeWindowsEngineBuildStep $commandPath @('unused') $commandRoot 'pub' }
        catch { $observed = $_.Exception }
        if (($null -ne $observed) -ne ($code -ne 0) -or
            -not [IO.File]::ReadAllText((Join-Path $commandRoot 'pub.stdout.log')).Contains('controlled stdout') -or
            -not [IO.File]::ReadAllText((Join-Path $commandRoot 'pub.stderr.log')).Contains('controlled stderr')) {
            throw 'The engine build must retain both streams and use exit status, not stderr, as failure'
        }
    }
    & {
        $nativeRoot = Join-Path $fixture 'process'
        $null = [IO.Directory]::CreateDirectory($nativeRoot)
        foreach ($scenario in @('success', 'run-failure', 'job-failure', 'process-failure')) {
            $fault = [InvalidOperationException]::new("controlled $scenario")
            $fixtureProcess = [pscustomobject]@{ Id = 123; DisposeCount = 0; Failure = $fault; Scenario = $scenario }
            $fixtureProcess | Add-Member ScriptMethod WaitForExit {
                param([int]$Milliseconds)
                if ($Milliseconds -eq 100 -and $this.Scenario -ceq 'run-failure') { throw $this.Failure }
                return $true
            }
            $fixtureProcess | Add-Member ScriptMethod Dispose {
                $this.DisposeCount++
                if ($this.Scenario -ceq 'process-failure') { throw $this.Failure }
            }
            $fixtureJob = [pscustomobject]@{ PrimaryExitCode = 0; Process = $fixtureProcess; DisposeCount = 0;
                Failure = $fault; Scenario = $scenario; CommandLine = '' }
            $fixtureJob | Add-Member ScriptMethod Start {
                param([string]$Command, [string]$Arguments, [string]$Directory)
                $this.CommandLine = $Arguments
                return $this.Process
            }
            $fixtureJob | Add-Member ScriptMethod Dispose {
                $this.DisposeCount++
                if ($this.Scenario -ceq 'job-failure') { throw $this.Failure }
            }
            function New-AmeWindowsEngineProcessJob { return $fixtureJob }
            $observed = $null
            try { Invoke-AmeWindowsEngineCases 'ctest.exe' $nativeRoot $nativeRoot (Join-Path $nativeRoot 'results.xml') | Out-Null }
            catch { $observed = $_.Exception }
            if (($null -ne $observed) -ne ($scenario -cne 'success') -or
                $fixtureJob.DisposeCount -ne 1 -or $fixtureProcess.DisposeCount -ne 1 -or
                -not $fixtureJob.CommandLine.Contains('--timeout 30') -or
                -not (Test-Path -LiteralPath (Join-Path $nativeRoot 'native-completion.json'))) {
                throw 'The real engine process owner lost execution, independent cleanup, or evidence'
            }
            if ($scenario -ceq 'success') {
                $completion = [IO.File]::ReadAllText((Join-Path $nativeRoot 'native-completion.json')) | ConvertFrom-Json
                if ($completion.processExited -ne $true -or $completion.jobClosed -ne $true -or $completion.primaryExitCode -ne 0) {
                    throw 'The engine process owner lacks authoritative retirement evidence'
                }
            }
        }
    }
} finally {
    Write-Host "Compiler-free engine fixture evidence retained: $fixture"
}
Write-Output 'Windows engine lifecycle offline inputs, exact results, and orchestration guardrails passed'
