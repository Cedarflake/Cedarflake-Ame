[CmdletBinding()]
param(
    [Parameter(Mandatory)][string]$ParentPath,
    [Parameter(Mandatory)][string]$HelperPath,
    [Parameter(Mandatory)][string]$UpdateToolPackagePath,
    [Parameter(Mandatory)][string]$UpdateToolDirectory,
    [Parameter(Mandatory)][string]$UpdateEnginePath,
    [Parameter(Mandatory)][string]$UpdateEngineReceiptPath,
    [string]$OutputDirectory = 'build/test/fixtures/application-update'
)

$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'quality_common.ps1')
. (Join-Path $PSScriptRoot 'acceptance_scratch_storage.ps1')
. (Join-Path $PSScriptRoot 'release_update_toolchain.ps1')
. (Join-Path $PSScriptRoot 'release_portable_engine.ps1')
Add-Type -AssemblyName System.IO.Compression.FileSystem
$output = Assert-AmeAcceptanceScratchPath $OutputDirectory
if (-not $output.StartsWith((Join-Path (Get-AmeRepositoryRoot) 'build/test/fixtures') + '\', [StringComparison]::OrdinalIgnoreCase)) {
    throw 'Reusable updater fixtures must remain beneath build/test/fixtures'
}
if (Test-Path -LiteralPath $output) { throw 'Preserve the existing fixture baseline; choose an unused output directory' }
$lock = Enter-AmeRepositoryToolLock
try {
    $tool = Get-AmeUpdateTool -PackagePath $UpdateToolPackagePath -Directory $UpdateToolDirectory
    $engine = Assert-AmeUpdateEngine -Path $UpdateEnginePath -ReceiptPath $UpdateEngineReceiptPath -Architecture x64
    Assert-AmeWindowsPeImage $ParentPath x64
    Assert-AmeWindowsPeImage $HelperPath x64
    $oldEngine = Join-Path $UpdateToolDirectory 'vendor/update_x64.exe'
    $oldEngineHash = (Get-FileHash -LiteralPath $oldEngine -Algorithm SHA256).Hash.ToLowerInvariant()
    Invoke-AmeAcceptanceScratch -Name 'portable-update-packaging' -Action {
        param($scratch)
        $products = @{}
        foreach ($version in @('1.2.3', '1.2.4')) {
            $stage = Join-Path $scratch $version
            $payload = Join-Path $stage 'payload'
            [void][IO.Directory]::CreateDirectory($payload)
            Copy-Item -LiteralPath $ParentPath -Destination (Join-Path $payload 'cedarflake_ame.exe')
            Copy-Item -LiteralPath $HelperPath -Destination (Join-Path $payload 'cedarflake_ame_update.exe')
            $names = @('cedarflake_ame_journal_broker.exe', 'cedarflake_ame_classification_worker.exe',
                'rust_lib_cedarflake_ame.dll', 'flutter_windows.dll', 'data/app.so', 'data/icudtl.dat',
                'data/flutter_assets/AssetManifest.bin', 'velopack-LICENSE.txt') +
                @(Get-AmeClassificationRuntimePayloadNames) + @(Get-AmeMsvcRuntimePayloadPaths)
            foreach ($name in $names) {
                $path = Join-Path $payload $name
                [void][IO.Directory]::CreateDirectory([IO.Path]::GetDirectoryName($path))
                [IO.File]::WriteAllText($path, $name, [Text.UTF8Encoding]::new($false))
            }
            [IO.File]::WriteAllText((Join-Path $payload 'fixture-version.txt'), $version, [Text.UTF8Encoding]::new($false))
            $files = @(Get-ChildItem -LiteralPath $payload -File -Recurse | ForEach-Object {
                [pscustomobject]@{ relativePath = [IO.Path]::GetRelativePath($payload, $_.FullName).Replace('\', '/')
                    length = $_.Length; sha256 = (Get-FileHash -LiteralPath $_.FullName -Algorithm SHA256).Hash.ToLowerInvariant() }
            })
            $selected = if ($version -ceq '1.2.3') { $oldEngine } else { $UpdateEnginePath }
            $selectedHash = if ($version -ceq '1.2.3') { $oldEngineHash } else { $engine.engineSha256 }
            $generated = Join-Path $stage 'generated'
            Invoke-AmePortablePack -Tool $tool -ToolDirectory $UpdateToolDirectory -EnginePath $selected `
                -PackDirectory $payload -OutputDirectory $generated -Scratch $stage -Version $version -Architecture x64
            $zip = @(Get-ChildItem -LiteralPath $generated -File -Filter '*-Portable.zip')
            $package = @(Get-ChildItem -LiteralPath $generated -File -Filter '*-full.nupkg')
            if ($zip.Count -ne 1 -or $package.Count -ne 1) { throw 'Fixture packaging output differs' }
            Assert-AmePortableGeneratedPackages -ArchivePath $zip[0].FullName -PackagePath $package[0].FullName `
                -Files $files -Version $version -Architecture x64 -EngineSha256 $selectedHash
            $products[$version] = @{ Zip = $zip[0].FullName; Package = $package[0].FullName }
        }
        [void][IO.Directory]::CreateDirectory($output)
        Copy-Item -LiteralPath $products['1.2.3'].Zip -Destination (Join-Path $output 'A.zip')
        Copy-Item -LiteralPath $products['1.2.4'].Package -Destination (Join-Path $output 'B.nupkg')
        Copy-Item -LiteralPath (Join-Path $PSScriptRoot 'acceptance_portable_update_guest.ps1') -Destination (Join-Path $output 'guest.ps1')
        Copy-Item -LiteralPath (Join-Path $PSScriptRoot 'acceptance_guest_desktop.ps1') -Destination (Join-Path $output 'desktop.ps1')
        Copy-Item -LiteralPath (Join-Path $PSScriptRoot 'acceptance_guest_user.ps1') -Destination (Join-Path $output 'user.ps1')
        $inputs = @(Get-ChildItem -LiteralPath $output -File | ForEach-Object {
            [pscustomobject]@{ name = $_.Name; length = $_.Length
                sha256 = (Get-FileHash -LiteralPath $_.FullName -Algorithm SHA256).Hash.ToLowerInvariant() }
        })
        $package = $inputs | Where-Object name -CEQ 'B.nupkg'
        $plan = [ordered]@{ format = 'ame-portable-update-fixture-v1'; files = $inputs
            packageSha256 = $package.sha256; packageLength = $package.length
            engineSha256 = $engine.engineSha256; oldEngineSha256 = $oldEngineHash
            helperSha256 = (Get-FileHash -LiteralPath $HelperPath -Algorithm SHA256).Hash.ToLowerInvariant()
            parentSha256 = (Get-FileHash -LiteralPath $ParentPath -Algorithm SHA256).Hash.ToLowerInvariant() }
        [IO.File]::WriteAllText((Join-Path $output 'plan.json'), ($plan | ConvertTo-Json -Depth 6), [Text.UTF8Encoding]::new($false))
    }
} finally { Exit-AmeRepositoryToolLock $lock }
Write-Output $output
