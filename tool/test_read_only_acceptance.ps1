$ErrorActionPreference = "Stop"

$repositoryRoot = Split-Path -Parent $PSScriptRoot
$buildRoot = Join-Path $repositoryRoot "build"
$testRoot = Join-Path $buildRoot "acceptance-harness-$PID"
$sourceRoot = Join-Path $repositoryRoot "windows\runner\resources"
$sourceFile = Join-Path $sourceRoot "app_icon.ico"
$resolvedBuildRoot = [System.IO.Path]::GetFullPath($buildRoot)
$resolvedTestRoot = [System.IO.Path]::GetFullPath($testRoot)
if (-not $resolvedTestRoot.StartsWith(
    "$resolvedBuildRoot$([System.IO.Path]::DirectorySeparatorChar)",
    [System.StringComparison]::OrdinalIgnoreCase
)) {
    throw "Acceptance harness storage must remain inside the repository build directory"
}

function Assert-Contains {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Value,
        [Parameter(Mandatory = $true)]
        [string]$Expected
    )

    if ($Value.IndexOf($Expected, [System.StringComparison]::Ordinal) -lt 0) {
        throw "Expected acceptance output to contain: $Expected"
    }
}

function Get-FileSha256 {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Path
    )

    $stream = [System.IO.File]::OpenRead($Path)
    $algorithm = [System.Security.Cryptography.SHA256]::Create()
    try {
        $hash = $algorithm.ComputeHash($stream)
        return [BitConverter]::ToString($hash).Replace("-", "")
    } finally {
        $algorithm.Dispose()
        $stream.Dispose()
    }
}

function Invoke-ControlledAcceptance {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Name,
        [UInt64]$CancelAfter,
        [UInt64]$PauseAfter
    )

    $arguments = @{
        RootPath = $sourceRoot
        StorageRoot = Join-Path $resolvedTestRoot $Name
        ScanId = $Name
        AuthorizationToken = "CEDARFLAKE_AME_READ_ONLY_ACCEPTANCE_V1"
    }
    if ($CancelAfter -gt 0) {
        $arguments.CancelAfter = $CancelAfter
    }
    if ($PauseAfter -gt 0) {
        $arguments.PauseAfter = $PauseAfter
    }
    return (& "$PSScriptRoot\accept_read_only_library.ps1" @arguments | Out-String)
}

$sourceHashBefore = Get-FileSha256 $sourceFile
$sourceEntriesBefore = @(Get-ChildItem -LiteralPath $sourceRoot -Force).Count
New-Item -ItemType Directory -Path $resolvedTestRoot -Force | Out-Null

Push-Location $repositoryRoot
try {
    try {
        & "$PSScriptRoot\accept_read_only_library.ps1" `
            -RootPath "Z:\must-not-be-read" `
            -StorageRoot (Join-Path $resolvedTestRoot "must-not-exist") `
            -ScanId "refusal" `
            -AuthorizationToken "wrong"
        throw "Invalid acceptance authorization unexpectedly succeeded"
    } catch {
        Assert-Contains $_.Exception.Message "exact current read-only acceptance authorization token"
    }

    $cloudRoot = Join-Path $resolvedTestRoot "OneDrive-guard"
    New-Item -ItemType Directory -Path $cloudRoot -Force | Out-Null
    try {
        & "$PSScriptRoot\accept_read_only_library.ps1" `
            -RootPath $cloudRoot `
            -StorageRoot (Join-Path $resolvedTestRoot "cloud-storage") `
            -ScanId "cloud-refusal" `
            -AuthorizationToken "CEDARFLAKE_AME_READ_ONLY_ACCEPTANCE_V1"
        throw "Cloud-backed acceptance unexpectedly omitted its explicit switch"
    } catch {
        Assert-Contains $_.Exception.Message "AllowCloudBackedRoot"
    }

    try {
        & "$PSScriptRoot\accept_read_only_library.ps1" `
            -RootPath $sourceRoot `
            -StorageRoot (Join-Path $sourceRoot "overlap") `
            -ScanId "overlap-refusal" `
            -AuthorizationToken "CEDARFLAKE_AME_READ_ONLY_ACCEPTANCE_V1"
        throw "Overlapping acceptance storage unexpectedly succeeded"
    } catch {
        Assert-Contains $_.Exception.Message "outside the source root"
    }

    $completed = Invoke-ControlledAcceptance -Name "controlled-completed"
    Assert-Contains $completed "status=completed"
    Assert-Contains $completed "scan_locations=1 is_active=true"
    Assert-Contains $completed "active_roots=1 active_locations_total=1"
    Assert-Contains $completed "issue_codes=none"
    Assert-Contains $completed "source_hash_samples=1"

    try {
        & "$PSScriptRoot\accept_read_only_library.ps1" `
            -RootPath $sourceRoot `
            -StorageRoot (Join-Path $resolvedTestRoot "controlled-completed") `
            -ScanId "storage-refusal" `
            -AuthorizationToken "CEDARFLAKE_AME_READ_ONLY_ACCEPTANCE_V1"
        throw "Nonempty acceptance storage unexpectedly succeeded"
    } catch {
        Assert-Contains $_.Exception.Message "storage is not empty"
    }

    $cancelled = Invoke-ControlledAcceptance -Name "controlled-cancelled" -CancelAfter 1
    Assert-Contains $cancelled "status=cancelled"
    Assert-Contains $cancelled "scan_locations=0 is_active=false"
    Assert-Contains $cancelled "active_roots=0 active_locations_total=0"
    Assert-Contains $cancelled "cancel_response_ms=Some("

    $resumed = Invoke-ControlledAcceptance -Name "controlled-resumed" -PauseAfter 1
    Assert-Contains $resumed "status=completed"
    Assert-Contains $resumed "pause_response_ms=Some("
    Assert-Contains $resumed "resume_ms=Some("

    $sourceHashAfter = Get-FileSha256 $sourceFile
    $sourceEntriesAfter = @(Get-ChildItem -LiteralPath $sourceRoot -Force).Count
    if ($sourceHashAfter -cne $sourceHashBefore) {
        throw "The controlled acceptance harness changed source bytes"
    }
    if ($sourceEntriesAfter -ne $sourceEntriesBefore) {
        throw "The controlled acceptance harness changed source entries"
    }

    Write-Output "AME_READ_ONLY_ACCEPTANCE_HARNESS passed"
} finally {
    Pop-Location
    if (Test-Path -LiteralPath $resolvedTestRoot) {
        Remove-Item -LiteralPath $resolvedTestRoot -Recurse -Force
    }
}
