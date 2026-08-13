[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [string]$RootPath,
    [Parameter(Mandatory = $true)]
    [string]$SourceCatalogPath,
    [Parameter(Mandatory = $true)]
    [string]$StorageRoot,
    [Parameter(Mandatory = $true)]
    [string]$AuthorizationToken,
    [ValidateRange(1, 512)]
    [int]$MaxItems = 512,
    [ValidateRange(1, 512)]
    [int]$MinSuccessfulItems = 24,
    [ValidateRange(67108864, 1073741824)]
    [UInt64]$CacheBudgetBytes = 67108864,
    [ValidateRange(1048576, 268435456)]
    [UInt64]$MaxSourceFileBytes = 67108864,
    [ValidateRange(60, 1800)]
    [int]$TimeLimitSeconds = 900,
    [ValidateRange(268435456, 4294967296)]
    [UInt64]$MemoryLimitBytes = 1073741824,
    [switch]$ValidationOnly
)

$ErrorActionPreference = "Stop"
. (Join-Path $PSScriptRoot "quality_common.ps1")

$requiredToken = "CEDARFLAKE_AME_PREVIEW_PERFORMANCE_ACCEPTANCE_V1"
if ($AuthorizationToken -cne $requiredToken) {
    throw "The exact current preview performance acceptance authorization token is required"
}
if ($MinSuccessfulItems -gt $MaxItems) {
    throw "MinSuccessfulItems cannot exceed MaxItems"
}

$resolvedRoot = [System.IO.Path]::GetFullPath($RootPath)
$resolvedCatalog = [System.IO.Path]::GetFullPath($SourceCatalogPath)
$resolvedStorage = [System.IO.Path]::GetFullPath($StorageRoot)
if (-not (Test-Path -LiteralPath $resolvedRoot -PathType Container)) {
    throw "The acceptance root is not an available directory"
}
if (-not (Test-Path -LiteralPath $resolvedCatalog -PathType Leaf)) {
    throw "The source catalog is not an available file"
}
if ($resolvedRoot.IndexOf("OneDrive", [System.StringComparison]::OrdinalIgnoreCase) -ge 0) {
    throw "Preview performance acceptance admits only the local-primary root"
}

function ConvertTo-NormalizedAmePath {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Path
    )

    return $Path.Replace("/", "\").TrimEnd("\").ToLowerInvariant()
}

function Test-AmePathOverlap {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Left,
        [Parameter(Mandatory = $true)]
        [string]$Right
    )

    $normalizedLeft = ConvertTo-NormalizedAmePath $Left
    $normalizedRight = ConvertTo-NormalizedAmePath $Right
    return (
        $normalizedLeft -eq $normalizedRight -or
        $normalizedLeft.StartsWith("$normalizedRight\") -or
        $normalizedRight.StartsWith("$normalizedLeft\")
    )
}

if (Test-AmePathOverlap -Left $resolvedRoot -Right $resolvedStorage) {
    throw "Acceptance storage must remain outside the source root"
}
if (Test-AmePathOverlap -Left $resolvedCatalog -Right $resolvedStorage) {
    throw "Acceptance storage must remain outside the source catalog"
}
if (Test-AmePathOverlap -Left $resolvedRoot -Right $resolvedCatalog) {
    throw "The catalog used for acceptance must remain outside the source root"
}
if (Test-Path -LiteralPath $resolvedStorage) {
    $existingContent = Get-ChildItem -LiteralPath $resolvedStorage -Force |
        Select-Object -First 1
    if ($null -ne $existingContent) {
        throw "Preview performance acceptance storage must be empty"
    }
}

if ($ValidationOnly) {
    Write-Output "AME_PREVIEW_ACCEPTANCE_VALIDATION passed"
    exit 0
}

$repositoryRoot = Get-AmeRepositoryRoot
$cargo = (Get-AmeToolchain).Cargo
$reportPath = Join-Path $resolvedStorage "r2b-preview-performance.log"
$environment = @{
    CEDARFLAKE_AME_PREVIEW_ACCEPTANCE_CONSENT = $requiredToken
    CEDARFLAKE_AME_PREVIEW_SOURCE_CATALOG = $resolvedCatalog
    CEDARFLAKE_AME_PREVIEW_SOURCE_ROOT = $resolvedRoot
    CEDARFLAKE_AME_PREVIEW_STORAGE_ROOT = $resolvedStorage
    CEDARFLAKE_AME_PREVIEW_REPORT = $reportPath
    CEDARFLAKE_AME_PREVIEW_LOGICAL_ROOT = "local-primary"
    CEDARFLAKE_AME_PREVIEW_MAX_ITEMS = $MaxItems.ToString()
    CEDARFLAKE_AME_PREVIEW_MIN_SUCCESSFUL_ITEMS = $MinSuccessfulItems.ToString()
    CEDARFLAKE_AME_PREVIEW_CACHE_BUDGET_BYTES = $CacheBudgetBytes.ToString()
    CEDARFLAKE_AME_PREVIEW_MAX_SOURCE_FILE_BYTES = $MaxSourceFileBytes.ToString()
}
$previousEnvironment = @{}
foreach ($name in $environment.Keys) {
    $previousEnvironment[$name] = [System.Environment]::GetEnvironmentVariable($name, "Process")
    [System.Environment]::SetEnvironmentVariable($name, $environment[$name], "Process")
}

$processInfo = [System.Diagnostics.ProcessStartInfo]::new()
$processInfo.FileName = $cargo
$processInfo.Arguments = (
    "test --release --locked --manifest-path rust\Cargo.toml " +
    "user_authorized_preview_performance_acceptance " +
    "-- --ignored --nocapture --test-threads=1"
)
$processInfo.WorkingDirectory = $repositoryRoot
$processInfo.UseShellExecute = $false

$process = [System.Diagnostics.Process]::new()
$process.StartInfo = $processInfo
$startedAt = Get-Date
$deadline = $startedAt.AddSeconds($TimeLimitSeconds)
$peakWorkingSetBytes = [UInt64]0
$failure = $null
$hasStarted = $false
$toolLock = Enter-AmeRepositoryToolLock
try {
    if (-not $process.Start()) {
        throw "Could not start the preview performance acceptance process"
    }
    $hasStarted = $true
    while (-not $process.HasExited) {
        $testProcesses = @(Get-Process -Name "rust_lib_cedarflake_ame-*" `
            -ErrorAction SilentlyContinue | Where-Object { $_.StartTime -ge $startedAt })
        foreach ($testProcess in $testProcesses) {
            $workingSetBytes = [UInt64]$testProcess.WorkingSet64
            if ($workingSetBytes -gt $peakWorkingSetBytes) {
                $peakWorkingSetBytes = $workingSetBytes
            }
            if ($workingSetBytes -gt $MemoryLimitBytes) {
                $failure = (
                    "Preview performance acceptance exceeded the memory limit of " +
                    "$MemoryLimitBytes bytes"
                )
            }
        }
        if ((Get-Date) -ge $deadline) {
            $failure = "Preview performance acceptance exceeded its time limit"
        }
        if ($null -ne $failure) {
            foreach ($testProcess in $testProcesses) {
                $testProcess.Kill()
                $testProcess.WaitForExit()
            }
            $process.Kill()
            $process.WaitForExit()
            break
        }
        Start-Sleep -Milliseconds 20
    }
    if (-not $process.HasExited) {
        $process.WaitForExit()
    }
} finally {
    if ($hasStarted -and -not $process.HasExited) {
        $process.Kill()
        $process.WaitForExit()
    }
    foreach ($name in $environment.Keys) {
        [System.Environment]::SetEnvironmentVariable(
            $name,
            $previousEnvironment[$name],
            "Process"
        )
    }
    Exit-AmeRepositoryToolLock $toolLock
}

if ($null -ne $failure) {
    throw $failure
}
if ($process.ExitCode -ne 0) {
    throw "Preview performance acceptance failed with exit code $($process.ExitCode)"
}
if (-not (Test-Path -LiteralPath $reportPath -PathType Leaf)) {
    throw "Preview performance acceptance completed without a report"
}

$memoryLine = (
    "AME_PREVIEW_MEMORY peak_working_set_bytes=$peakWorkingSetBytes " +
    "limit_bytes=$MemoryLimitBytes"
)
[System.IO.File]::AppendAllText(
    $reportPath,
    "$memoryLine$([Environment]::NewLine)",
    [System.Text.UTF8Encoding]::new($false)
)
$report = [System.IO.File]::ReadAllText($reportPath)
Write-Output $report.TrimEnd()
Write-Output "AME_PREVIEW_REPORT path=$reportPath"
