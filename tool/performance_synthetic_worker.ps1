param([Parameter(Mandatory = $true)] [string]$RequestPath)

$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest
$request = Get-Content -LiteralPath $RequestPath -Raw -Encoding UTF8 | ConvertFrom-Json
$elapsed = [Diagnostics.Stopwatch]::new()
$process = [Diagnostics.Process]::new()
$peak = 0L
$finalPeak = 0L
$childId = 0
$exitCode = $null
$failure = $null
$captured = 0L
$outputFiles = @()
try {
    $outputFiles += [IO.File]::Open($request.stdout, [IO.FileMode]::CreateNew, [IO.FileAccess]::Write, [IO.FileShare]::Read)
    $outputFiles += [IO.File]::Open($request.stderr, [IO.FileMode]::CreateNew, [IO.FileAccess]::Write, [IO.FileShare]::Read)
    $process.StartInfo.FileName = $request.executable
    $process.StartInfo.WorkingDirectory = $request.repository
    $process.StartInfo.UseShellExecute = $false
    $process.StartInfo.CreateNoWindow = $true
    $process.StartInfo.RedirectStandardOutput = $true
    $process.StartInfo.RedirectStandardError = $true
    $quoted = foreach ($argument in $request.arguments) {
        if ($argument.Contains('"') -or $argument.Contains("`0") -or $argument.EndsWith('\')) { throw "Unsupported synthetic argument quoting" }
        '"' + $argument + '"'
    }
    $process.StartInfo.Arguments = $quoted -join ' '
    # This fresh worker owns its environment; children inherit only these process-local overrides.
    [Environment]::SetEnvironmentVariable("TEMP", $request.scratch)
    [Environment]::SetEnvironmentVariable("TMP", $request.scratch)
    [Environment]::SetEnvironmentVariable("CARGO_TARGET_DIR", (Join-Path $request.repository "rust\target"))
    [Environment]::SetEnvironmentVariable("RUST_TEST_THREADS", "1")
    . (Join-Path $PSScriptRoot "performance_synthetic_memory.ps1")
    Initialize-AmeSyntheticMemoryQuery
    $elapsed.Start()
    if (-not $process.Start()) { throw "Could not start synthetic child" }
    $childId = $process.Id
    $retainedHandle = $process.Handle
    $sources = @($process.StandardOutput.BaseStream, $process.StandardError.BaseStream)
    $buffers = @([byte[]]::new(8192), [byte[]]::new(8192))
    $reads = @($sources[0].ReadAsync($buffers[0], 0, 8192), $sources[1].ReadAsync($buffers[1], 0, 8192))
    while ($null -ne $reads[0] -or $null -ne $reads[1] -or -not $process.HasExited) {
        for ($index = 0; $index -lt 2; $index++) {
            if ($null -ne $reads[$index] -and $reads[$index].IsCompleted) {
                $count = $reads[$index].GetAwaiter().GetResult()
                if ($count -eq 0) { $reads[$index] = $null }
                else {
                    if ($captured + $count -gt $request.maximumOutputBytes) { throw "Synthetic output exceeded its byte budget" }
                    $outputFiles[$index].Write($buffers[$index], 0, $count)
                    $captured += $count
                    $reads[$index] = $sources[$index].ReadAsync($buffers[$index], 0, 8192)
                }
            }
        }
        $peak = [Math]::Max($peak, [AmeSyntheticMemoryQuery]::ReadPeakWorkingSet($retainedHandle))
        if ($peak -gt $request.maximumBytes) { throw "Synthetic child exceeded its peak working-set budget" }
        if ($elapsed.Elapsed.TotalSeconds -gt $request.seconds) { throw "Synthetic child exceeded its wall-clock budget" }
        [Threading.Thread]::Sleep(10)
    }
    $process.WaitForExit()
    $finalPeak = [AmeSyntheticMemoryQuery]::ReadPeakWorkingSet($retainedHandle)
    $peak = [Math]::Max($peak, $finalPeak)
    if ($peak -gt $request.maximumBytes) { throw "Synthetic child exceeded its peak working-set budget" }
    $exitCode = $process.ExitCode
    if ($exitCode -ne 0) { throw "Synthetic child exited with code $exitCode" }
    if ($peak -eq 0) { throw "Synthetic child memory evidence is missing" }
} catch {
    $failure = $_.Exception.Message
} finally {
    foreach ($stream in $outputFiles) { $stream.Dispose() }
    $elapsed.Stop()
    $result = [ordered]@{
        token = $request.token; executable = $request.executable; workerId = $PID; childId = $childId
        status = $(if ($null -eq $failure) { "complete" } else { "failed" })
        exitCode = $exitCode; elapsedMilliseconds = $elapsed.ElapsedMilliseconds
        peakWorkingSetBytes = $peak; memoryScope = "direct-child-kernel-peak-working-set"
        finalPeakWorkingSetBytes = $finalPeak
        maximumBytes = $request.maximumBytes; maximumOutputBytes = $request.maximumOutputBytes
        capturedBytes = $captured
        failure = $failure
    }
    [IO.File]::WriteAllText($request.result, ($result | ConvertTo-Json -Depth 4), [Text.UTF8Encoding]::new($false))
    $process.Dispose()
}
if ($null -ne $failure) { exit 1 }
