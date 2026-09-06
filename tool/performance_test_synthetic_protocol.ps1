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
$scanMarker = "AME_SYNTHETIC_BENCHMARK files=10000 fixture_ms=923 cold_ms=10107 warm_ms=8300 pause_ms=2 resume_ms=10065 cancel_ms=86 catalog_bytes=55885824 resumed_catalog_bytes=32149504"
$markers = @(
    "full_decode_resize_ms=12.5 scaled_decode_resize_ms=3.2 speedup=3.90",
    $scanMarker,
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

$scanName = $cases[1].Test
$scanOutput = "`nrunning 1 test`ntest $scanName ... $scanMarker`nok`n`ntest result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 1166 filtered out; finished in 30.91s`n`n"
foreach ($output in @($scanOutput, $scanOutput.Replace("`n", "`r`n"))) {
    Assert-AmeSyntheticResult -Output $output -TestName $scanName
}
foreach ($field in @("catalog_bytes", "resumed_catalog_bytes")) {
    $token = [regex]::Match($scanMarker, " ${field}=[0-9]+").Value
    if (-not $token) { throw "The scan fixture must contain $field" }
    $invalidMarkers = @(
        $scanMarker.Replace($token, ""),
        $scanMarker.Replace($token, "$token$token")
    )
    foreach ($value in @("", "-1", "+1", "1.5", "1e3", "NaN", " 1", "1 ", [string][char]0x0661)) {
        $invalidMarkers += $scanMarker.Replace($token, " ${field}=$value")
    }
    foreach ($invalid in $invalidMarkers) {
        Assert-Refused { Assert-AmeSyntheticResult -Output $scanOutput.Replace($scanMarker, $invalid) -TestName $scanName } "exact workload or evidence"
    }
}
foreach ($invalid in @(
    "$scanMarker extra_bytes=1",
    "$scanMarker`n$scanMarker",
    $scanMarker.Replace("catalog_bytes=55885824 resumed_catalog_bytes=32149504", "resumed_catalog_bytes=32149504 catalog_bytes=55885824")
)) {
    Assert-Refused { Assert-AmeSyntheticResult -Output $scanOutput.Replace($scanMarker, $invalid) -TestName $scanName } "exact workload or evidence"
}
Write-Host "AME_SYNTHETIC_PROTOCOL exact_allowlist=3 locked_build=passed list_and_run_tamper=passed scan_bytes_schema=passed"
