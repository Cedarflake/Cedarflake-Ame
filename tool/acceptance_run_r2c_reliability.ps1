[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [string]$LocalRoot,
    [Parameter(Mandatory = $true)]
    [string]$CloudRoot,
    [Parameter(Mandatory = $true)]
    [string]$SourceCatalogPath,
    [Parameter(Mandatory = $true)]
    [string]$StorageRoot,
    [Parameter(Mandatory = $true)]
    [string]$AuthorizationToken,
    [switch]$AcknowledgeCloudReadOnly,
    [ValidateRange(60, 3600)]
    [int]$TimeLimitSeconds = 1800,
    [ValidateRange(268435456, 4294967296)]
    [UInt64]$MemoryLimitBytes = 2147483648,
    [switch]$ValidationOnly
)

$ErrorActionPreference = "Stop"
. (Join-Path $PSScriptRoot "quality_common.ps1")

$requiredToken = "CEDARFLAKE_AME_R2C_RELIABILITY_ACCEPTANCE_V1"
if ($AuthorizationToken -cne $requiredToken) {
    throw "The exact current R2c-H reliability authorization token is required"
}
if (-not $AcknowledgeCloudReadOnly) {
    throw "The cloud root requires an explicit read-only acknowledgement"
}

$resolvedLocalRoot = [System.IO.Path]::GetFullPath($LocalRoot)
$resolvedCloudRoot = [System.IO.Path]::GetFullPath($CloudRoot)
$resolvedCatalog = [System.IO.Path]::GetFullPath($SourceCatalogPath)
$resolvedStorage = [System.IO.Path]::GetFullPath($StorageRoot)
if (-not (Test-Path -LiteralPath $resolvedLocalRoot -PathType Container)) {
    throw "The local-primary acceptance root is not an available directory"
}
if (-not (Test-Path -LiteralPath $resolvedCloudRoot -PathType Container)) {
    throw "The cloud-primary acceptance root is not an available directory"
}
if (-not (Test-Path -LiteralPath $resolvedCatalog -PathType Leaf)) {
    throw "The retained source catalog is not an available file"
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

if (Test-AmePathOverlap -Left $resolvedLocalRoot -Right $resolvedCloudRoot) {
    throw "The two authorized logical roots must not overlap"
}
foreach ($sourcePath in @($resolvedLocalRoot, $resolvedCloudRoot, $resolvedCatalog)) {
    if (Test-AmePathOverlap -Left $sourcePath -Right $resolvedStorage) {
        throw "R2c-H isolated storage must remain outside every source path"
    }
}
foreach ($rootPath in @($resolvedLocalRoot, $resolvedCloudRoot)) {
    if (Test-AmePathOverlap -Left $rootPath -Right $resolvedCatalog) {
        throw "The retained catalog must remain outside both source roots"
    }
}
if (Test-Path -LiteralPath $resolvedStorage) {
    $existingContent = Get-ChildItem -LiteralPath $resolvedStorage -Force |
        Select-Object -First 1
    if ($null -ne $existingContent) {
        throw "R2c-H reliability acceptance storage must be empty"
    }
}

if ($ValidationOnly) {
    Write-Output "AME_R2C_H_VALIDATION status=passed"
    exit 0
}

New-Item -ItemType Directory -Path $resolvedStorage -Force | Out-Null
$repositoryRoot = Get-AmeRepositoryRoot
$cargo = (Get-AmeToolchain).Cargo
$reportPath = Join-Path $resolvedStorage "r2c-h-large-library-reliability.log"
$environment = @{
    CEDARFLAKE_AME_R2C_H_CONSENT = $requiredToken
    CEDARFLAKE_AME_R2C_H_CLOUD_READ_ONLY_ACK = "true"
    CEDARFLAKE_AME_R2C_H_LOCAL_ROOT = $resolvedLocalRoot
    CEDARFLAKE_AME_R2C_H_CLOUD_ROOT = $resolvedCloudRoot
    CEDARFLAKE_AME_R2C_H_SOURCE_CATALOG = $resolvedCatalog
    CEDARFLAKE_AME_R2C_H_STORAGE_ROOT = $resolvedStorage
    CEDARFLAKE_AME_R2C_H_REPORT = $reportPath
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
    "r2c_h_ -- --ignored --nocapture --test-threads=1"
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
        throw "Could not start the R2c-H reliability acceptance process"
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
                    "R2c-H reliability acceptance exceeded the memory limit of " +
                    "$MemoryLimitBytes bytes"
                )
            }
        }
        if ((Get-Date) -ge $deadline) {
            $failure = "R2c-H reliability acceptance exceeded its time limit"
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
    throw "R2c-H reliability acceptance failed with exit code $($process.ExitCode)"
}
if (-not (Test-Path -LiteralPath $reportPath -PathType Leaf)) {
    throw "R2c-H reliability acceptance completed without a report"
}

$memoryLine = (
    "AME_R2C_H_MEMORY peak_working_set_bytes=$peakWorkingSetBytes " +
    "limit_bytes=$MemoryLimitBytes"
)
[System.IO.File]::AppendAllText(
    $reportPath,
    "$memoryLine$([Environment]::NewLine)",
    [System.Text.UTF8Encoding]::new($false)
)
$report = [System.IO.File]::ReadAllText($reportPath)
Write-Output $report.TrimEnd()
Write-Output "AME_R2C_H_REPORT status=available"
