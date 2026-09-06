$ErrorActionPreference = "Stop"
. (Join-Path $PSScriptRoot "performance_synthetic_common.ps1")

function Assert-Refused {
    param([scriptblock]$Action, [string]$Reason)
    try { & $Action } catch {
        if ($_.Exception.Message -notmatch $Reason) { throw }
        return
    }
    throw "Expected synthetic guardrail refusal: $Reason"
}

$cases = @(Get-AmeSyntheticCases)
if (-not (Test-AmeSyntheticPlatform -Platform Win32NT -Architecture X64 -Is64BitProcess $true) -or
    (Test-AmeSyntheticPlatform -Platform Win32NT -Architecture Arm64 -Is64BitProcess $true) -or
    (Test-AmeSyntheticPlatform -Platform Win32NT -Architecture X64 -Is64BitProcess $false) -or
    (Test-AmeSyntheticPlatform -Platform Unix -Architecture X64 -Is64BitProcess $true)) {
    throw "Synthetic admission must require native Windows x64"
}
if (((Get-AmeSyntheticBuildArguments) -join ' ') -cne 'test --locked --manifest-path rust/Cargo.toml --release --lib --all-features --jobs 1 --no-run --message-format=json') {
    throw "Synthetic builds must use the exact locked serial release library-test command"
}
if (($cases.Name -join ',') -cne "jpeg,scan,usn" -or @($cases.Test | Select-Object -Unique).Count -ne 3) {
    throw "The synthetic allowlist must contain the exact three independent workloads"
}
$markers = @(
    "full_decode_resize_ms=12.5 scaled_decode_resize_ms=3.2 speedup=3.90",
    "AME_SYNTHETIC_BENCHMARK files=10000 fixture_ms=1 cold_ms=2 warm_ms=3 pause_ms=4 resume_ms=5 cancel_ms=6 catalog_bytes=1000",
    "R2c-R million-backlog records=1000000 covered_records=1000000 pages=245 production_records_per_native_buffer=4095 retained_frames_max=1 root_scope_checks=1000000 emitted_candidates=0 elapsed_ms=3"
)
for ($index = 0; $index -lt $cases.Count; $index++) {
    $name = $cases[$index].Test
    $list = "${name}: test`n`n1 test, 0 benchmarks`n"
    Assert-AmeSyntheticList -Output $list -TestName $name
    Assert-Refused { Assert-AmeSyntheticList -Output "0 tests, 0 benchmarks" -TestName $name } "exactly one"
    Assert-Refused { Assert-AmeSyntheticList -Output ($list + $list) -TestName $name } "exactly one"
    $output = "running 1 test`ntest $name ... $($markers[$index])`nok`n`ntest result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 1160 filtered out; finished in 0.12s`n"
    Assert-AmeSyntheticResult -Output $output -TestName $name
    foreach ($invalid in @(
        $output.Replace("1 passed", "0 passed"),
        $output.Replace("0 ignored", "1 ignored"),
        $output.Replace($markers[$index], ""),
        ($output + $output),
        $output.Replace($name, "user_authorized_read_only_library_acceptance")
    )) {
        Assert-Refused { Assert-AmeSyntheticResult -Output $invalid -TestName $name } "Synthetic execution"
    }
}
Assert-Refused { Assert-AmeSyntheticList -Output "" -TestName "user_authorized_read_only_library_acceptance" } "not allowlisted"
Assert-Refused { & (Join-Path $PSScriptRoot "performance_run_synthetic.ps1") -Case "--include-ignored" } "ValidateSet|validation|argument"

$repository = Get-AmeRepositoryRoot
$directory = New-AmeSyntheticEvidenceDirectory -RepositoryRoot $repository -Category "performance_synthetic_guardrails"
$hostExecutable = (Get-Process -Id $PID).Path
$artifactRoot = Join-Path $directory "rust\target\release\deps"
$null = New-Item -ItemType Directory -Path $artifactRoot
$artifactPath = Join-Path $artifactRoot "rust_lib_cedarflake_ame-0123456789abcdef.exe"
[IO.File]::WriteAllBytes($artifactPath, [byte[]]@(0))
$artifact = [ordered]@{
    reason = "compiler-artifact"; target = @{ name = "rust_lib_cedarflake_ame" }; profile = @{ test = $true }
    manifest_path = Join-Path $directory "rust\Cargo.toml"; executable = $artifactPath
}
$artifactJson = $artifact | ConvertTo-Json -Depth 4 -Compress
if ((Get-AmeSyntheticTestExecutable -Output $artifactJson -RepositoryRoot $directory) -cne $artifactPath) {
    throw "The exact cargo release artifact was not accepted"
}
Assert-Refused { Get-AmeSyntheticTestExecutable -Output ($artifactJson + "`n" + $artifactJson) -RepositoryRoot $directory } "exactly one"
$artifact.manifest_path = Join-Path $directory "other\Cargo.toml"
Assert-Refused { Get-AmeSyntheticTestExecutable -Output ($artifact | ConvertTo-Json -Depth 4 -Compress) -RepositoryRoot $directory } "unexpected manifest"
$artifact.manifest_path = Join-Path $directory "rust\Cargo.toml"
$artifact.executable = $hostExecutable
Assert-Refused { Get-AmeSyntheticTestExecutable -Output ($artifact | ConvertTo-Json -Depth 4 -Compress) -RepositoryRoot $directory } "expected release"
function Invoke-Fixture {
    param([string]$Name, [string]$Code, [int]$Seconds = 5, [long]$Bytes = 536870912, [int]$OutputBytes = 4096)
    $encoded = [Convert]::ToBase64String([Text.Encoding]::Unicode.GetBytes($Code))
    Invoke-AmeSyntheticOwnedProcess -Executable $hostExecutable -Arguments @("-NoLogo", "-NoProfile", "-NonInteractive", "-EncodedCommand", $encoded) `
        -RepositoryRoot $repository -Directory $directory -Name $Name -Seconds $Seconds -MaximumBytes $Bytes -MaximumOutputBytes $OutputBytes
}

$success = Invoke-Fixture -Name "success" -Code "Write-Output 'owned-success'; Start-Sleep -Milliseconds 100"
if ($success.StandardOutput.Trim() -cne "owned-success" -or $success.Evidence.peakWorkingSetBytes -le 0) {
    throw "Owned process success and memory evidence were not observed"
}
$fast = Invoke-AmeSyntheticOwnedProcess -Executable (Join-Path $env:SystemRoot "System32\where.exe") `
    -Arguments @("/q", "cmd.exe") -RepositoryRoot $repository -Directory $directory `
    -Name "fast-exit" -Seconds 5 -MaximumBytes 536870912
if ($fast.Evidence.finalPeakWorkingSetBytes -le 0) { throw "Fast exit lost its final kernel memory peak" }
. (Join-Path $PSScriptRoot "performance_synthetic_memory.ps1")
Initialize-AmeSyntheticMemoryQuery
Assert-Refused { [AmeSyntheticMemoryQuery]::ReadPeakWorkingSet([IntPtr]::Zero) } "retained child process handle"
Assert-Refused { [AmeSyntheticMemoryQuery]::ReadPeakWorkingSet([IntPtr]::new(1234)) } "Child memory query failed"
Assert-Refused {
    Invoke-AmeSyntheticOwnedProcess -Executable $hostExecutable -Arguments @('"') -RepositoryRoot $repository `
        -Directory $directory -Name "preparation" -Seconds 5 -MaximumBytes 536870912
} "argument quoting"
$preparation = Get-Content -LiteralPath (Join-Path $directory "preparation.json") -Raw -Encoding UTF8 | ConvertFrom-Json
if ($preparation.childId -ne 0 -or $preparation.status -cne "failed") { throw "Preparation failure must not start a child" }

$runnerPath = Join-Path $PSScriptRoot "performance_run_synthetic.ps1"
$runnerAst = [Management.Automation.Language.Parser]::ParseFile($runnerPath, [ref]$null, [ref]$null)
$runnerTry = @($runnerAst.EndBlock.Statements | Where-Object { $_ -is [Management.Automation.Language.TryStatementAst] })
if ($runnerTry.Count -ne 1) { throw "The runner must retain one outer resource-owner scope" }
$finallyText = $runnerTry[0].Finally.Extent.Text
$finallyBody = [scriptblock]::Create($finallyText.Substring(1, $finallyText.Length - 2))
$null = New-Item -ItemType Directory -Path (Join-Path $directory "summary.json")
& {
    $script:syntheticLockReleased = $false
    function Exit-AmeRepositoryToolLock { param($Mutex) $script:syntheticLockReleased = $true }
    $lock = $null
    $summary = @{}
    Assert-Refused { & $finallyBody } "WriteAllText|denied|directory"
    if (-not $script:syntheticLockReleased) { throw "Summary failure skipped tool-lock release" }
}
Assert-Refused { Invoke-Fixture -Name "nonzero" -Code "exit 7" } "exited with code 7"
Assert-Refused { Invoke-Fixture -Name "timeout" -Code "Start-Sleep -Seconds 60" -Seconds 1 } "wall-clock budget"
Assert-Refused { Invoke-Fixture -Name "memory" -Code "Start-Sleep -Seconds 60" -Bytes 1 } "working-set budget"
Assert-Refused { Invoke-Fixture -Name "output" -Code "[Console]::Write(('x' * 20000)); Start-Sleep -Milliseconds 100" -OutputBytes 1024 } "byte budget"
foreach ($name in @("timeout", "memory", "output")) {
    $record = Get-Content -LiteralPath (Join-Path $directory "$name.json") -Raw -Encoding UTF8 | ConvertFrom-Json
    try { $child = Get-Process -Id $record.childId -ErrorAction Stop } catch [Microsoft.PowerShell.Commands.ProcessCommandException] { continue }
    try { if (-not $child.WaitForExit(5000)) { throw "Owned child survived the Job Object cleanup" } }
    finally { $child.Dispose() }
}
Write-Host "AME_SYNTHETIC_GUARDRAILS exact_allowlist=3 locked_build=passed artifact_tamper=passed list_and_run_tamper=passed owned_success=passed fast_exit_peak=passed preparation=passed nonzero=passed timeout=passed memory=passed output=passed cleanup=passed"
