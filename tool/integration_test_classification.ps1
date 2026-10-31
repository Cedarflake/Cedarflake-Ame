[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)] [string]$ModelDirectory,
    [Parameter(Mandatory = $true)] [string]$RuntimeDirectory,
    [Parameter(Mandatory = $true)] [string]$PythonPath,
    [string]$FixturePath = "build/test/metadata"
)

$ErrorActionPreference = "Stop"
. (Join-Path $PSScriptRoot "quality_common.ps1")
. (Join-Path $PSScriptRoot "integration_windows_scan_run.ps1")
$repositoryRoot = Get-AmeRepositoryRoot
$toolchain = Get-AmeToolchain
$environment = @{}
$job = $null
$process = $null
$toolLock = Enter-AmeRepositoryToolLock
try {
    $fixture = (Resolve-Path -LiteralPath $FixturePath).Path
    $fixtureParent = Join-Path $repositoryRoot "build\test\"
    if (-not $fixture.StartsWith($fixtureParent, [StringComparison]::OrdinalIgnoreCase)) {
        throw "Classification client verification requires existing build/test fixtures"
    }
    Assert-AmeWindowsScanDirectoryChain $fixture
    $storage = New-AmeWindowsScanStorage $repositoryRoot ([Guid]::NewGuid().ToString("N"))
    $variables = @{
        CEDARFLAKE_AME_TEST_STORAGE_ROOT = $storage
        CEDARFLAKE_AME_CLASSIFICATION_FIXTURE = $fixture
        CEDARFLAKE_AME_CLASSIFICATION_MODELS = (Resolve-Path -LiteralPath $ModelDirectory).Path
        CEDARFLAKE_AME_CLASSIFICATION_RUNTIME = (Resolve-Path -LiteralPath $RuntimeDirectory).Path
        CEDARFLAKE_AME_CLASSIFICATION_PYTHON = (Resolve-Path -LiteralPath $PythonPath).Path
        CARGO_BUILD_JOBS = "1"
    }
    foreach ($entry in $variables.GetEnumerator()) {
        $environment[$entry.Key] = [Environment]::GetEnvironmentVariable($entry.Key, "Process")
        [Environment]::SetEnvironmentVariable($entry.Key, $entry.Value, "Process")
    }
    $stdout = Join-Path $storage "classification.stdout.log"
    $stderr = Join-Path $storage "classification.stderr.log"
    foreach ($path in @($toolchain.Flutter, $stdout, $stderr)) {
        if ($path.IndexOfAny([char[]]@('"', '%', "`r", "`n", [char]0)) -ge 0) {
            throw "Classification command paths cannot be quoted safely"
        }
    }
    $utf8 = [Text.UTF8Encoding]::new($false)
    [IO.File]::WriteAllText($stdout, "", $utf8)
    [IO.File]::WriteAllText($stderr, "", $utf8)
    Initialize-AmeWindowsAccessibilityProcessJob
    $job = [AmeWindowsAccessibilityProcessJob]::new()
    $command = ('"{0}" test "integration_test\classification_runtime_test.dart" ' +
        '-d windows --no-pub --reporter expanded 1>"{1}" 2>"{2}"') -f $toolchain.Flutter, $stdout, $stderr
    $cmd = Join-Path ([Environment]::GetFolderPath("System")) "cmd.exe"
    $clock = [Diagnostics.Stopwatch]::StartNew()
    Write-Output "Classification client evidence: $storage"
    $process = $job.Start($cmd, "/d /v:off /s /c `"$command`"", $repositoryRoot)
    $exitCode = Wait-AmeWindowsScanProcess $process $job $clock 1800
    Get-Content -LiteralPath $stdout -Encoding UTF8
    Get-Content -LiteralPath $stderr -Encoding UTF8
    if ($exitCode -ne 0) { throw "Classification client verification failed: $exitCode" }
} finally {
    try {
        if ($null -ne $job) { $job.Dispose() }
        if ($null -ne $process) {
            try {
                if (-not $process.WaitForExit(5000)) {
                    throw "Classification client did not exit after its owned job closed"
                }
            } finally { $process.Dispose() }
        }
    } finally {
        foreach ($entry in $environment.GetEnumerator()) {
            [Environment]::SetEnvironmentVariable($entry.Key, $entry.Value, "Process")
        }
        Exit-AmeRepositoryToolLock $toolLock
    }
}
