param(
    [Parameter(Mandatory = $true)]
    [string]$RootPath,
    [Parameter(Mandatory = $true)]
    [string]$StorageRoot,
    [Parameter(Mandatory = $true)]
    [ValidatePattern("^[A-Za-z0-9][A-Za-z0-9_-]{0,63}$")]
    [string]$ScanId,
    [Parameter(Mandatory = $true)]
    [string]$AuthorizationToken,
    [UInt64]$CancelAfter,
    [UInt64]$PauseAfter,
    [switch]$UseExistingStorage,
    [switch]$AllowCloudBackedRoot
)

$ErrorActionPreference = "Stop"
$requiredToken = "CEDARFLAKE_AME_READ_ONLY_ACCEPTANCE_V1"
if ($AuthorizationToken -cne $requiredToken) {
    throw "The exact current read-only acceptance authorization token is required"
}
if ($CancelAfter -gt 0 -and $PauseAfter -gt 0) {
    throw "CancelAfter and PauseAfter cannot be combined"
}

$resolvedRoot = [System.IO.Path]::GetFullPath($RootPath)
if (-not (Test-Path -LiteralPath $resolvedRoot -PathType Container)) {
    throw "The acceptance root is not an available directory: $resolvedRoot"
}
if (
    $resolvedRoot.IndexOf("OneDrive", [System.StringComparison]::OrdinalIgnoreCase) -ge 0 -and
    -not $AllowCloudBackedRoot
) {
    throw "A cloud-backed root requires the explicit AllowCloudBackedRoot switch"
}

$resolvedStorage = [System.IO.Path]::GetFullPath($StorageRoot)
$normalize = {
    param([string]$Path)
    return $Path.Replace("/", "\").TrimEnd("\").ToLowerInvariant()
}
$normalizedRoot = & $normalize $resolvedRoot
$normalizedStorage = & $normalize $resolvedStorage
if (
    $normalizedRoot -eq $normalizedStorage -or
    $normalizedRoot.StartsWith("$normalizedStorage\") -or
    $normalizedStorage.StartsWith("$normalizedRoot\")
) {
    throw "Acceptance storage must remain outside the source root"
}

if (Test-Path -LiteralPath $resolvedStorage) {
    $hasExistingContent = Get-ChildItem -LiteralPath $resolvedStorage -Force | Select-Object -First 1
    if ($hasExistingContent -and -not $UseExistingStorage) {
        throw "Acceptance storage is not empty; use a new path or explicitly allow existing storage"
    }
} else {
    New-Item -ItemType Directory -Path $resolvedStorage -Force | Out-Null
}

$repositoryRoot = Split-Path -Parent $PSScriptRoot
$cargo = (Get-Command cargo -ErrorAction Stop).Source
$startedAt = Get-Date
$processInfo = [System.Diagnostics.ProcessStartInfo]::new()
$processInfo.FileName = $cargo
$processInfo.Arguments = (
    "test --manifest-path rust\Cargo.toml " +
    "user_authorized_read_only_library_acceptance " +
    "-- --ignored --nocapture --test-threads=1"
)
$processInfo.WorkingDirectory = $repositoryRoot
$processInfo.UseShellExecute = $false

$environmentNames = @(
    "CEDARFLAKE_AME_ACCEPTANCE_CONSENT",
    "CEDARFLAKE_AME_ACCEPTANCE_ROOT",
    "CEDARFLAKE_AME_ACCEPTANCE_STORAGE_ROOT",
    "CEDARFLAKE_AME_ACCEPTANCE_SCAN_ID",
    "CEDARFLAKE_AME_ACCEPTANCE_REPORT",
    "CEDARFLAKE_AME_ACCEPTANCE_CANCEL_AFTER",
    "CEDARFLAKE_AME_ACCEPTANCE_PAUSE_AFTER"
)
$previousEnvironment = @{}
foreach ($name in $environmentNames) {
    $previousEnvironment[$name] = [System.Environment]::GetEnvironmentVariable($name, "Process")
}

$env:CEDARFLAKE_AME_ACCEPTANCE_CONSENT = $requiredToken
$env:CEDARFLAKE_AME_ACCEPTANCE_ROOT = $resolvedRoot
$env:CEDARFLAKE_AME_ACCEPTANCE_STORAGE_ROOT = $resolvedStorage
$env:CEDARFLAKE_AME_ACCEPTANCE_SCAN_ID = $ScanId
$reportPath = Join-Path $resolvedStorage "acceptance-$ScanId.log"
$env:CEDARFLAKE_AME_ACCEPTANCE_REPORT = $reportPath
$env:CEDARFLAKE_AME_ACCEPTANCE_CANCEL_AFTER = if ($CancelAfter -gt 0) {
    $CancelAfter.ToString()
} else {
    $null
}
$env:CEDARFLAKE_AME_ACCEPTANCE_PAUSE_AFTER = if ($PauseAfter -gt 0) {
    $PauseAfter.ToString()
} else {
    $null
}

$process = [System.Diagnostics.Process]::new()
$process.StartInfo = $processInfo
$peakWorkingSetBytes = [UInt64]0
$hasStarted = $false
try {
    if (-not $process.Start()) {
        throw "Could not start the read-only acceptance process"
    }
    $hasStarted = $true
    while (-not $process.HasExited) {
        $testProcesses = Get-Process -Name "rust_lib_cedarflake_ame-*" `
            -ErrorAction SilentlyContinue | Where-Object { $_.StartTime -ge $startedAt }
        foreach ($testProcess in $testProcesses) {
            $workingSetBytes = [UInt64]$testProcess.WorkingSet64
            if ($workingSetBytes -gt $peakWorkingSetBytes) {
                $peakWorkingSetBytes = $workingSetBytes
            }
        }
        Start-Sleep -Milliseconds 5
    }
    $process.WaitForExit()
} finally {
    if ($hasStarted -and -not $process.HasExited) {
        Get-Process -Name "rust_lib_cedarflake_ame-*" `
            -ErrorAction SilentlyContinue | Where-Object {
                $_.StartTime -ge $startedAt
            } | Stop-Process -Force -ErrorAction SilentlyContinue
        $process.Kill()
        $process.WaitForExit()
    }
    foreach ($name in $environmentNames) {
        [System.Environment]::SetEnvironmentVariable(
            $name,
            $previousEnvironment[$name],
            "Process"
        )
    }
}

$wasMemoryObserved = $peakWorkingSetBytes -gt 0
$memoryLine = (
    "AME_REAL_LIBRARY_MEMORY peak_working_set_bytes=$peakWorkingSetBytes " +
    "observed=$($wasMemoryObserved.ToString().ToLowerInvariant())"
)
[System.IO.File]::AppendAllText(
    $reportPath,
    "$memoryLine$([Environment]::NewLine)",
    [System.Text.UTF8Encoding]::new($false)
)
$report = [System.IO.File]::ReadAllText($reportPath)
Write-Output $report.TrimEnd()
Write-Output "AME_REAL_LIBRARY_REPORT path=$reportPath"
if ($process.ExitCode -ne 0) {
    throw "Read-only acceptance failed with exit code $($process.ExitCode)"
}
