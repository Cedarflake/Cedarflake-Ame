[CmdletBinding()]
param(
    [Parameter(Mandatory)][string]$PackageDirectory,
    [Parameter(Mandatory)][string]$OutputDirectory,
    [ValidateSet('Submit', 'Status')][string]$Operation = 'Status',
    [string]$SubmissionId
)

$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'release_store_identity.ps1')
. (Join-Path $PSScriptRoot 'release_store_api.ps1')
. (Join-Path $PSScriptRoot 'acceptance_scratch_storage.ps1')
$receipt = Read-AmeStorePackageReceipt (Join-Path $PackageDirectory 'msix-package.json')
$package = Join-Path $PackageDirectory $receipt.fileName
$lease = [IO.FileStream]::new($package, [IO.FileMode]::Open, [IO.FileAccess]::Read, [IO.FileShare]::Read)
try {
    if ((Get-FileHash -LiteralPath $package -Algorithm SHA256).Hash -ine $receipt.sha256) { throw 'Store upload package changed' }
    Assert-AmeMsixArchive $package (@($receipt.payload) + @($receipt.generatedPayload))
    Assert-AmeStoreApplicationArchive $package $receipt
    $output = Assert-AmeAcceptanceScratchPath $OutputDirectory
    [void][IO.Directory]::CreateDirectory($output)
    $resultPath = Join-Path $output 'store-submission.json'
    $token = Get-AmeStoreAccessToken
    $appPath = "applications/$($receipt.identity.storeId)"
    if ($SubmissionId) {
        if ($SubmissionId -cnotmatch '^[1-9][0-9]*$') { throw 'Invalid Store submission ID' }
        $submission = Invoke-AmeStoreRequest Get "$appPath/submissions/$SubmissionId" $token
        if ([string]$submission.id -cne $SubmissionId) { throw 'Store returned another submission identity' }
        Assert-AmeStoreSubmissionIdentity $submission $receipt
    } else {
        if ($Operation -cne 'Submit') { throw 'Status requires the recorded submission ID' }
        $application = Invoke-AmeStoreRequest Get $appPath $token
        if (-not $application.lastPublishedApplicationSubmission) { throw 'Complete the first Store publication in Partner Center' }
        if ($application.pendingApplicationSubmission) { throw 'An existing Store draft must be resolved; it will not be deleted' }
        $submission = Invoke-AmeStoreRequest Post "$appPath/submissions" $token
        $SubmissionId = [string]$submission.id
        if ($SubmissionId -cnotmatch '^[1-9][0-9]*$') { throw 'Store returned an invalid draft ID' }
        Save-AmeStoreSubmissionReceipt $resultPath $receipt $SubmissionId 'PendingCommit'
        $submission = Set-AmeStoreSubmissionPackage $submission $receipt
        $submission = Invoke-AmeStoreRequest Put "$appPath/submissions/$SubmissionId" $token $submission
        Assert-AmeStoreSubmissionIdentity $submission $receipt
    }
    Save-AmeStoreSubmissionReceipt $resultPath $receipt $SubmissionId $submission.status
    if ($Operation -ceq 'Submit' -and $submission.status -ceq 'PendingCommit') {
        Invoke-AmeAcceptanceScratch -Name 'store-submission-upload' -Action {
            param($scratch)
            Send-AmeStorePackageArchive $submission.fileUploadUrl $package (Join-Path $scratch 'upload.zip')
        }
        # Re-read the exact draft before commit; never let a changed listing select another package.
        $current = Invoke-AmeStoreRequest Get "$appPath/submissions/$SubmissionId" $token
        if ([string]$current.id -cne $SubmissionId) { throw 'Store returned another submission identity' }
        Assert-AmeStoreSubmissionIdentity $current $receipt
        if ($current.status -cne 'PendingCommit') { throw 'Store draft changed during upload; inspect its recorded status' }
        Invoke-AmeStoreRequest Post "$appPath/submissions/$SubmissionId/commit" $token | Out-Null
    }
    $status = Invoke-AmeStoreRequest Get "$appPath/submissions/$SubmissionId/status" $token
    Save-AmeStoreSubmissionReceipt $resultPath $receipt $SubmissionId $status.status
    $disposition = Get-AmeStoreStatusDisposition $status.status
    Write-Host "Store submission $SubmissionId`: $($status.status)"
    if ($disposition -ceq 'failed') { throw 'Store submission failed; inspect its certification report in Partner Center' }
} finally { $lease.Dispose() }
