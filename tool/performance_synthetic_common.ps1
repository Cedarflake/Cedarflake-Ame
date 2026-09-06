Set-StrictMode -Version Latest
. (Join-Path $PSScriptRoot "quality_common.ps1")
. (Join-Path $PSScriptRoot "integration_windows_accessibility_process.ps1")

function Get-AmeSyntheticBuildArguments {
    @("test", "--locked", "--manifest-path", "rust/Cargo.toml", "--release", "--lib", "--all-features", "--jobs", "1", "--no-run", "--message-format=json")
}

function Test-AmeSyntheticPlatform {
    param([PlatformID]$Platform, [Runtime.InteropServices.Architecture]$Architecture, [bool]$Is64BitProcess)
    return $Platform -eq [PlatformID]::Win32NT -and
        $Architecture -eq [Runtime.InteropServices.Architecture]::X64 -and $Is64BitProcess
}

function Get-AmeSyntheticCases {
    @(
        [pscustomobject]@{ Name = "jpeg"; Test = "adapters::jpeg_preview::tests::benchmark_high_resolution_jpeg_preview_conversion"; Seconds = 180; Bytes = 536870912; Evidence = '(?m)^full_decode_resize_ms=[0-9.]+ scaled_decode_resize_ms=[0-9.]+ speedup=[0-9.]+\r?$' }
        [pscustomobject]@{ Name = "scan"; Test = "application::scan_library::tests::synthetic_ten_thousand_file_scan_records_bounded_acceptance_evidence"; Seconds = 360; Bytes = 536870912; Evidence = '(?m)^AME_SYNTHETIC_BENCHMARK files=10000 fixture_ms=\d+ cold_ms=\d+ warm_ms=\d+ pause_ms=\d+ resume_ms=\d+ cancel_ms=\d+ catalog_bytes=[0-9]+ resumed_catalog_bytes=[0-9]+\r?$' }
        [pscustomobject]@{ Name = "usn"; Test = "journal_broker::windows::usn::tests::million_unrelated_records_stream_through_bounded_production_parser_pages"; Seconds = 180; Bytes = 536870912; Evidence = '(?m)^R2c-R million-backlog records=1000000 covered_records=1000000 pages=245 production_records_per_native_buffer=4095 .* root_scope_checks=1000000 emitted_candidates=0 elapsed_ms=\d+\r?$' }
    )
}

function Assert-AmeSyntheticList {
    param([string]$Output, [string]$TestName)
    if ($TestName -cnotin @((Get-AmeSyntheticCases).Test)) {
        throw "Synthetic test is not allowlisted"
    }
    $expected = "${TestName}: test`n`n1 test, 0 benchmarks"
    if ($Output.Replace("`r`n", "`n").Trim() -cne $expected) {
        throw "Synthetic list must prove exactly one allowlisted test"
    }
}

function Assert-AmeSyntheticResult {
    param([string]$Output, [string]$TestName)
    $case = @(Get-AmeSyntheticCases | Where-Object { $_.Test -ceq $TestName })
    if ($case.Count -ne 1) { throw "Synthetic test is not allowlisted" }
    $summary = [regex]::Matches($Output, '(?m)^test result: .*$')
    if ($summary.Count -ne 1 -or $summary[0].Value -cnotmatch '^test result: ok\. 1 passed; 0 failed; 0 ignored; 0 measured; \d+ filtered out; finished in [0-9.]+s\r?$') {
        throw "Synthetic execution must prove one passed test and no ignored tests"
    }
    if ([regex]::Matches($Output, '(?m)^running 1 test\r?$').Count -ne 1 -or
        [regex]::Matches($Output, '(?m)^test ' + [regex]::Escape($TestName) + ' \.\.\. ').Count -ne 1 -or
        [regex]::Matches([regex]::Replace($Output, '(?m)^test ' + [regex]::Escape($TestName) + ' \.\.\. ', ''), $case[0].Evidence).Count -ne 1) {
        throw "Synthetic execution is missing its exact workload or evidence"
    }
}

function New-AmeSyntheticEvidenceDirectory {
    param([string]$RepositoryRoot, [string]$Category = "performance_synthetic")
    if ($Category -cnotin @("performance_synthetic", "performance_synthetic_guardrails")) { throw "Invalid synthetic storage category" }
    $path = [IO.Path]::GetFullPath($RepositoryRoot)
    foreach ($component in @("", ".dart_tool", $Category)) {
        if ($component) { $path = Join-Path $path $component }
        if (Test-Path -LiteralPath $path) {
            $entry = Get-Item -LiteralPath $path -Force
            if (-not $entry.PSIsContainer -or ($entry.Attributes -band [IO.FileAttributes]::ReparsePoint)) {
                throw "Synthetic evidence storage must use ordinary repository directories"
            }
        } else { $null = New-Item -ItemType Directory -Path $path }
    }
    $path = Join-Path $path ([Guid]::NewGuid().ToString("N"))
    $null = New-Item -ItemType Directory -Path $path
    return $path
}

function Get-AmeSyntheticTestExecutable {
    param([string]$Output, [string]$RepositoryRoot)
    $manifest = Join-Path $RepositoryRoot "rust\Cargo.toml"
    $artifacts = @($Output -split '\r?\n' | Where-Object { $_.Trim() } | ForEach-Object {
        $record = $_ | ConvertFrom-Json
        if ($record.reason -eq "compiler-artifact" -and $record.target.name -ceq "rust_lib_cedarflake_ame" -and $record.profile.test -eq $true -and $null -ne $record.executable) {
            if ([IO.Path]::GetFullPath($record.manifest_path) -ine $manifest) {
                throw "Synthetic artifact has an unexpected manifest"
            }
            $record.executable
        }
    })
    if ($artifacts.Count -ne 1) { throw "Synthetic build must identify exactly one library-test executable" }
    $executable = [IO.Path]::GetFullPath($artifacts[0])
    $expectedParent = Join-Path $RepositoryRoot "rust\target\release\deps"
    if ((Split-Path -Parent $executable) -ine $expectedParent -or
        (Split-Path -Leaf $executable) -cnotmatch '^rust_lib_cedarflake_ame-[a-f0-9]+\.exe$' -or
        -not (Test-Path -LiteralPath $executable -PathType Leaf)) {
        throw "Synthetic build did not produce the expected release test artifact"
    }
    return $executable
}

function Invoke-AmeSyntheticOwnedProcess {
    param(
        [Parameter(Mandatory = $true)] [string]$Executable,
        [Parameter(Mandatory = $true)] [string[]]$Arguments,
        [Parameter(Mandatory = $true)] [string]$RepositoryRoot,
        [Parameter(Mandatory = $true)] [string]$Directory,
        [Parameter(Mandatory = $true)] [string]$Name,
        [ValidateRange(1, 1800)] [int]$Seconds,
        [ValidateRange(1, 8589934592)] [long]$MaximumBytes,
        [ValidateRange(1024, 8388608)] [int]$MaximumOutputBytes = 4194304
    )
    if ($Name -cnotmatch '^[a-z][a-z0-9-]*$') { throw "Invalid synthetic phase name" }
    $token = [Guid]::NewGuid().ToString("N")
    $requestPath = Join-Path $Directory "$Name.request.json"
    $resultPath = Join-Path $Directory "$Name.json"
    $stdoutPath = Join-Path $Directory "$Name.stdout.log"
    $stderrPath = Join-Path $Directory "$Name.stderr.log"
    $scratch = Join-Path $Directory "$Name-scratch"
    foreach ($path in @($requestPath, $resultPath, $stdoutPath, $stderrPath, $scratch)) {
        if (Test-Path -LiteralPath $path) { throw "Synthetic process evidence must be fresh" }
    }
    $null = New-Item -ItemType Directory -Path $scratch
    $request = [ordered]@{
        token = $token; executable = $Executable; arguments = @($Arguments)
        repository = $RepositoryRoot; result = $resultPath; stdout = $stdoutPath; stderr = $stderrPath
        scratch = $scratch; seconds = $Seconds; maximumBytes = $MaximumBytes; maximumOutputBytes = $MaximumOutputBytes
    }
    [IO.File]::WriteAllText($requestPath, ($request | ConvertTo-Json -Depth 4), [Text.UTF8Encoding]::new($false))
    $worker = Join-Path $PSScriptRoot "performance_synthetic_worker.ps1"
    $command = "& '" + $worker.Replace("'", "''") + "' -RequestPath '" + $requestPath.Replace("'", "''") + "'"
    $encoded = [Convert]::ToBase64String([Text.Encoding]::Unicode.GetBytes($command))
    Initialize-AmeWindowsAccessibilityProcessJob
    $job = [AmeWindowsAccessibilityProcessJob]::new()
    $process = $null
    $elapsed = [Diagnostics.Stopwatch]::StartNew()
    try {
        $process = $job.Start((Get-Process -Id $PID).Path, "-NoLogo -NoProfile -NonInteractive -EncodedCommand $encoded", $RepositoryRoot)
        while (-not $process.WaitForExit(100)) {
            if ($elapsed.Elapsed.TotalSeconds -ge ($Seconds + 15)) { throw "Synthetic process exceeded its parent deadline" }
        }
        $exitCode = $job.PrimaryExitCode
        if (-not (Test-Path -LiteralPath $resultPath -PathType Leaf) -or (Get-Item -LiteralPath $resultPath).Length -gt 65536) {
            throw "Synthetic process did not produce bounded completion evidence"
        }
        $result = Get-Content -LiteralPath $resultPath -Raw -Encoding UTF8 | ConvertFrom-Json
        if ($result.token -cne $token -or $result.executable -cne $Executable -or $result.workerId -ne $process.Id) {
            throw "Synthetic process completion is not owned by this request"
        }
        foreach ($field in @("workerId", "childId", "peakWorkingSetBytes", "finalPeakWorkingSetBytes", "elapsedMilliseconds", "capturedBytes")) {
            if (($result.$field -isnot [int] -and $result.$field -isnot [long]) -or $result.$field -lt 0) {
                throw "Synthetic completion has an invalid numeric field"
            }
        }
        if ((Get-Item -LiteralPath $stdoutPath).Length + (Get-Item -LiteralPath $stderrPath).Length -ne $result.capturedBytes -or $result.capturedBytes -gt $MaximumOutputBytes) {
            throw "Synthetic output evidence exceeds or disagrees with its byte budget"
        }
        if ($exitCode -ne 0 -or $result.status -cne "complete" -or $result.exitCode -ne 0 -or
            $result.peakWorkingSetBytes -le 0 -or $result.peakWorkingSetBytes -gt $MaximumBytes -or
            $result.finalPeakWorkingSetBytes -le 0 -or $result.finalPeakWorkingSetBytes -gt $result.peakWorkingSetBytes -or
            $result.elapsedMilliseconds -gt ($Seconds * 1000)) {
            throw "Synthetic process failed: $($result.failure)"
        }
        return [pscustomobject]@{
            Output = [IO.File]::ReadAllText($stdoutPath) + "`n" + [IO.File]::ReadAllText($stderrPath)
            StandardOutput = [IO.File]::ReadAllText($stdoutPath)
            Evidence = $result
        }
    } finally {
        $job.Dispose()
        if ($null -ne $process) {
            try { if (-not $process.WaitForExit(5000)) { throw "Owned synthetic process did not terminate" } }
            finally { $process.Dispose() }
        }
    }
}
