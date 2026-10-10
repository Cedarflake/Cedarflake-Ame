. (Join-Path $PSScriptRoot "performance_test_synthetic_protocol.ps1")

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
    $runFailure = $null
    Assert-Refused { & $finallyBody } "WriteAllText|denied|directory"
    if (-not $script:syntheticLockReleased) { throw "Summary failure skipped tool-lock release" }
    $script:syntheticLockReleased = $false
    try {
        try { throw "original-synthetic-workload-failure" }
        catch { $runFailure = $_; throw }
        finally { & $finallyBody }
    } catch {
        if (-not [object]::ReferenceEquals($_.Exception, $runFailure.Exception) -or
            $_.Exception.Message -cne "original-synthetic-workload-failure") {
            throw "Summary persistence replaced the original workload failure"
        }
    }
    if (-not $script:syntheticLockReleased) { throw "Workload and summary failure skipped tool-lock release" }
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
Write-Host "AME_SYNTHETIC_GUARDRAILS exact_allowlist=5 locked_build=passed artifact_tamper=passed list_and_run_tamper=passed owned_success=passed fast_exit_peak=passed preparation=passed nonzero=passed timeout=passed memory=passed output=passed cleanup=passed"
