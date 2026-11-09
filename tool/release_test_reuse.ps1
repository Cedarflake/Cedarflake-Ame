$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
. (Join-Path $PSScriptRoot 'quality_common.ps1')
. (Join-Path $PSScriptRoot 'acceptance_scratch_storage.ps1')
. (Join-Path $PSScriptRoot 'release_reuse_policy.ps1')
. (Join-Path $PSScriptRoot 'release_reuse_github.ps1')

function Assert-ReuseRejected([scriptblock]$Action, [string]$Reason) {
    $rejectionObserved = $false
    try { & $Action } catch { $rejectionObserved = $true }
    if (-not $rejectionObserved) { throw "Release reuse admitted $Reason" }
}

foreach ($case in @(
    @{ paths = @('docs/releases/v1.2.3.md', 'tool/release_test_reuse.ps1'); stage = 'packages' },
    @{ paths = @('.github/workflows/release_publish_windows.yml'); stage = 'packages' },
    @{ paths = @('tool/release_package_installer_windows.ps1'); stage = 'candidate' },
    @{ paths = @('windows/installer/package.wxs'); stage = 'candidate' },
    @{ paths = @('tool/release_verify_windows_candidate.ps1'); stage = 'fresh' },
    @{ paths = @('tool/quality_common.ps1'); stage = 'fresh' },
    @{ paths = @('unknown/owner.ps1'); stage = 'fresh' }
)) {
    if ((Get-AmeReleaseReuseStage $case.paths) -cne $case.stage) { throw "Wrong continuation stage for $($case.paths)" }
}

$script:run = [pscustomobject]@{
    id = 42; head_sha = 'a' * 40; head_branch = 'main'; event = 'workflow_dispatch'
    path = '.github/workflows/release_candidate_windows.yml'; status = 'completed'
    repository = @{ full_name = 'fixture/ame' }; head_repository = @{ full_name = 'fixture/ame' }
}
$tag = 'v1.2.3'
$source = 'b' * 40
$script:candidate = [pscustomobject]@{
    id = 101; name = "Cedarflake-Ame-$tag-unsigned-42-1"; expired = $false
    size_in_bytes = 200; digest = 'sha256:' + ('c' * 64)
    workflow_run = @{ id = 42; head_sha = 'a' * 40 }
}
$script:packages = [pscustomobject]@{
    id = 102; name = "Cedarflake-Ame-$tag-windows-$source-42-1"; expired = $false
    size_in_bytes = 200; digest = 'sha256:' + ('d' * 64)
    workflow_run = @{ id = 42; head_sha = 'a' * 40 }
}
$script:receiptArtifact = [pscustomobject]@{
    id = 103; name = "$($packages.name)-receipt"; expired = $false
    size_in_bytes = 200; digest = 'sha256:' + ('e' * 64)
    workflow_run = @{ id = 42; head_sha = 'a' * 40 }
}
$script:jobs = @(
    @{ name = 'Windows Candidate Gate / Windows Gate'; conclusion = 'success' },
    @{ name = 'Windows Candidate Gate / Release / Verify Unsigned Candidate'; conclusion = 'success' },
    @{ name = 'Windows Packages / Windows ZIP and MSI Packages'; conclusion = 'success' }
)
$script:artifacts = @($candidate, $packages, $receiptArtifact)
$script:changes = @('.github/workflows/release_publish_windows.yml')
$script:receipt = [pscustomobject]@{
    format = 'cedarflake-ame-release-packages-v1'; tag = $tag; sourceCommit = $source
    workflowCommit = $run.head_sha; runId = 42; runAttempt = 1; packageArtifact = $packages.name
    signingMode = 'unsigned'; expectedPublisher = ''
    archiveName = "Cedarflake-Ame-$tag-windows-x64-portable.zip"; archiveSha256 = 'f' * 64
    installerName = "Cedarflake-Ame-$tag-windows-x64.msi"; installerSha256 = 'f' * 64
}
function Read-AmeReleaseApi([string]$Endpoint) {
    switch -Wildcard ($Endpoint) {
        '*/workflows/*/runs?*' { return @{ workflow_runs = @($script:run) } }
        '*/runs/42' { return $script:run }
        '*/runs/42/artifacts?*' { return @{ artifacts = $script:artifacts } }
        '*/runs/42/attempts/1/jobs?*' { return @{ jobs = $script:jobs } }
        default { throw "Unexpected provenance endpoint: $Endpoint" }
    }
}
function Get-AmeReleaseToolChanges { return $script:changes }
function Read-AmePackageReceiptArtifact { return $script:receipt }
$request = @{
    Repository = 'fixture/ame'; Tag = $tag; SourceCommit = $source; WorkflowCommit = 'f' * 40
    SigningMode = 'unsigned'; ExpectedPublisher = ''; PreviousRun = 'auto'; Directory = 'unused'
}
if ((Find-AmeReleaseReuse @request).mode -cne 'packages') { throw 'A publication-only change failed to reuse verified packages' }
$script:changes = @('tool/release_package_installer_windows.ps1')
if ((Find-AmeReleaseReuse @request).mode -cne 'candidate') { throw 'Packaging changes must retain the verified application' }
$script:changes = @('tool/quality_unsigned_windows.ps1')
if ($null -ne (Find-AmeReleaseReuse @request)) { throw 'Changed candidate gates reused old verification' }
$script:changes = @('docs/releases/v1.2.3.md')
$script:artifacts = @($candidate, $packages)
if ((Find-AmeReleaseReuse @request).mode -cne 'candidate') { throw 'Legacy packages without a receipt must fall back to a verified candidate' }
$script:jobs[0].conclusion = 'failure'
if ($null -ne (Find-AmeReleaseReuse @request)) { throw 'Failed candidate gate was reused' }
$script:jobs[0].conclusion = 'success'
$candidate.expired = $true
if ($null -ne (Find-AmeReleaseReuse @request)) { throw 'Expired candidate was reused' }
$candidate.expired = $false
$run.event = 'pull_request'
if ($null -ne (Find-AmeReleaseReuse @request)) { throw 'Pull-request artifacts were trusted for release' }
$request.PreviousRun = '42'
Assert-ReuseRejected { Find-AmeReleaseReuse @request } 'an explicit untrusted run'
$run.event = 'workflow_dispatch'
$request.PreviousRun = 'auto'
$request.Tag = 'v1.2.4'
if ($null -ne (Find-AmeReleaseReuse @request)) { throw 'A different release tag reused packages' }
$request.Tag = $tag
$script:artifacts = @($packages, $receiptArtifact)
$receipt.sourceCommit = '0' * 40
Assert-ReuseRejected { Find-AmeReleaseReuse @request } 'a mismatched receipt source'
$receipt.sourceCommit = $source
$receipt.runAttempt = 2
Assert-ReuseRejected { Find-AmeReleaseReuse @request } 'a different producing attempt'
$receipt.runAttempt = 1
$candidate.workflow_run.head_sha = '0' * 40
Assert-ReuseRejected { Assert-AmeReleaseArtifact $candidate $run } 'an artifact from another commit'

Invoke-AmeAcceptanceScratch -Name 'release-reuse' -Action {
    param($directory)
    $pair = Join-Path $directory 'pair'
    [void][IO.Directory]::CreateDirectory($pair)
    foreach ($name in @($script:receipt.archiveName, $script:receipt.installerName)) {
        [IO.File]::WriteAllText((Join-Path $pair $name), $name, [Text.UTF8Encoding]::new($false))
    }
    $script:receipt.archiveSha256 = (Get-FileHash (Join-Path $pair $script:receipt.archiveName) -Algorithm SHA256).Hash.ToLowerInvariant()
    $script:receipt.installerSha256 = (Get-FileHash (Join-Path $pair $script:receipt.installerName) -Algorithm SHA256).Hash.ToLowerInvariant()
    Assert-AmeReusedPackageFiles $pair $script:receipt
    [IO.File]::WriteAllText((Join-Path $pair $script:receipt.installerName), 'changed', [Text.UTF8Encoding]::new($false))
    Assert-ReuseRejected { Assert-AmeReusedPackageFiles $pair $script:receipt } 'a changed MSI'
}
# Restore the real transport/parser after the API-selection fixtures.
. (Join-Path $PSScriptRoot 'release_reuse_github.ps1')
Invoke-AmeAcceptanceScratch -Name 'release-reuse-transport' -Action {
    param($directory)
    Add-Type -AssemblyName System.IO.Compression
    Add-Type -AssemblyName System.IO.Compression.FileSystem
    $script:downloadFixture = Join-Path $directory 'transport.zip'
    function Invoke-WebRequest {
        param($Uri, $Headers, $OutFile, $TimeoutSec)
        if ($Uri -cnotmatch '^https://api\.github\.com/repos/fixture/ame/actions/artifacts/[0-9]+/zip$') {
            throw "Unexpected artifact endpoint: $Uri"
        }
        Copy-Item -LiteralPath $script:downloadFixture -Destination $OutFile
    }
    function Write-TransportZip([string]$Entry, [string]$Content) {
        $stream = [IO.File]::Create($script:downloadFixture)
        $archive = $null
        try {
            $archive = [IO.Compression.ZipArchive]::new($stream, [IO.Compression.ZipArchiveMode]::Create)
            $writer = [IO.StreamWriter]::new($archive.CreateEntry($Entry).Open(), [Text.UTF8Encoding]::new($false))
            try { $writer.Write($Content) } finally { $writer.Dispose() }
        } finally {
            if ($null -ne $archive) { $archive.Dispose() }
            $stream.Dispose()
        }
        return 'sha256:' + (Get-FileHash $script:downloadFixture -Algorithm SHA256).Hash.ToLowerInvariant()
    }

    $selected = [pscustomobject]@{ artifactId = 101; digest = (Write-TransportZip 'payload/app.txt' 'verified') }
    $extracted = Join-Path $directory 'valid'
    Expand-AmeReleaseReuseArtifact 'fixture/ame' $selected $extracted
    if ([IO.File]::ReadAllText((Join-Path $extracted 'payload/app.txt')) -cne 'verified') {
        throw 'Verified artifact bytes did not reach the extracted payload'
    }
    [void](Write-TransportZip 'payload/app.txt' 'replaced')
    $rejected = Join-Path $directory 'changed'
    Assert-ReuseRejected { Expand-AmeReleaseReuseArtifact 'fixture/ame' $selected $rejected } 'changed transport bytes'
    if (Test-Path -LiteralPath $rejected) { throw 'Digest failure extracted a payload' }
    $selected.digest = Write-TransportZip '../escaped.txt' 'untrusted'
    Assert-ReuseRejected { Expand-AmeReleaseReuseArtifact 'fixture/ame' $selected $rejected } 'archive traversal'
    if (Test-Path -LiteralPath (Join-Path $directory 'escaped.txt')) { throw 'Unsafe artifact escaped its destination' }

    $receiptZip = [pscustomobject]@{ id = 103; size_in_bytes = 1000; digest = '' }
    $receiptZip.digest = Write-TransportZip 'release-package-receipt.json' ($script:receipt | ConvertTo-Json)
    $decoded = Read-AmePackageReceiptArtifact 'fixture/ame' $receiptZip (Join-Path $directory 'receipts')
    Assert-AmeReleasePackageReceipt $decoded $script:run $tag $source 'unsigned' '' $script:packages.name 1
    [void](Write-TransportZip 'release-package-receipt.json' '{"tag":"changed"}')
    Assert-ReuseRejected {
        Read-AmePackageReceiptArtifact 'fixture/ame' $receiptZip (Join-Path $directory 'receipts')
    } 'a changed receipt archive'
}
Write-Host 'Release reuse selection, provenance and package integrity fixtures passed'
