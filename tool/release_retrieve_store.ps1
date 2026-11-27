[CmdletBinding()]
param(
    [Parameter(Mandatory)][ValidatePattern('^[1-9][0-9]*$')][string]$PackageRun,
    [Parameter(Mandatory)][ValidatePattern('^[1-9][0-9]*$')][string]$PackageArtifact,
    [Parameter(Mandatory)][ValidatePattern('^[1-9][0-9]*$')][string]$SubmissionId,
    [Parameter(Mandatory)][string]$OutputDirectory,
    [ValidateRange(0, 45)][int]$WaitMinutes = 0,
    [switch]$Publish
)

$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'release_store_identity.ps1')
. (Join-Path $PSScriptRoot 'release_store_api.ps1')
. (Join-Path $PSScriptRoot 'release_store_artifacts.ps1')

if (-not $IsWindows) { throw 'Store package retrieval requires Windows with an authenticated WinGet account' }
$repository = 'Cedarflake/Cedarflake-Ame'
$output = Assert-AmeAcceptanceScratchPath $OutputDirectory
$build = Join-Path (Get-AmeRepositoryRoot) 'build'
if (-not $output.StartsWith($build + '\', [StringComparison]::OrdinalIgnoreCase)) {
    throw 'Store retrieval output must remain beneath build'
}
if (Test-Path -LiteralPath $output) { throw 'Store retrieval requires a new output directory' }
$winget = Get-Command winget -CommandType Application -ErrorAction Stop | Select-Object -First 1

Invoke-AmeAcceptanceScratch -Name 'store-retrieval' -Action {
    param($scratch)
    if ($output.StartsWith($scratch + '\', [StringComparison]::OrdinalIgnoreCase) -or
        $output.Equals($scratch, [StringComparison]::OrdinalIgnoreCase)) {
        throw 'Store retrieval output cannot be inside its disposable scratch directory'
    }
    $packageDirectory = Receive-AmeStoreArtifact -Repository $repository -RunId $PackageRun `
        -ArtifactId $PackageArtifact -Kind Msix -Directory (Join-Path $scratch 'candidate')
    $lock = Enter-AmeRepositoryToolLock
    try {
        $receipt = Read-AmeStorePackageReceipt (Join-Path $packageDirectory 'msix-package.json')
        Assert-AmeAcceptanceScratchPath $output | Out-Null
        New-Item -ItemType Directory -Path $output -ErrorAction Stop | Out-Null
    } finally { Exit-AmeRepositoryToolLock $lock }
    Assert-AmeStoreReleaseTag $repository $receipt

    $token = Get-AmeStoreAccessToken
    $submissionPath = "applications/$($receipt.identity.storeId)/submissions/$SubmissionId"
    $wait = [Diagnostics.Stopwatch]::StartNew()
    try {
        do {
            $submission = Invoke-AmeStoreRequest Get $submissionPath $token
            if ([string]$submission.id -cne $SubmissionId) { throw 'Store returned another submission identity' }
            Assert-AmeStoreSubmissionIdentity $submission $receipt
            $status = Invoke-AmeStoreRequest Get "$submissionPath/status" $token
            Save-AmeStoreSubmissionReceipt (Join-Path $output 'store-submission.json') `
                $receipt $SubmissionId $status.status
            $disposition = Get-AmeStoreStatusDisposition $status.status
            Write-Host "Store submission $SubmissionId`: $($status.status)"
            if ($disposition -ceq 'published') { break }
            if ($disposition -ceq 'failed') {
                throw 'Store submission failed; inspect its certification report in Partner Center'
            }
            $remainingSeconds = ($WaitMinutes * 60) - $wait.Elapsed.TotalSeconds
            if ($remainingSeconds -le 0) {
                Write-Host 'Store publication is pending. No package was downloaded or uploaded; retry with a new output directory.'
                return
            }
            Start-Sleep -Seconds ([Math]::Min(30, [Math]::Ceiling($remainingSeconds)))
        } while ($true)
    } finally {
        $wait.Stop()
        $token = $null
    }

    $downloadDirectory = Join-Path $scratch 'download'
    [void][IO.Directory]::CreateDirectory($downloadDirectory)
    & $winget.Source download --id $receipt.identity.storeId --exact --source msstore `
        --architecture x64 --platform Windows.Desktop --download-directory $downloadDirectory `
        --skip-license --skip-dependencies --accept-source-agreements --accept-package-agreements `
        --authentication-mode silent --disable-interactivity | Out-Host
    if ($LASTEXITCODE -ne 0) {
        throw 'WinGet Store download failed. Confirm that this Windows user has a valid Microsoft Entra sign-in and offline-download access.'
    }

    # WinGet may place its main package inside a product directory beneath the requested destination.
    $entries = @(Get-ChildItem -LiteralPath $downloadDirectory -Recurse -Force)
    if (@($entries | Where-Object { ($_.Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0 }).Count -gt 0) {
        throw 'WinGet download contains a reparse point'
    }
    $packages = @($entries | Where-Object {
        -not $_.PSIsContainer -and $_.Extension.ToLowerInvariant() -in @('.msix', '.appx', '.msixbundle', '.appxbundle')
    })
    if ($packages.Count -ne 1) { throw 'WinGet did not return one unique Store application package' }
    $download = Assert-AmeAcceptanceScratchPath $packages[0].FullName
    $extension = $packages[0].Extension.ToLowerInvariant()
    $fileName = "Cedarflake-Ame-$($receipt.releaseTag)-windows-x64-store$extension"
    $destination = Join-Path $output $fileName
    $validation = Join-Path $scratch 'validation'
    [void][IO.Directory]::CreateDirectory($validation)
    $sourceLease = [IO.FileStream]::new($download, [IO.FileMode]::Open, [IO.FileAccess]::Read, [IO.FileShare]::Read)
    $outputLease = $null
    try {
        $lock = Enter-AmeRepositoryToolLock
        try {
            Assert-AmeStoreDownload $download $receipt $validation
            $hash = (Get-FileHash -LiteralPath $download -Algorithm SHA256).Hash.ToLowerInvariant()
            Assert-AmeAcceptanceScratchPath $destination | Out-Null
            [IO.File]::Copy($download, $destination, $false)
            $outputLease = [IO.FileStream]::new($destination, [IO.FileMode]::Open, [IO.FileAccess]::Read, [IO.FileShare]::Read)
            if ((Get-FileHash -LiteralPath $destination -Algorithm SHA256).Hash.ToLowerInvariant() -cne $hash) {
                throw 'Retrieved Store package changed while copying to the output directory'
            }
            $signedReceipt = [ordered]@{
                format = 'cedarflake-ame-store-signed-v1'
                storeId = $receipt.identity.storeId; submissionId = $SubmissionId
                packageRun = $PackageRun; packageArtifact = $PackageArtifact
                releaseTag = $receipt.releaseTag; sourceCommit = $receipt.sourceCommit
                applicationVersion = $receipt.applicationVersion; packageVersion = $receipt.packageVersion
                architecture = $receipt.architecture; unsignedPackageSha256 = $receipt.sha256
                fileName = $fileName; length = $outputLease.Length; sha256 = $hash
                retrievedAt = [DateTimeOffset]::UtcNow.ToString('o')
            }
            [IO.File]::WriteAllText((Join-Path $output 'store-signed-package.json'),
                ($signedReceipt | ConvertTo-Json), [Text.UTF8Encoding]::new($false))
        } finally { Exit-AmeRepositoryToolLock $lock }
        if ($Publish) {
            Publish-AmeStoreAsset $repository $receipt $destination
        } else {
            Write-Host 'Store package verified and saved. GitHub Release upload was not requested.'
        }
        Write-Output $destination
    } finally {
        if ($null -ne $outputLease) { $outputLease.Dispose() }
        $sourceLease.Dispose()
    }
}
