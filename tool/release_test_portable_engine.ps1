[CmdletBinding()]
param(
    [Parameter(Mandatory)][string]$UpdateToolPackagePath,
    [Parameter(Mandatory)][string]$UpdateToolDirectory,
    [Parameter(Mandatory)][string]$UpdateEnginePath,
    [Parameter(Mandatory)][string]$UpdateEngineReceiptPath,
    [ValidateSet('x64', 'arm64')][string]$Architecture = 'x64'
)

$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'quality_common.ps1')
. (Join-Path $PSScriptRoot 'acceptance_scratch_storage.ps1')
. (Join-Path $PSScriptRoot 'release_update_toolchain.ps1')
. (Join-Path $PSScriptRoot 'release_portable_engine.ps1')
Add-Type -AssemblyName System.IO.Compression.FileSystem

$lock = Enter-AmeRepositoryToolLock
try {
    $tool = Get-AmeUpdateTool -PackagePath $UpdateToolPackagePath -Directory $UpdateToolDirectory
    $engine = Assert-AmeUpdateEngine -Path $UpdateEnginePath -ReceiptPath $UpdateEngineReceiptPath -Architecture $Architecture
    Invoke-AmeAcceptanceScratch -Name "portable-engine-$Architecture" -Action {
        param($scratch)
        $payload = Join-Path $scratch 'payload'
        [void][IO.Directory]::CreateDirectory($payload)
        $executable = Join-Path $env:SystemRoot 'System32/notepad.exe'
        Assert-AmeWindowsPeImage -Path $executable -Architecture $Architecture
        Copy-Item -LiteralPath $executable -Destination (Join-Path $payload 'cedarflake_ame.exe')
        $names = @('cedarflake_ame_update.exe', 'cedarflake_ame_journal_broker.exe',
            'cedarflake_ame_classification_worker.exe', 'rust_lib_cedarflake_ame.dll',
            'flutter_windows.dll', 'data/app.so', 'data/icudtl.dat',
            'data/flutter_assets/AssetManifest.bin', 'data/flutter_assets/中文 说明.txt', 'velopack-LICENSE.txt') +
            @(Get-AmeClassificationRuntimePayloadNames -Architecture $Architecture) +
            @(Get-AmeMsvcRuntimePayloadPaths -Architecture $Architecture)
        foreach ($name in $names) {
            $path = Join-Path $payload $name
            [void][IO.Directory]::CreateDirectory([IO.Path]::GetDirectoryName($path))
            [IO.File]::WriteAllText($path, $name, [Text.UTF8Encoding]::new($false))
        }
        $files = @(Get-ChildItem -LiteralPath $payload -File -Recurse | ForEach-Object {
            [pscustomobject]@{
                relativePath = [IO.Path]::GetRelativePath($payload, $_.FullName).Replace('\', '/')
                length = $_.Length
                sha256 = (Get-FileHash -LiteralPath $_.FullName -Algorithm SHA256).Hash.ToLowerInvariant()
            }
        })
        $generated = Join-Path $scratch 'generated'
        Invoke-AmePortablePack -Tool $tool -ToolDirectory $UpdateToolDirectory -EnginePath $UpdateEnginePath `
            -PackDirectory $payload -OutputDirectory $generated -Scratch $scratch -Version '1.2.3' -Architecture $Architecture
        $archives = @(Get-ChildItem -LiteralPath $generated -File -Filter '*-Portable.zip')
        $packages = @(Get-ChildItem -LiteralPath $generated -File -Filter '*-full.nupkg')
        if ($archives.Count -ne 1 -or $packages.Count -ne 1) { throw 'Packaging fixture expected one portable and one full package' }
        $arguments = @{ ArchivePath = $archives[0].FullName; PackagePath = $packages[0].FullName
            Files = $files; Version = '1.2.3'; Architecture = $Architecture; EngineSha256 = $engine.engineSha256 }
        Assert-AmePortableGeneratedPackages @arguments
        $wrapped = Join-Path $scratch 'wrapped.zip'
        Add-AmePortableEnvelope -InputPath $archives[0].FullName -OutputPath $wrapped
        $zip = [IO.Compression.ZipFile]::OpenRead($wrapped)
        try {
            $entries = Get-AmePortableArchiveEntries $zip
            Assert-AmePortableLayout -Entries $entries -Version '1.2.3' -Architecture $Architecture
            Assert-AmePortablePayloadEntries -Entries $entries -Files $files -Prefix 'Cedarflake-Ame/current/' -GeneratedNames @('sq.version')
        } finally { $zip.Dispose() }
        $package = [IO.Compression.ZipFile]::Open($packages[0].FullName, [IO.Compression.ZipArchiveMode]::Update)
        try {
            $package.GetEntry('lib/app/Squirrel.exe').Delete()
            $writer = [IO.StreamWriter]::new($package.CreateEntry('lib/app/Squirrel.exe').Open(), [Text.UTF8Encoding]::new($false))
            try { $writer.Write('unverified updater') } finally { $writer.Dispose() }
        } finally { $package.Dispose() }
        $rejected = $false
        try { Assert-AmePortableGeneratedPackages @arguments }
        catch {
            if ($_.Exception.Message -notlike '*disagree on the verified engine*') { throw }
            $rejected = $true
        }
        if (-not $rejected) { throw 'Full package with an unverified updater was admitted' }
        Write-Host 'Pinned Velopack packaging, payload preservation and updater substitution fixtures passed'
    }
} finally { Exit-AmeRepositoryToolLock $lock }
