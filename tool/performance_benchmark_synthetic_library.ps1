param(
    [UInt64]$MaxPeakWorkingSetBytes = 536870912
)

$ErrorActionPreference = "Stop"
. (Join-Path $PSScriptRoot "quality_common.ps1")

$repositoryRoot = Get-AmeRepositoryRoot
$cargo = (Get-AmeToolchain).Cargo
$startedAt = Get-Date
$processInfo = [System.Diagnostics.ProcessStartInfo]::new()
$processInfo.FileName = $cargo
$processInfo.Arguments = (
    "test --manifest-path rust\Cargo.toml " +
    "synthetic_ten_thousand_file_scan_records_bounded_acceptance_evidence " +
    "-- --ignored --nocapture"
)
$processInfo.WorkingDirectory = $repositoryRoot
$processInfo.UseShellExecute = $false
$processInfo.RedirectStandardOutput = $true
$processInfo.RedirectStandardError = $true

$process = [System.Diagnostics.Process]::new()
$process.StartInfo = $processInfo
if (-not $process.Start()) {
    throw "Could not start the synthetic benchmark"
}

$standardOutput = $process.StandardOutput.ReadToEndAsync()
$standardError = $process.StandardError.ReadToEndAsync()
$peakWorkingSetBytes = [UInt64]0

while (-not $process.HasExited) {
    $testProcesses = Get-Process -Name "rust_lib_cedarflake_ame-*" `
        -ErrorAction SilentlyContinue | Where-Object { $_.StartTime -ge $startedAt }
    foreach ($testProcess in $testProcesses) {
        $workingSetBytes = [UInt64]$testProcess.WorkingSet64
        if ($workingSetBytes -gt $peakWorkingSetBytes) {
            $peakWorkingSetBytes = $workingSetBytes
        }
    }
    Start-Sleep -Milliseconds 100
}

$process.WaitForExit()
$stdout = $standardOutput.GetAwaiter().GetResult()
$stderr = $standardError.GetAwaiter().GetResult()
if ($stdout) {
    Write-Output $stdout.TrimEnd()
}
if ($stderr) {
    Write-Output $stderr.TrimEnd()
}
Write-Output (
    "AME_SYNTHETIC_MEMORY peak_working_set_bytes=$peakWorkingSetBytes " +
    "limit_bytes=$MaxPeakWorkingSetBytes"
)

if ($process.ExitCode -ne 0) {
    throw "Synthetic benchmark failed with exit code $($process.ExitCode)"
}
if ($peakWorkingSetBytes -eq 0) {
    throw "Synthetic benchmark process memory was not observed"
}
if ($peakWorkingSetBytes -gt $MaxPeakWorkingSetBytes) {
    throw "Synthetic benchmark exceeded the working-set limit"
}
