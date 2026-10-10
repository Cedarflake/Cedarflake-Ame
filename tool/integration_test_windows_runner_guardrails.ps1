$ErrorActionPreference = "Stop"
. (Join-Path $PSScriptRoot "quality_common.ps1")
. (Join-Path $PSScriptRoot "integration_windows_runner_lifecycle.ps1")

function Assert-RunnerRejection {
    param([scriptblock]$Action, [string]$Reason)
    $rejected = $false
    try { & $Action | Out-Null } catch { $rejected = $true }
    if (-not $rejected) { throw "Runner lifecycle guard accepted $Reason" }
}

$caseXml = @(foreach ($phase in @("control", "startup", "teardown")) {
    $fonts = if ($phase -ceq "control") { 0 } else { 1 }
    '<testcase name="window_lifecycle_' + $phase + '" classname="window_lifecycle_' +
        $phase + '" time="0.63" status="run"><properties/><system-out>' +
        "PASS: $phase fonts=$fonts returned=$fonts engine_started=0 hwnd_retired=1`n" +
        '</system-out></testcase>'
})
$validXml = '<testsuite name="(empty)" tests="3" failures="0" disabled="0" skipped="0">' +
    ($caseXml -join "") + '</testsuite>'
Assert-AmeWindowsRunnerResults -JUnitXml $validXml
$invalidIndex = 0
foreach ($invalid in @(
    $validXml.Replace('tests="3"', 'tests="0"'),
    $validXml.Replace('failures="0"', 'failures="1"'),
    $validXml.Replace('disabled="0"', 'disabled="1"'),
    $validXml.Replace('skipped="0"', 'skipped="1"'),
    $validXml.Replace('<testsuite ', '<testsuite errors="1" '),
    $validXml.Replace($caseXml[2], ''),
    $validXml.Replace('window_lifecycle_teardown', 'window_lifecycle_startup'),
    $validXml.Replace('window_lifecycle_teardown', 'unexpected_case'),
    $validXml.Replace('status="run"', 'status="disabled"'),
    $validXml.Replace('status="run"', 'status="notrun"'),
    $validXml.Replace('status="run"', 'status="fail"'),
    $validXml.Replace('<system-out>', '<skipped/><system-out>'),
    $validXml.Replace('<system-out>', '<failure/><system-out>'),
    $validXml.Replace('<system-out>', '<error/><system-out>'),
    $validXml.Replace('PASS: teardown', 'PASS: startup'),
    $validXml.Replace('engine_started=0', 'engine_started=1'),
    $validXml.Replace('returned=1', 'returned=0'),
    $validXml.Replace('PASS: control', "PASS: duplicate`nPASS: control"),
    $validXml.Replace('PASS: teardown', 'NOT EXECUTED: teardown'),
    ('<!DOCTYPE testsuite [<!ENTITY x "data">]>' + $validXml)
)) {
    $invalidIndex++
    Assert-RunnerRejection { Assert-AmeWindowsRunnerResults -JUnitXml $invalid } "invalid JUnit evidence $invalidIndex"
}

$fixtureRoot = Join-Path ([System.IO.Path]::GetTempPath()) (
    "ame-runner-guard-" + [Guid]::NewGuid().ToString("N")
)
$binRoot = Join-Path $fixtureRoot "bin"
$cacheRoot = Join-Path $fixtureRoot "build/windows/x64"
$buildRoot = Join-Path $fixtureRoot "build/windows-runner-lifecycle"
$cmakePath = Join-Path $binRoot "cmake.exe"
$ctestPath = Join-Path $binRoot "ctest.exe"
$cachePath = Join-Path $cacheRoot "CMakeCache.txt"
$calls = [System.Collections.Generic.List[object]]::new()
$resultPaths = [System.Collections.Generic.List[string]]::new()
$omitResult = $false
$failureStep = 0
$nativeFailure = [InvalidOperationException]::new("controlled runner command failure")
function Invoke-AmeChecked {
    param([string]$Command, [string[]]$Arguments)
    $calls.Add([pscustomobject]@{ Command = $Command; Arguments = $Arguments })
    if ($calls.Count -eq $failureStep) { throw $nativeFailure }
    if ($Command -ceq $ctestPath) {
        $resultPath = $Arguments[[Array]::IndexOf($Arguments, "--output-junit") + 1]
        $resultPaths.Add($resultPath)
        if (-not $omitResult) { [System.IO.File]::WriteAllText($resultPath, $validXml) }
    }
    Write-Output "Controlled command output"
}
try {
    [System.IO.Directory]::CreateDirectory($binRoot) | Out-Null
    [System.IO.Directory]::CreateDirectory($cacheRoot) | Out-Null
    foreach ($path in @($cmakePath, $ctestPath)) { [System.IO.File]::WriteAllText($path, "") }
    [System.IO.File]::WriteAllText($cachePath, "CMAKE_COMMAND:INTERNAL=$cmakePath`n")
    $results = @(Invoke-AmeWindowsRunnerLifecycle -RepositoryRoot $fixtureRoot)
    if ($results.Count -ne 1 -or $results[0].status -cne "passed" -or $calls.Count -ne 3) {
        throw "Runner lifecycle execution did not return exactly one verified result"
    }
    $expectedArguments = @(
        @("-S", (Join-Path $fixtureRoot "windows/runner/tests"), "-B", $buildRoot, "-A", "x64"),
        @("--build", $buildRoot, "--config", "Release", "--parallel", "1", "--target", "ame_window_lifecycle_test"),
        @("--test-dir", $buildRoot, "--build-config", "Release", "--parallel", "1", "--no-tests=error",
            "--timeout", "15", "--output-on-failure", "--output-junit", $results[0].junitPath)
    )
    for ($index = 0; $index -lt 3; $index++) {
        $expectedCommand = if ($index -eq 2) { $ctestPath } else { $cmakePath }
        if ($calls[$index].Command -cne $expectedCommand -or
            ($calls[$index].Arguments -join "`0") -cne ($expectedArguments[$index] -join "`0")) {
            throw "Runner lifecycle command admission changed at step $index"
        }
    }
    $calls.Clear()
    $omitResult = $true
    Assert-RunnerRejection { Invoke-AmeWindowsRunnerLifecycle -RepositoryRoot $fixtureRoot } "missing fresh report"
    if ($resultPaths.Count -ne 2 -or $resultPaths[0] -ceq $resultPaths[1]) {
        throw "Runner lifecycle execution reused earlier completion evidence"
    }
    $omitResult = $false
    foreach ($step in @(1, 2, 3)) {
        $calls.Clear()
        $failureStep = $step
        $observed = $null
        try { Invoke-AmeWindowsRunnerLifecycle -RepositoryRoot $fixtureRoot | Out-Null }
        catch { $observed = $_.Exception }
        if (-not [object]::ReferenceEquals($observed, $nativeFailure) -or $calls.Count -ne $step) {
            throw "Runner lifecycle execution replaced a command failure or continued after it"
        }
    }
    foreach ($invalidCache in @("", "CMAKE_COMMAND:INTERNAL=relative.exe`n",
        "CMAKE_COMMAND:INTERNAL=$cmakePath`nCMAKE_COMMAND:INTERNAL=$cmakePath`n")) {
        [System.IO.File]::WriteAllText($cachePath, $invalidCache)
        Assert-RunnerRejection { Get-AmeWindowsRunnerTools -RepositoryRoot $fixtureRoot } "ambiguous tool discovery"
    }
    $explicit = Get-AmeWindowsRunnerTools -RepositoryRoot $fixtureRoot -CMakePath $cmakePath
    if ($explicit.CMake -cne $cmakePath -or $explicit.CTest -cne $ctestPath) {
        throw "Runner lifecycle tools did not honor the explicit executable and sibling CTest"
    }
} finally {
    foreach ($path in @($resultPaths.ToArray()) + @($cachePath, $cmakePath, $ctestPath)) {
        if (-not [System.IO.Path]::GetFullPath($path).StartsWith(
            $fixtureRoot + [System.IO.Path]::DirectorySeparatorChar, [StringComparison]::OrdinalIgnoreCase
        )) { throw "Runner guard cleanup target escaped its generated fixture" }
        if (Test-Path -LiteralPath $path -PathType Leaf) { Remove-Item -LiteralPath $path -Force }
    }
    foreach ($path in @($buildRoot, $cacheRoot, (Join-Path $fixtureRoot "build/windows"),
        (Join-Path $fixtureRoot "build"), $binRoot, $fixtureRoot)) {
        if (Test-Path -LiteralPath $path) { [System.IO.Directory]::Delete($path, $false) }
    }
}

$repositoryRoot = Get-AmeRepositoryRoot
$entry = Get-Content -LiteralPath (Join-Path $PSScriptRoot "integration_test_windows_runner.ps1") -Raw -Encoding UTF8
$internal = Get-Content -LiteralPath (Join-Path $PSScriptRoot "integration_windows_runner_lifecycle.ps1") -Raw -Encoding UTF8
foreach ($command in @("Enter-AmeRepositoryToolLock", "Exit-AmeRepositoryToolLock", "Invoke-AmeWindowsRunnerLifecycle", "Invoke-AmeWindowsEngineRetirement")) {
    if ([regex]::Matches($entry, [regex]::Escape($command)).Count -ne 1) {
        throw "The public runner entry must own exactly one lock lifetime and invocation"
    }
}
$tokens = $null
$parseErrors = $null
$entryAst = [System.Management.Automation.Language.Parser]::ParseInput($entry, [ref]$tokens, [ref]$parseErrors)
if ($parseErrors.Count -ne 0) { throw "The runner facade could not be parsed for execution" }
$facadeStatements = @($entryAst.EndBlock.Statements | Where-Object {
    -not ($_ -is [System.Management.Automation.Language.PipelineAst] -and
        $_.PipelineElements.Count -eq 1 -and
        $_.PipelineElements[0] -is [System.Management.Automation.Language.CommandAst] -and
        $_.PipelineElements[0].InvocationOperator -eq [System.Management.Automation.Language.TokenKind]::Dot)
})
if ($facadeStatements.Count -ne $entryAst.EndBlock.Statements.Count - 3) {
    throw "The runner facade must have only its three explicit dependency imports"
}
$facade = [scriptblock]::Create(
    $entryAst.ParamBlock.Extent.Text + "`n" + (($facadeStatements | ForEach-Object { $_.Extent.Text }) -join "`n")
)
& {
    param([scriptblock]$Facade)
    foreach ($scenario in @(@($false, $false, $false), @($true, $false, $false),
        @($false, $true, $false), @($true, $true, $false), @($false, $false, $true), @($false, $true, $true))) {
        $runThrows = $scenario[0]
        $unlockThrows = $scenario[1]
        $engineThrows = $scenario[2]
        $runError = [InvalidOperationException]::new("facade run failed")
        $unlockError = [InvalidOperationException]::new("facade unlock failed")
        $engineError = [InvalidOperationException]::new("facade engine failed")
        $sentinel = [object]::new()
        $expectedResult = [pscustomobject]@{ status = "passed" }
        $order = [System.Collections.Generic.List[string]]::new()
        function Get-AmeRepositoryRoot { return "controlled-runner-repository" }
        function Enter-AmeRepositoryToolLock {
            $order.Add("enter")
            return $sentinel
        }
        function Invoke-AmeWindowsRunnerLifecycle {
            param([string]$RepositoryRoot, [string]$CMakePath)
            if ($RepositoryRoot -cne "controlled-runner-repository" -or $CMakePath -cne "controlled-cmake") {
                throw "The real facade changed the configured invocation"
            }
            $order.Add("run")
            if ($runThrows) { throw $runError }
            return $expectedResult
        }
        function Exit-AmeRepositoryToolLock {
            param([object]$Mutex)
            if (-not [object]::ReferenceEquals($Mutex, $sentinel)) { throw "The facade released another lock" }
            $order.Add("release")
            if ($unlockThrows) { throw $unlockError }
        }
        function Invoke-AmeWindowsEngineRetirement {
            param([string]$RepositoryRoot, [string]$CMakePath)
            if ($RepositoryRoot -cne "controlled-runner-repository" -or $CMakePath -cne "controlled-cmake") {
                throw "The facade changed the engine invocation"
            }
            $order.Add("engine")
            if ($engineThrows) { throw $engineError }
            return $expectedResult
        }
        $outputs = [System.Collections.Generic.List[object]]::new()
        $observed = $null
        try { & $Facade -CMakePath "controlled-cmake" | ForEach-Object { $outputs.Add($_) } }
        catch { $observed = $_.Exception }
        $expectedOrder = if ($runThrows) { "enter,run,release" } else { "enter,run,engine,release" }
        if (($order -join ",") -cne $expectedOrder) {
            throw "The real facade must acquire, invoke, and release exactly once"
        }
        $expectedError = if ($runThrows) { $runError } elseif ($engineThrows) { $engineError } elseif ($unlockThrows) { $unlockError } else { $null }
        if (-not [object]::ReferenceEquals($observed, $expectedError)) {
            throw "The real facade replaced the original failure or ignored unlock failure"
        }
        if ($null -eq $expectedError) {
            if ($outputs.Count -ne 1 -or
                -not [object]::ReferenceEquals($outputs[0].windowLifecycle, $expectedResult) -or
                -not [object]::ReferenceEquals($outputs[0].engineRetirement, $expectedResult)) {
                throw "The real facade changed or duplicated its normal result"
            }
        } elseif ($outputs.Count -ne 0) {
            throw "The real facade emitted success before unlock completed"
        }
        if ($unlockThrows -and $observed.Data["ameWindowsRunnerUnlockFailure"] -cne $unlockError.ToString()) {
            throw "The real facade lost the independently recorded unlock failure"
        }
    }
} $facade
if ($internal -match '(Enter|Exit)-AmeRepositoryToolLock') {
    throw "Internal runner execution must not reacquire the caller's tool lock"
}
$unsigned = Get-Content -LiteralPath (Join-Path $PSScriptRoot "quality_verify_unsigned_windows.ps1") -Raw -Encoding UTF8
$release = $unsigned.IndexOf("@('build', 'windows', '--release', '--no-pub')", [StringComparison]::Ordinal)
$native = $unsigned.IndexOf("Invoke-AmeWindowsRunnerLifecycle -RepositoryRoot", [StringComparison]::Ordinal)
$broker = $unsigned.IndexOf('Invoke-AmeChecked $toolchain.Cargo', [StringComparison]::Ordinal)
$engine = $unsigned.IndexOf("Invoke-AmeWindowsEngineRetirement -RepositoryRoot", [StringComparison]::Ordinal)
if ($release -lt 0 -or $native -le $release -or $engine -le $native -or $broker -le $engine -or
    [regex]::Matches($unsigned, 'Invoke-AmeWindowsRunnerLifecycle').Count -ne 1 -or
    [regex]::Matches($unsigned, 'Invoke-AmeWindowsEngineRetirement').Count -ne 1 -or
    $unsigned.Contains('integration_test_windows_runner.ps1')) {
    throw "Unsigned verification must run the internal lifecycle gate after the pinned Release build"
}
& (Join-Path $PSScriptRoot "integration_test_windows_engine_guardrails.ps1")
Write-Output "Windows runner lifecycle result, command, and lock ownership guardrails passed"
