function Get-AmeWindowsRunnerTools {
    param(
        [Parameter(Mandatory = $true)][string]$RepositoryRoot,
        [string]$CMakePath = ""
    )

    if ([string]::IsNullOrWhiteSpace($CMakePath)) {
        $cachePath = Join-Path $RepositoryRoot "build/windows/x64/CMakeCache.txt"
        if (-not (Test-Path -LiteralPath $cachePath -PathType Leaf)) {
            throw "Prepare the pinned Flutter Windows build or supply -CMakePath"
        }
        $commands = @([regex]::Matches(
            (Get-Content -LiteralPath $cachePath -Raw -Encoding UTF8),
            '(?m)^CMAKE_COMMAND:INTERNAL=([^\r\n]+)\r?$'
        ))
        if ($commands.Count -ne 1) {
            throw "The Flutter CMake cache must identify exactly one CMAKE_COMMAND"
        }
        $CMakePath = $commands[0].Groups[1].Value
        if (-not [System.IO.Path]::IsPathRooted($CMakePath)) {
            throw "The Flutter CMake cache must identify an absolute CMake executable"
        }
    }
    $cmake = Get-Item -LiteralPath $CMakePath -ErrorAction Stop
    if ($cmake.PSIsContainer -or $cmake.Name -ine "cmake.exe") {
        throw "The runner lifecycle gate requires an explicit cmake.exe"
    }
    $ctest = Join-Path $cmake.DirectoryName "ctest.exe"
    if (-not (Test-Path -LiteralPath $ctest -PathType Leaf)) {
        throw "ctest.exe must be available beside the selected cmake.exe"
    }
    return [pscustomobject]@{ CMake = $cmake.FullName; CTest = $ctest }
}

function Assert-AmeWindowsRunnerResults {
    param([Parameter(Mandatory = $true)][string]$JUnitXml)

    $settings = [System.Xml.XmlReaderSettings]::new()
    $settings.DtdProcessing = [System.Xml.DtdProcessing]::Prohibit
    $settings.XmlResolver = $null
    $settings.MaxCharactersInDocument = 1MB
    $text = [System.IO.StringReader]::new($JUnitXml)
    $reader = $null
    $document = [System.Xml.XmlDocument]::new()
    $document.XmlResolver = $null
    try {
        $reader = [System.Xml.XmlReader]::Create($text, $settings)
        $document.Load($reader)
    } finally {
        if ($null -ne $reader) { $reader.Dispose() }
        $text.Dispose()
    }
    $suite = $document.DocumentElement
    if ($null -eq $suite -or $suite.get_Name() -cne "testsuite" -or
        $suite.GetAttribute("tests") -cne "3") {
        throw "Runner lifecycle results must contain exactly three executed cases"
    }
    foreach ($attribute in @("failures", "disabled", "skipped")) {
        if ($suite.GetAttribute($attribute) -cne "0") {
            throw "Runner lifecycle results contain unsuccessful or unexecuted cases: $attribute"
        }
    }
    if ($suite.GetAttribute("errors") -cnotin @("", "0")) {
        throw "Runner lifecycle results contain errors"
    }
    $cases = @($document.SelectNodes("/testsuite/testcase"))
    if ($cases.Count -ne 3) {
        throw "Runner lifecycle results have an incomplete or duplicate case roster"
    }
    foreach ($phase in @("control", "startup", "teardown")) {
        $matches = @($cases | Where-Object {
            $_.GetAttribute("name") -ceq "window_lifecycle_$phase"
        })
        if ($matches.Count -ne 1) {
            throw "Runner lifecycle case must execute exactly once: $phase"
        }
        $case = $matches[0]
        if ($case.GetAttribute("status") -cne "run" -or
            $case.SelectNodes("failure|error|skipped").Count -ne 0) {
            throw "Runner lifecycle case did not pass execution: $phase"
        }
        $outputs = @($case.SelectNodes("system-out"))
        $fontCount = if ($phase -ceq "control") { 0 } else { 1 }
        $marker = "PASS: $phase fonts=$fontCount returned=$fontCount engine_started=0 hwnd_retired=1"
        if ($outputs.Count -ne 1 -or
            [regex]::Matches($outputs[0].InnerText, '(?m)^PASS:[^\r\n]*\r?$').Count -ne 1 -or
            [regex]::Matches($outputs[0].InnerText, '(?m)^' + [regex]::Escape($marker) + '\r?$').Count -ne 1) {
            throw "Runner lifecycle case lacks its exact native completion evidence: $phase"
        }
    }
}

function Invoke-AmeWindowsRunnerLifecycle {
    param(
        [Parameter(Mandatory = $true)][string]$RepositoryRoot,
        [string]$CMakePath = ""
    )

    if ([Environment]::OSVersion.Platform -ne [PlatformID]::Win32NT -or
        [System.Runtime.InteropServices.RuntimeInformation]::OSArchitecture -ne 'X64' -or
        -not [Environment]::Is64BitProcess) {
        throw "The runner lifecycle gate requires a Windows x64 process"
    }
    $tools = Get-AmeWindowsRunnerTools -RepositoryRoot $RepositoryRoot -CMakePath $CMakePath
    $sourceRoot = Join-Path $RepositoryRoot "windows/runner/tests"
    $buildRoot = Join-Path $RepositoryRoot "build/windows-runner-lifecycle"
    $junitPath = Join-Path $buildRoot ("results-" + [Guid]::NewGuid().ToString("N") + ".xml")
    New-Item -ItemType Directory -Path $buildRoot -Force | Out-Null
    if (Test-Path -LiteralPath $junitPath) {
        throw "Runner lifecycle evidence requires a fresh result path"
    }
    Write-Host "Runner lifecycle evidence: $junitPath"
    Invoke-AmeChecked $tools.CMake @("-S", $sourceRoot, "-B", $buildRoot, "-A", "x64") | Out-Host
    Invoke-AmeChecked $tools.CMake @(
        "--build", $buildRoot, "--config", "Release", "--parallel", "1",
        "--target", "ame_window_lifecycle_test"
    ) | Out-Host
    Invoke-AmeChecked $tools.CTest @(
        "--test-dir", $buildRoot, "--build-config", "Release", "--parallel", "1",
        "--no-tests=error", "--timeout", "15", "--output-on-failure", "--output-junit", $junitPath
    ) | Out-Host
    $report = Get-Item -LiteralPath $junitPath -ErrorAction Stop
    if ($report.PSIsContainer -or $report.Length -eq 0 -or $report.Length -gt 1MB) {
        throw "Runner lifecycle evidence is empty or exceeds its bound"
    }
    Assert-AmeWindowsRunnerResults -JUnitXml (Get-Content -LiteralPath $junitPath -Raw -Encoding UTF8)
    return [pscustomobject]@{
        status = "passed"
        cases = @("window_lifecycle_control", "window_lifecycle_startup", "window_lifecycle_teardown")
        engineStarted = $false
        junitPath = $junitPath
    }
}
