$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'quality_common.ps1')
. (Join-Path $PSScriptRoot 'acceptance_scratch_storage.ps1')
. (Join-Path $PSScriptRoot 'release_store_identity.ps1')
. (Join-Path $PSScriptRoot 'release_store_api.ps1')
. (Join-Path $PSScriptRoot 'release_store_artifacts.ps1')

function Assert-StoreRejected {
    param([scriptblock]$Action, [string]$Reason)
    $rejected = $false
    try { & $Action | Out-Null } catch { $rejected = $true }
    if (-not $rejected) { throw "Store boundary admitted $Reason" }
}

function New-StoreTestArchive {
    param([string]$Path, [string]$Manifest, [string]$Content, [switch]$ExtraExecutable)
    $archive = [IO.Compression.ZipFile]::Open($Path, [IO.Compression.ZipArchiveMode]::Create)
    try {
        $entries = @{
            'AppxManifest.xml' = $Manifest; 'cedarflake_ame.exe' = $Content; 'data/app.so' = $Content
            'AppxBlockMap.xml' = '<BlockMap />'; '[Content_Types].xml' = '<Types />'
        }
        if ($ExtraExecutable) { $entries['unadmitted.dll'] = 'unexpected' }
        foreach ($name in $entries.Keys) {
            $writer = [IO.StreamWriter]::new($archive.CreateEntry($name).Open(), [Text.UTF8Encoding]::new($false))
            try { $writer.Write($entries[$name]) } finally { $writer.Dispose() }
        }
    } finally { $archive.Dispose() }
}

$run = [pscustomobject]@{
    id = 123; head_sha = 'a' * 40; status = 'completed'; conclusion = 'success'
    event = 'workflow_dispatch'; head_branch = 'main'; path = '.github/workflows/release_store_windows.yml'
    repository = @{ full_name = 'Cedarflake/Cedarflake-Ame' }; head_repository = @{ full_name = 'Cedarflake/Cedarflake-Ame' }
}
$artifact = [pscustomobject]@{
    id = 456; name = 'store-msix-123-1'; expired = $false; size_in_bytes = 100
    digest = 'sha256:' + ('b' * 64); workflow_run = @{ id = 123; head_sha = 'a' * 40 }
}
Assert-AmeStoreArtifactProducer $run $artifact 'Cedarflake/Cedarflake-Ame' Msix
foreach ($field in @('head_branch', 'conclusion', 'path')) {
    $prior = $run.$field
    $run.$field = 'untrusted'
    Assert-StoreRejected { Assert-AmeStoreArtifactProducer $run $artifact 'Cedarflake/Cedarflake-Ame' Msix } $field
    $run.$field = $prior
}
$artifact.workflow_run.id = 999
Assert-StoreRejected { Assert-AmeStoreArtifactProducer $run $artifact 'Cedarflake/Cedarflake-Ame' Msix } 'another artifact producer'
$artifact.workflow_run.id = 123

$asset = [pscustomobject]@{ name = 'ame.msix'; state = 'uploaded'; digest = 'sha256:' + ('a' * 64) }
if ((Get-AmeStoreAssetDisposition @() 'ame.msix' ('a' * 64)) -cne 'upload' -or
    (Get-AmeStoreAssetDisposition @($asset) 'ame.msix' ('a' * 64)) -cne 'retained') { throw 'Store asset retry is not idempotent' }
Assert-StoreRejected { Get-AmeStoreAssetDisposition @($asset) 'ame.msix' ('b' * 64) } 'replacement of another build'
Assert-StoreRejected { Get-AmeStoreAssetDisposition @($asset, $asset) 'ame.msix' ('a' * 64) } 'ambiguous existing assets'
foreach ($state in @('PendingCommit', 'CommitStarted', 'PendingPublication', 'Certification')) {
    if ((Get-AmeStoreStatusDisposition $state) -cne 'pending') { throw 'Unpublished Store state admitted publication' }
}
if ((Get-AmeStoreStatusDisposition 'Published') -cne 'published' -or
    (Get-AmeStoreStatusDisposition 'CertificationFailed') -cne 'failed') { throw 'Store terminal outcome differs' }
Assert-StoreRejected { Get-AmeStoreStatusDisposition 'UnexpectedState' } 'unknown Store state'

Invoke-AmeAcceptanceScratch -Name 'store-publication-contract' -Action {
    param($scratch)
    $manifestPath = Join-Path $scratch 'AppxManifest.xml'
    New-AmeMsixManifest $manifestPath '1.1.4.0'
    $manifest = Get-Content -LiteralPath $manifestPath -Raw -Encoding utf8
    $content = 'candidate-build'
    $hash = [Convert]::ToHexString([Security.Cryptography.SHA256]::HashData([Text.Encoding]::UTF8.GetBytes($content))).ToLowerInvariant()
    $receipt = [pscustomobject]@{
        format = 'cedarflake-ame-msix-v1'; sourceCommit = 'a' * 40; dirtyWorktree = $false
        sourceEvidenceFormat = 'cedarflake-ame-unsigned-v1'; releaseTag = 'v0.1.0-beta.1'; architecture = 'x64'
        applicationVersion = '0.1.0-beta.1+4'; packageVersion = '1.1.4.0'; identity = Get-AmeMsixIdentity
        fileName = 'Cedarflake-Ame-v0.1.0-beta.1-windows-x64.msix'; sha256 = 'b' * 64
        payload = @('cedarflake_ame.exe', 'data/app.so') | ForEach-Object {
            [pscustomobject]@{ relativePath = $_; length = $content.Length; sha256 = $hash }
        }
        generatedPayload = @([pscustomobject]@{
            relativePath = 'AppxManifest.xml'; length = ([Text.Encoding]::UTF8.GetByteCount($manifest))
            sha256 = (Get-FileHash -LiteralPath $manifestPath -Algorithm SHA256).Hash.ToLowerInvariant()
        })
    }
    $package = Join-Path $scratch $receipt.fileName
    New-StoreTestArchive $package $manifest $content
    $receipt.sha256 = (Get-FileHash -LiteralPath $package -Algorithm SHA256).Hash.ToLowerInvariant()
    $receiptPath = Join-Path $scratch 'msix-package.json'
    $receipt | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath $receiptPath -Encoding utf8
    $admitted = Read-AmeStorePackageReceipt $receiptPath
    Assert-AmeStoreApplicationArchive $package $admitted
    foreach ($case in @('version', 'build', 'extra')) {
        $invalid = Join-Path $scratch "$case.msix"
        $caseManifest = if ($case -ceq 'version') { $manifest.Replace('1.1.4.0', '1.1.5.0') } else { $manifest }
        $caseContent = if ($case -ceq 'build') { 'different-build' } else { $content }
        New-StoreTestArchive $invalid $caseManifest $caseContent -ExtraExecutable:($case -ceq 'extra')
        Assert-StoreRejected { Assert-AmeStoreApplicationArchive $invalid $admitted } "wrong downloaded $case"
    }
    $receipt.dirtyWorktree = $true
    $receipt | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath $receiptPath -Encoding utf8
    Assert-StoreRejected { Read-AmeStorePackageReceipt $receiptPath } 'a dirty candidate'
    $receipt.dirtyWorktree = $false
    $receipt | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath $receiptPath -Encoding utf8

    $draft = [pscustomobject]@{
        id = '1234'; status = 'PendingCommit'; notesForCertification = 'Existing review notes'
        targetPublishMode = 'Manual'; fileUploadUrl = 'https://example.blob.core.windows.net/package?fixture'
        listings = @{ 'en-us' = @{ title = 'Ame' } }; pricing = @{ priceId = 'Free' }
        applicationPackages = @(
            [pscustomobject]@{ fileName = 'old.msix'; architecture = 'x64'; version = '1.1.3.0'; fileStatus = 'Uploaded' }
            [pscustomobject]@{ fileName = 'arm.msix'; architecture = 'ARM64'; version = '1.1.3.0'; fileStatus = 'Uploaded' }
        )
    }
    $updated = Set-AmeStoreSubmissionPackage $draft $receipt
    Assert-AmeStoreSubmissionIdentity $updated $receipt
    if ($updated.applicationPackages[0].fileStatus -cne 'PendingDelete' -or
        $updated.applicationPackages[1].fileStatus -cne 'Uploaded' -or
        -not $updated.notesForCertification.StartsWith('Existing review notes') -or
        $updated.listings['en-us'].title -cne 'Ame' -or $updated.pricing.priceId -cne 'Free') {
        throw 'Package replacement changed retained Store content or another architecture'
    }
    $other = $receipt | ConvertTo-Json -Depth 8 | ConvertFrom-Json
    $other.sha256 = 'c' * 64
    Assert-StoreRejected { Assert-AmeStoreSubmissionIdentity $updated $other } 'another submission candidate'
    $recordPath = Join-Path $scratch 'store-submission.json'
    Save-AmeStoreSubmissionReceipt $recordPath $receipt '1234' 'PendingCommit'
    $record = Get-Content -LiteralPath $recordPath -Raw -Encoding utf8 | ConvertFrom-Json
    if ($record.submissionId -cne '1234' -or $record.packageSha256 -cne $receipt.sha256 -or
        $record.sourceCommit -cne $receipt.sourceCommit) { throw 'Recovery record lost the exact submission identity' }

    $storeFixture = @{
        Requests = [Collections.Generic.List[string]]::new(); Draft = $updated
        HasOtherDraft = $false; FileName = $receipt.fileName
    }
    function Invoke-RestMethod {
        param($Method, $Uri, $Headers, $TimeoutSec, $MaximumRedirection, $ContentType, $Body)
        if ($Uri -like 'https://login.microsoftonline.com/*') { return @{ access_token = 'fixture-token' } }
        $path = ([uri]$Uri).AbsolutePath
        $storeFixture.Requests.Add("$Method $path")
        if ($path -eq '/v1.0/my/applications/9NRXBZ70WGVX') {
            return [pscustomobject]@{
                lastPublishedApplicationSubmission = @{ id = '1000' }
                pendingApplicationSubmission = $(if ($storeFixture.HasOtherDraft) { @{ id = '9999' } } else { $null })
            }
        }
        if ($path.EndsWith('/submissions') -and $Method -eq 'Post') { return $storeFixture.Draft }
        if ($path.EndsWith('/commit')) { $storeFixture.Draft.status = 'Certification'; return @{} }
        if ($path.EndsWith('/status')) { return @{ status = $storeFixture.Draft.status } }
        if ($Method -eq 'Put') {
            $storeFixture.Draft = [Text.Encoding]::UTF8.GetString($Body) | ConvertFrom-Json
        }
        return $storeFixture.Draft
    }
    function Invoke-WebRequest {
        param($Method, $Uri, $InFile, $ContentType, $Headers, $TimeoutSec, $MaximumRedirection)
        if ($Method -ne 'Put' -or $Headers['x-ms-blob-type'] -cne 'BlockBlob') { throw 'Submission did not use a blob upload' }
        $zip = [IO.Compression.ZipFile]::OpenRead($InFile)
        try {
            if ($zip.Entries.Count -ne 1 -or $zip.Entries[0].FullName -cne $storeFixture.FileName) {
                throw 'Submission upload contains unrelated files'
            }
        } finally { $zip.Dispose() }
        $storeFixture.Requests.Add('Upload package')
    }
    $credentials = @{}
    foreach ($name in @('AME_STORE_TENANT_ID', 'AME_STORE_CLIENT_ID', 'AME_STORE_CLIENT_SECRET')) {
        $credentials[$name] = [Environment]::GetEnvironmentVariable($name)
        [Environment]::SetEnvironmentVariable($name, '00000000-0000-0000-0000-000000000001')
    }
    try {
        # Start from a cloned published submission, before this candidate was selected.
        $storeFixture.Draft.applicationPackages = @($storeFixture.Draft.applicationPackages | Select-Object -First 2)
        & (Join-Path $PSScriptRoot 'release_submit_store.ps1') -PackageDirectory $scratch `
            -OutputDirectory (Join-Path $scratch 'submitted') -Operation Submit
        if (@($storeFixture.Requests | Where-Object { $_ -like 'Post */commit' }).Count -ne 1 -or
            @($storeFixture.Requests | Where-Object { $_ -ceq 'Upload package' }).Count -ne 1) {
            throw 'Submission did not upload and commit exactly once'
        }
        $storeFixture.Requests.Clear()
        & (Join-Path $PSScriptRoot 'release_submit_store.ps1') -PackageDirectory $scratch `
            -OutputDirectory (Join-Path $scratch 'resumed') -Operation Submit -SubmissionId 1234
        if (@($storeFixture.Requests | Where-Object { $_ -match '^(Post|Put|Upload)' }).Count -ne 0) {
            throw 'Resuming an in-progress submission repeated mutation'
        }
        $storeFixture.HasOtherDraft = $true
        $storeFixture.Requests.Clear()
        Assert-StoreRejected {
            & (Join-Path $PSScriptRoot 'release_submit_store.ps1') -PackageDirectory $scratch `
                -OutputDirectory (Join-Path $scratch 'foreign-draft') -Operation Submit
        } 'replacement of a pre-existing draft'
        if (@($storeFixture.Requests | Where-Object { $_ -match '^(Post|Put|Upload)' }).Count -ne 0) {
            throw 'Foreign draft refusal mutated Store state'
        }
        Assert-StoreRejected {
            & (Join-Path $PSScriptRoot 'release_submit_store.ps1') -PackageDirectory $scratch `
                -OutputDirectory (Join-Path $scratch 'wrong-id') -Operation Status -SubmissionId 5678
        } 'another submission ID returned by the service'
    } finally {
        foreach ($name in $credentials.Keys) { [Environment]::SetEnvironmentVariable($name, $credentials[$name]) }
    }
}
Write-Host 'Store provenance, payload, submission and immutable attachment contracts passed'
