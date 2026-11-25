[CmdletBinding()]
param(
    [Parameter(Mandatory)][string]$Tag,
    [string]$CandidateRoot,
    [string]$SourceCommit,
    [string]$ReleaseRoot,
    [string]$UnsignedEvidencePath,
    [ValidateSet('x64', 'arm64')][string]$Architecture = 'x64',
    [string]$OutputDirectory,
    [string]$UpdateToolPackagePath,
    [string]$UpdateToolDirectory,
    [Parameter(Mandatory)][string]$UpdateEnginePath,
    [Parameter(Mandatory)][string]$UpdateEngineReceiptPath,
    [string]$PubspecPath,
    [string]$CargoManifestPath,
    [AllowEmptyString()][string]$ExpectedBrokerPublisher = ''
)

$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'quality_common.ps1')
. (Join-Path $PSScriptRoot 'acceptance_scratch_storage.ps1')
. (Join-Path $PSScriptRoot 'release_installer_source.ps1')
. (Join-Path $PSScriptRoot 'release_installer_payload.ps1')
. (Join-Path $PSScriptRoot 'release_journal_broker_common.ps1')
. (Join-Path $PSScriptRoot 'release_update_toolchain.ps1')
. (Join-Path $PSScriptRoot 'release_portable_engine.ps1')
Add-Type -AssemblyName System.IO.Compression.FileSystem

$repository = Get-AmeRepositoryRoot
& (Join-Path $PSScriptRoot 'release_validate_version.ps1') -Tag $Tag `
    -PubspecPath $PubspecPath -CargoManifestPath $CargoManifestPath
$version = $Tag.Substring(1)
if ($CandidateRoot -and ($ReleaseRoot -or $UnsignedEvidencePath)) {
    throw 'Choose one verified candidate or quality payload source'
}
if (-not $CandidateRoot -and -not $UnsignedEvidencePath -and $ReleaseRoot) {
    $possible = Split-Path -Parent ([IO.Path]::GetFullPath($ReleaseRoot))
    if ([IO.Path]::GetFileName($ReleaseRoot.TrimEnd('\', '/')) -ceq 'payload' -and
        (Test-Path -LiteralPath (Join-Path $possible 'manifest.json') -PathType Leaf)) { $CandidateRoot = $possible }
}
if ($CandidateRoot) {
    if ($Architecture -cne 'x64') { throw 'ARM64 packaging requires the native preview payload evidence' }
    if (-not $SourceCommit) {
        $SourceCommit = (Get-Content -LiteralPath (Join-Path $CandidateRoot 'manifest.json') -Raw -Encoding utf8 | ConvertFrom-Json).sourceCommit
    }
    $signingMode = if ($ExpectedBrokerPublisher) { 'signed' } else { 'unsigned' }
    $source = Get-AmeInstallerSource -CandidateRoot $CandidateRoot -Tag $Tag -SourceCommit $SourceCommit `
        -SigningMode $signingMode -ExpectedBrokerPublisher $ExpectedBrokerPublisher
} elseif ($ReleaseRoot -and $UnsignedEvidencePath) {
    if ($ExpectedBrokerPublisher) { throw 'Quality payload evidence only admits unsigned packages' }
    $source = Get-AmeInstallerSource -BundlePath $ReleaseRoot -UnsignedEvidencePath $UnsignedEvidencePath -Architecture $Architecture
} else { throw 'Portable packaging requires a verified candidate or Release payload with quality evidence' }
Assert-AmeInstallerPayloadNames $source.Files -RequireModels $false -Architecture $Architecture
$sourceNames = @($source.Files | ForEach-Object { $_.relativePath })
if ($sourceNames -cnotcontains 'cedarflake_ame_update.exe' -or
    @($sourceNames | Where-Object { $_ -like 'data/classification-models/*' -or
        $_ -in @('sq.version', 'Squirrel.exe', 'Cedarflake Ame_ExecutionStub.exe', 'velopack-LICENSE.txt') }).Count -ne 0) {
    throw 'Portable source is missing its handoff helper or contains model/update-engine payload'
}
if (-not $OutputDirectory) { $OutputDirectory = Join-Path $repository 'build/release-artifacts' }
$output = [IO.Path]::GetFullPath($OutputDirectory).TrimEnd('\', '/')
if (-not $output.StartsWith((Join-Path $repository 'build') + '\', [StringComparison]::OrdinalIgnoreCase) -or
    $output.Equals($source.Root, [StringComparison]::OrdinalIgnoreCase) -or
    $output.StartsWith($source.Root + '\', [StringComparison]::OrdinalIgnoreCase)) {
    throw 'Portable output must remain beneath repository build and outside its source payload'
}
Assert-AmeUnsignedBuildPath $output
$archiveName = "Cedarflake-Ame-$Tag-windows-$Architecture-portable.zip"
$fullName = if ($Architecture -ceq 'x64') { "CedarflakeAme-$version-full.nupkg" } else { "CedarflakeAme-$version-arm64-full.nupkg" }
$archivePath = Join-Path $output $archiveName
$fullPath = Join-Path $output $fullName
$receiptPath = Join-Path $output 'portable-package.json'
foreach ($path in @($archivePath, $fullPath, $receiptPath)) {
    if (Test-Path -LiteralPath $path) { throw 'Portable output already exists; preserve the previous package' }
}
if (-not $UpdateToolPackagePath -and -not $UpdateToolDirectory) {
    $prepared = & (Join-Path $PSScriptRoot 'release_prepare_update_toolchain.ps1') `
        -OutputDirectory (Join-Path $repository 'build/tool-cache/velopack-1.2.161')
    $UpdateToolPackagePath = $prepared.UpdateToolPackagePath
    $UpdateToolDirectory = $prepared.UpdateToolDirectory
}
if (-not $UpdateToolPackagePath -or -not $UpdateToolDirectory) { throw 'Provide both pinned update toolchain paths' }
$lock = Enter-AmeRepositoryToolLock
$leases = [Collections.Generic.List[IDisposable]]::new()
try {
    $tool = Get-AmeUpdateTool -PackagePath $UpdateToolPackagePath -Directory $UpdateToolDirectory
    $engine = Assert-AmeUpdateEngine -Path $UpdateEnginePath -ReceiptPath $UpdateEngineReceiptPath -Architecture $Architecture
    [void][IO.Directory]::CreateDirectory($output)
    foreach ($file in $source.Files) {
        $path = Join-Path $source.Root $file.relativePath
        Assert-AmeUnsignedBuildPath $path
        $lease = [IO.FileStream]::new($path, [IO.FileMode]::Open, [IO.FileAccess]::Read, [IO.FileShare]::Read)
        $leases.Add($lease)
        if ($lease.Length -ne $file.length -or (Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash -ine $file.sha256) {
            throw 'Portable payload differs from its verified Release manifest'
        }
        if ([IO.Path]::GetExtension($path) -in @('.exe', '.dll')) { Assert-AmeWindowsPeImage $path $Architecture }
    }
    if ($Architecture -ceq 'x64') {
        Assert-AmeBrokerBinary -Path (Join-Path $source.Root 'cedarflake_ame_journal_broker.exe') `
            -ExpectedPublisher $ExpectedBrokerPublisher | Out-Null
    } else { Get-AmeBrokerProtocolFacts -Path (Join-Path $source.Root 'cedarflake_ame_journal_broker.exe') | Out-Null }
    foreach ($accelerator in @($false, $true)) {
        $runtime = Get-AmeClassificationRuntimeManifest -Architecture $Architecture -Accelerator:$accelerator
        Assert-AmeClassificationRuntimeDirectory (Join-Path $source.Root $runtime.bundleDirectory) $runtime `
            -CompanionFiles @(Get-AmeMsvcRuntimeCompanionFiles $source.Root -Architecture $Architecture)
    }
    Invoke-AmeAcceptanceScratch -Name "portable-package-$Architecture" -Action {
        param($scratch)
        $pack = Join-Path $scratch 'payload'
        [void][IO.Directory]::CreateDirectory($pack)
        foreach ($file in $source.Files) {
            $target = Join-Path $pack $file.relativePath
            [void][IO.Directory]::CreateDirectory([IO.Path]::GetDirectoryName($target))
            Copy-Item -LiteralPath (Join-Path $source.Root $file.relativePath) -Destination $target
        }
        $license = Join-Path $pack 'velopack-LICENSE.txt'
        [IO.File]::WriteAllText($license, (Get-AmeUpdateLicense), [Text.UTF8Encoding]::new($false))
        $files = @($source.Files) + @([pscustomobject]@{ relativePath = 'velopack-LICENSE.txt';
            length = (Get-Item -LiteralPath $license).Length; sha256 = (Get-FileHash -LiteralPath $license -Algorithm SHA256).Hash.ToLowerInvariant() })
        $generated = Join-Path $scratch 'generated'
        Invoke-AmePortablePack -Tool $tool -ToolDirectory $UpdateToolDirectory -EnginePath $UpdateEnginePath `
            -PackDirectory $pack -OutputDirectory $generated -Scratch $scratch -Version $version -Architecture $Architecture
        $zips = @(Get-ChildItem -LiteralPath $generated -File -Filter '*-Portable.zip')
        $packages = @(Get-ChildItem -LiteralPath $generated -File -Filter '*-full.nupkg')
        if ($zips.Count -ne 1 -or $packages.Count -ne 1 -or
            @(Get-ChildItem -LiteralPath $generated -File | Where-Object Extension -In @('.exe', '.msi')).Count -ne 0) {
            throw 'Velopack did not produce one portable ZIP and one full update package'
        }
        Assert-AmePortableGeneratedPackages -ArchivePath $zips[0].FullName -PackagePath $packages[0].FullName `
            -Files $files -Version $version -Architecture $Architecture -EngineSha256 $engine.engineSha256
        $stagedZip = Join-Path $scratch $archiveName
        Add-AmePortableEnvelope -InputPath $zips[0].FullName -OutputPath $stagedZip
        $verify = if ($ExpectedBrokerPublisher) { 'release_verify_portable_signatures.ps1' } else { 'release_verify_portable_unsigned.ps1' }
        $arguments = @{ ArchivePath = $stagedZip; Tag = $Tag; Architecture = $Architecture;
            PubspecPath = $PubspecPath; CargoManifestPath = $CargoManifestPath }
        if ($ExpectedBrokerPublisher) { $arguments.ExpectedBrokerPublisher = $ExpectedBrokerPublisher }
        & (Join-Path $PSScriptRoot $verify) @arguments | Out-Host
        Move-Item -LiteralPath $stagedZip -Destination $archivePath
        Copy-Item -LiteralPath $packages[0].FullName -Destination $fullPath
        $receipt = [ordered]@{
            format = 'cedarflake-ame-portable-v2'; sourceCommit = $source.SourceCommit
            dirtyWorktree = $source.DirtyWorktree; sourceEvidenceFormat = $source.EvidenceFormat
            releaseTag = $Tag; architecture = $Architecture; version = $version
            portableFile = $archiveName; portableSha256 = (Get-FileHash -LiteralPath $archivePath -Algorithm SHA256).Hash.ToLowerInvariant()
            fullPackageFile = $fullName; fullPackageSha256 = (Get-FileHash -LiteralPath $fullPath -Algorithm SHA256).Hash.ToLowerInvariant()
            updateToolVersion = $script:AmeUpdateToolchain.Version; updateToolSha256 = $script:AmeUpdateToolchain.Sha256
            updateEngine = $engine; payload = @($source.Files)
            generatedPayload = @($files | Where-Object relativePath -CEQ 'velopack-LICENSE.txt')
        }
        [IO.File]::WriteAllText($receiptPath, ($receipt | ConvertTo-Json -Depth 8), [Text.UTF8Encoding]::new($false))
    }
} finally {
    foreach ($lease in $leases) { $lease.Dispose() }
    Exit-AmeRepositoryToolLock $lock
}
Write-Output $archivePath
