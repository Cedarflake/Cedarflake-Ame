$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

function Get-AmeStoreAccessToken {
    foreach ($name in @('AME_STORE_TENANT_ID', 'AME_STORE_CLIENT_ID', 'AME_STORE_CLIENT_SECRET')) {
        if ([string]::IsNullOrWhiteSpace([Environment]::GetEnvironmentVariable($name))) {
            throw "Store submission credential is missing: $name"
        }
    }
    if ($env:AME_STORE_TENANT_ID -notmatch '^[0-9a-fA-F-]{36}$' -or
        $env:AME_STORE_CLIENT_ID -notmatch '^[0-9a-fA-F-]{36}$') { throw 'Store Entra identifiers are invalid' }
    try {
        $response = Invoke-RestMethod -Method Post -TimeoutSec 60 -MaximumRedirection 0 `
            -Uri "https://login.microsoftonline.com/$env:AME_STORE_TENANT_ID/oauth2/token" `
            -ContentType 'application/x-www-form-urlencoded' -Body @{
                grant_type = 'client_credentials'
                client_id = $env:AME_STORE_CLIENT_ID
                client_secret = $env:AME_STORE_CLIENT_SECRET
                resource = 'https://manage.devcenter.microsoft.com'
            }
    } catch { throw 'Store authentication failed; check the protected Entra credentials and Partner Center role' }
    if (-not $response.access_token) { throw 'Store authentication returned no access token' }
    return [string]$response.access_token
}

function Invoke-AmeStoreRequest {
    param([ValidateSet('Get', 'Post', 'Put')][string]$Method, [string]$Path,
        [string]$Token, [object]$Body)
    if ($Path -cnotmatch '^applications/[A-Z0-9]+(?:/submissions(?:/[0-9]+(?:/(?:status|commit))?)?)?$') {
        throw 'Store request path is outside the submission API'
    }
    $arguments = @{
        Method = $Method; Uri = "https://manage.devcenter.microsoft.com/v1.0/my/$Path"
        Headers = @{ Authorization = "Bearer $Token" }
        TimeoutSec = 120; MaximumRedirection = 0
    }
    if ($null -ne $Body) {
        $arguments.ContentType = 'application/json; charset=utf-8'
        $arguments.Body = [Text.Encoding]::UTF8.GetBytes(($Body | ConvertTo-Json -Depth 50))
    }
    try { Invoke-RestMethod @arguments }
    catch { throw "Store $Method request failed at $Path; inspect that submission before retrying" }
}

function Get-AmeStoreStatusDisposition {
    param([string]$Status)
    switch -CaseSensitive ($Status) {
        'Published' { return 'published' }
        { $_ -cin @('Canceled', 'CommitFailed', 'PublishFailed', 'PreProcessingFailed', 'CertificationFailed', 'ReleaseFailed') } {
            return 'failed'
        }
        { $_ -cin @('PendingCommit', 'CommitStarted', 'PendingPublication', 'Publishing', 'PreProcessing', 'Certification', 'Release') } {
            return 'pending'
        }
        default { throw "Unknown Store submission status: $Status" }
    }
}

function Assert-AmeStoreSubmissionIdentity {
    param([object]$Submission, [object]$Receipt)
    $marker = "Ame package SHA-256: $($Receipt.sha256)"
    $notes = ([string]$Submission.notesForCertification).Split("`n") | ForEach-Object { $_.TrimEnd("`r") }
    $packages = @($Submission.applicationPackages | Where-Object {
        $_.fileName -ceq $Receipt.fileName -and $_.fileStatus -cne 'PendingDelete'
    })
    if ([string]$Submission.id -cnotmatch '^[1-9][0-9]*$' -or $notes -cnotcontains $marker -or $packages.Count -ne 1) {
        throw 'Store submission belongs to another package; it will not be changed or reused'
    }
    if ($packages[0].PSObject.Properties['version'] -and $packages[0].version -and
        $packages[0].version -cne $Receipt.packageVersion) { throw 'Store submission package version differs' }
}

function Set-AmeStoreSubmissionPackage {
    param([object]$Submission, [object]$Receipt)
    # The API clones the last published listing. Retain listing, pricing and other architectures.
    foreach ($package in $Submission.applicationPackages) {
        if ($package.architecture -ieq $Receipt.architecture) { $package.fileStatus = 'PendingDelete' }
    }
    $Submission.applicationPackages = @($Submission.applicationPackages) + @([pscustomobject]@{
        fileName = $Receipt.fileName; fileStatus = 'PendingUpload'
        minimumDirectXVersion = 'None'; minimumSystemRam = 'None'
    })
    $notes = @(([string]$Submission.notesForCertification).Split("`n") | Where-Object {
        $_ -notmatch '^Ame package SHA-256: [0-9a-f]{64}\r?$'
    })
    $Submission.notesForCertification = ($notes + "Ame package SHA-256: $($Receipt.sha256)") -join "`n"
    $Submission.targetPublishMode = 'Immediate'
    return $Submission
}

function Send-AmeStorePackageArchive {
    param([string]$UploadUrl, [string]$PackagePath, [string]$ZipPath)
    $uri = [uri]$UploadUrl
    if ($uri.Scheme -cne 'https' -or -not $uri.Host.EndsWith('.blob.core.windows.net') -or $uri.UserInfo) {
        throw 'Store upload endpoint is not an Azure Blob SAS URL'
    }
    $archive = [IO.Compression.ZipFile]::Open($ZipPath, [IO.Compression.ZipArchiveMode]::Create)
    try {
        [void][IO.Compression.ZipFileExtensions]::CreateEntryFromFile($archive, $PackagePath,
            [IO.Path]::GetFileName($PackagePath), [IO.Compression.CompressionLevel]::NoCompression)
    } finally { $archive.Dispose() }
    try {
        Invoke-WebRequest -Method Put -Uri $uri -InFile $ZipPath -ContentType 'application/zip' `
            -Headers @{ 'x-ms-blob-type' = 'BlockBlob'; 'x-ms-version' = '2021-12-02' } `
            -TimeoutSec 1200 -MaximumRedirection 0 | Out-Null
    } catch { throw 'Store package upload failed; the recorded draft remains available for recovery' }
}

function Save-AmeStoreSubmissionReceipt {
    param([string]$Path, [object]$Package, [string]$SubmissionId, [string]$Status)
    $data = [ordered]@{
        format = 'cedarflake-ame-store-submission-v1'
        storeId = $Package.identity.storeId; submissionId = $SubmissionId; status = $Status
        releaseTag = $Package.releaseTag; sourceCommit = $Package.sourceCommit
        applicationVersion = $Package.applicationVersion; packageVersion = $Package.packageVersion
        packageSha256 = $Package.sha256
    }
    [IO.File]::WriteAllText($Path, ($data | ConvertTo-Json), [Text.UTF8Encoding]::new($false))
}
