param([ValidateSet("all", "jpeg", "scan", "usn", "media", "publication")] [string]$Case = "all")

$ErrorActionPreference = "Stop"
. (Join-Path $PSScriptRoot "performance_synthetic_common.ps1")
if (-not (Test-AmeSyntheticPlatform -Platform ([Environment]::OSVersion.Platform) `
    -Architecture ([Runtime.InteropServices.RuntimeInformation]::OSArchitecture) -Is64BitProcess ([Environment]::Is64BitProcess))) {
    throw "Synthetic performance execution requires Windows x64; it does not replace Windows 11 client acceptance"
}
$repository = Get-AmeRepositoryRoot
$cargo = Resolve-AmeExecutable -Name "cargo" -FallbackPath (Join-Path $env:USERPROFILE ".cargo\bin\cargo.exe")
$lock = Enter-AmeRepositoryToolLock
$directory = $null
$runFailure = $null
$summary = [ordered]@{ format = "ame-synthetic-performance-v1"; status = "running"; cases = @(); failure = $null }
try {
    $directory = New-AmeSyntheticEvidenceDirectory -RepositoryRoot $repository
    Write-Host "Synthetic performance evidence: $directory"
    $build = Invoke-AmeSyntheticOwnedProcess -Executable $cargo -Arguments (Get-AmeSyntheticBuildArguments) `
        -RepositoryRoot $repository -Directory $directory -Name "build" -Seconds 1800 -MaximumBytes 8589934592
    $summary.build = $build.Evidence
    $summary.buildMemoryScope = "cargo-primary-only-not-compiler-tree"
    $executable = Get-AmeSyntheticTestExecutable -Output $build.StandardOutput -RepositoryRoot $repository
    $summary.executableSha256 = (Get-FileHash -LiteralPath $executable -Algorithm SHA256).Hash
    $selected = @(Get-AmeSyntheticCases | Where-Object { $Case -eq "all" -or $_.Name -eq $Case })
    foreach ($workload in $selected) {
        $list = Invoke-AmeSyntheticOwnedProcess -Executable $executable -Arguments @(
            "--exact", $workload.Test, "--ignored", "--list"
        ) -RepositoryRoot $repository -Directory $directory -Name ($workload.Name + "-list") -Seconds 30 -MaximumBytes $workload.Bytes
        Assert-AmeSyntheticList -Output $list.StandardOutput -TestName $workload.Test
        $run = Invoke-AmeSyntheticOwnedProcess -Executable $executable -Arguments @(
            "--exact", $workload.Test, "--ignored", "--nocapture", "--test-threads=1"
        ) -RepositoryRoot $repository -Directory $directory -Name $workload.Name -Seconds $workload.Seconds -MaximumBytes $workload.Bytes
        Assert-AmeSyntheticResult -Output $run.Output -TestName $workload.Test
        $summary.cases += [ordered]@{ name = $workload.Name; test = $workload.Test; passed = 1; ignored = 0; resource = $run.Evidence }
        Write-Host $run.Output.TrimEnd()
    }
    if ($summary.cases.Count -ne $selected.Count) { throw "Synthetic workload coverage is incomplete" }
    $summary.status = "complete"
} catch {
    $runFailure = $_
    $summary.status = "failed"
    $summary.failure = $_.Exception.Message
    throw
} finally {
    try {
        if ($null -ne $directory -and (Test-Path -LiteralPath $directory -PathType Container)) {
            [IO.File]::WriteAllText((Join-Path $directory "summary.json"), ($summary | ConvertTo-Json -Depth 8), [Text.UTF8Encoding]::new($false))
        }
    } catch {
        if ($null -eq $runFailure) { throw }
        Write-Warning -Message ("Synthetic summary persistence failed after the workload failure: " + $_.Exception.Message) -WarningAction Continue
    } finally { Exit-AmeRepositoryToolLock -Mutex $lock }
}
