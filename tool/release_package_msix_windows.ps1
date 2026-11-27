[CmdletBinding()]
param(
    [Parameter(Mandatory)][string]$Tag,
    [string]$CandidateRoot,
    [string]$SourceCommit,
    [string]$ReleaseRoot,
    [string]$UnsignedEvidencePath,
    [string]$ModelSourceDirectory,
    [ValidateSet('x64', 'arm64')][string]$Architecture = 'x64',
    [string]$OutputDirectory
)

$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'quality_common.ps1')
. (Join-Path $PSScriptRoot 'acceptance_scratch_storage.ps1')
. (Join-Path $PSScriptRoot 'release_installer_source.ps1')
. (Join-Path $PSScriptRoot 'release_installer_payload.ps1')
. (Join-Path $PSScriptRoot 'release_msix_manifest.ps1')
. (Join-Path $PSScriptRoot 'release_msix_assets.ps1')
. (Join-Path $PSScriptRoot 'release_msix_payload.ps1')

$repository = Get-AmeRepositoryRoot
& (Join-Path $PSScriptRoot 'release_validate_version.ps1') -Tag $Tag
$applicationVersion = [regex]::Match([IO.File]::ReadAllText((Join-Path $repository 'pubspec.yaml'), [Text.Encoding]::UTF8), '(?m)^version:\s*(\S+)\s*$').Groups[1].Value
$version = Get-AmeMsixVersion -Tag $Tag -ApplicationVersion $applicationVersion
if ($CandidateRoot) {
    if ($ReleaseRoot -or $UnsignedEvidencePath -or $Architecture -cne 'x64') { throw 'Choose one admitted x64 candidate or architecture-specific Release payload' }
    if (-not $SourceCommit) { $SourceCommit = (Get-Content -LiteralPath (Join-Path $CandidateRoot 'manifest.json') -Raw -Encoding UTF8 | ConvertFrom-Json).sourceCommit }
    $source = Get-AmeInstallerSource -CandidateRoot $CandidateRoot -Tag $Tag -SourceCommit $SourceCommit
} elseif ($ReleaseRoot -and $UnsignedEvidencePath) {
    $source = Get-AmeInstallerSource -BundlePath $ReleaseRoot -UnsignedEvidencePath $UnsignedEvidencePath -Architecture $Architecture
} else { throw 'MSIX requires a verified candidate or Release payload with quality evidence' }
Assert-AmeInstallerPayloadNames -Files $source.Files -RequireModels $false -Architecture $Architecture
if (-not $OutputDirectory) { $OutputDirectory = Join-Path $repository 'build/release-artifacts/msix' }
$output = Assert-AmeAcceptanceScratchPath $OutputDirectory
if (-not $output.StartsWith((Join-Path $repository 'build') + '\', [StringComparison]::OrdinalIgnoreCase) -or
    $output.Equals($source.Root, [StringComparison]::OrdinalIgnoreCase) -or
    $output.StartsWith($source.Root + '\', [StringComparison]::OrdinalIgnoreCase)) { throw 'MSIX output must remain beneath build and outside its source' }
$msixArtifactName = "Cedarflake-Ame-$Tag-windows-$Architecture.msix"
$package = Join-Path $output $msixArtifactName
$msixReceiptPath = Join-Path $output 'msix-package.json'
foreach ($path in @($package, $msixReceiptPath)) { if (Test-Path -LiteralPath $path) { throw 'MSIX output already exists' } }
$makeAppx = Get-AmeMsixSdkTool 'makeappx.exe'
$lock = Enter-AmeRepositoryToolLock
$leases = [Collections.Generic.List[IDisposable]]::new()
try {
    $modelManifest = Get-AmeClassificationModelsManifest
    $modelNames = @(Get-AmeClassificationModelsPayloadNames $modelManifest)
    $bundledModels = @($source.Files | Where-Object { $_.relativePath -cin $modelNames })
    $additionalModels = @()
    if ($bundledModels.Count -eq 0) {
        $modelDirectory = Join-Path $repository "build/classification-models/$($modelManifest.package)"
        & (Join-Path $PSScriptRoot 'release_prepare_classification_models.ps1') `
            -OutputDirectory $modelDirectory -SourceDirectory $ModelSourceDirectory
        $additionalModels = @(Get-AmeClassificationModelsPayload $modelDirectory -Manifest $modelManifest)
    } else {
        Assert-AmeClassificationModelsDirectory (Join-Path $source.Root $modelManifest.bundleDirectory) $modelManifest
    }
    $completePayload = @($source.Files) + $additionalModels
    Assert-AmeInstallerPayloadNames $completePayload -ModelManifest $modelManifest -Architecture $Architecture
    $payload = @($completePayload | Where-Object { Test-AmeMsixPayloadFile $_.relativePath })
    foreach ($file in $additionalModels) {
        $path = Join-Path $modelDirectory ([IO.Path]::GetFileName($file.relativePath))
        $leases.Add([IO.FileStream]::new($path, [IO.FileMode]::Open, [IO.FileAccess]::Read, [IO.FileShare]::Read))
        Assert-AmeClassificationRuntimeArtifact $path $file
    }
    foreach ($file in $source.Files) {
        $path = Join-Path $source.Root $file.relativePath
        Assert-AmeUnsignedBuildPath $path
        $lease = [IO.FileStream]::new($path, [IO.FileMode]::Open, [IO.FileAccess]::Read, [IO.FileShare]::Read)
        $leases.Add($lease)
        if ($lease.Length -ne $file.length -or (Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash -ine $file.sha256) { throw 'MSIX input differs from admitted payload' }
        if ([IO.Path]::GetExtension($path) -in @('.exe', '.dll')) { Assert-AmeWindowsPeImage $path $Architecture }
    }
    foreach ($accelerator in @($false, $true)) {
        $runtime = Get-AmeClassificationRuntimeManifest -Architecture $Architecture -Accelerator:$accelerator
        Assert-AmeClassificationRuntimeDirectory (Join-Path $source.Root $runtime.bundleDirectory) $runtime -CompanionFiles @(Get-AmeMsvcRuntimeCompanionFiles $source.Root -Architecture $Architecture)
    }
    Invoke-AmeAcceptanceScratch -Name "msix-package-$Architecture" -Action {
        param($scratch)
        $pack = Join-Path $scratch 'payload'
        [void][IO.Directory]::CreateDirectory($pack)
        foreach ($file in $payload) {
            $target = Join-Path $pack $file.relativePath
            [void][IO.Directory]::CreateDirectory([IO.Path]::GetDirectoryName($target))
            $inputPath = Join-Path $source.Root $file.relativePath
            if ($additionalModels.Count -gt 0 -and $file.relativePath -cin $modelNames) {
                $inputPath = Join-Path $modelDirectory ([IO.Path]::GetFileName($file.relativePath))
            }
            Copy-Item -LiteralPath $inputPath -Destination $target
        }
        New-AmeMsixManifest -Path (Join-Path $pack 'AppxManifest.xml') -Version $version -Architecture $Architecture
        New-AmeMsixAssets -Directory (Join-Path $pack 'Assets')
        New-AmeMsixResourceIndex -Root $pack -ConfigurationPath (Join-Path $scratch 'priconfig.xml')
        $assetNames = @(Get-AmeMsixAssetDefinitions | ForEach-Object { "Assets/$($_.name)" })
        $generated = @('AppxManifest.xml', 'resources.pri') + $assetNames | ForEach-Object {
            $path = Join-Path $pack $_
            [pscustomobject]@{ relativePath = $_; length = (Get-Item -LiteralPath $path).Length; sha256 = (Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash.ToLowerInvariant() }
        }
        $staged = Join-Path $scratch $msixArtifactName
        Invoke-AmeChecked $makeAppx @('pack', '/d', $pack, '/p', $staged, '/o') | Out-Host
        Assert-AmeMsixArchive -Path $staged -Files (@($payload) + @($generated))
        [void][IO.Directory]::CreateDirectory($output)
        Move-Item -LiteralPath $staged -Destination $package
        $result = [ordered]@{
            format = 'cedarflake-ame-msix-v1'; sourceCommit = $source.SourceCommit; dirtyWorktree = $source.DirtyWorktree
            sourceEvidenceFormat = $source.EvidenceFormat; releaseTag = $Tag; architecture = $Architecture
            packageVersion = $version; identity = Get-AmeMsixIdentity; windowsSdk = '10.0.26100.0'
            fileName = $msixArtifactName; sha256 = (Get-FileHash -LiteralPath $package -Algorithm SHA256).Hash.ToLowerInvariant()
            payload = @($payload); generatedPayload = @($generated)
        }
        [IO.File]::WriteAllText($msixReceiptPath, ($result | ConvertTo-Json -Depth 8), [Text.UTF8Encoding]::new($false))
    }
} finally {
    foreach ($lease in $leases) { $lease.Dispose() }
    Exit-AmeRepositoryToolLock $lock
}
Write-Output $package
