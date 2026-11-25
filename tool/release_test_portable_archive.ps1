$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'acceptance_scratch_storage.ps1')
. (Join-Path $PSScriptRoot 'release_portable_payload.ps1')
Add-Type -AssemblyName System.IO.Compression.FileSystem

function Assert-PortableRejection {
    param([scriptblock]$Action, [string]$Expected, [string]$Detail)
    $failure = $null
    try { & $Action } catch { $failure = $_ }
    if ($null -eq $failure -or $failure.Exception.Message -notlike $Expected) { throw $Detail }
}

Invoke-AmeAcceptanceScratch -Name 'portable-archive-tests' -Action {
    param($scratch)
    $tag = 'v1.2.3'
    $name = "Cedarflake-Ame-$tag-windows-x64-portable.zip"
    $root = Join-Path $scratch 'fixture/Cedarflake-Ame'
    $current = Join-Path $root 'current'
    [void][IO.Directory]::CreateDirectory((Join-Path $current 'data/flutter_assets'))
    $pubspec = Join-Path $scratch 'pubspec.yaml'
    $cargo = Join-Path $scratch 'Cargo.toml'
    [IO.File]::WriteAllText($pubspec, "name: fixture`nversion: 1.2.3+4`n", [Text.UTF8Encoding]::new($false))
    [IO.File]::WriteAllText($cargo, "[package]`nname = `"fixture`"`nversion = `"1.2.3`"`n", [Text.UTF8Encoding]::new($false))
    $signedSource = Join-Path $env:SystemRoot 'System32/notepad.exe'
    $signature = Get-AuthenticodeSignature -LiteralPath $signedSource
    if ($signature.Status -ne 'Valid' -or $null -eq $signature.SignerCertificate) { throw 'Windows signed fixture is unavailable' }
    foreach ($file in @('cedarflake_ame.exe', 'cedarflake_ame_update.exe', 'cedarflake_ame_journal_broker.exe', 'cedarflake_ame_classification_worker.exe')) {
        Copy-Item -LiteralPath $signedSource -Destination (Join-Path $current $file)
    }
    Copy-Item -LiteralPath $signedSource -Destination (Join-Path $root 'Update.exe')
    Copy-Item -LiteralPath $signedSource -Destination (Join-Path $root 'Cedarflake Ame.exe')
    [IO.File]::WriteAllBytes((Join-Path $root '.portable'), [byte[]]::new(0))
    foreach ($file in @('rust_lib_cedarflake_ame.dll', 'flutter_windows.dll', 'data/app.so',
        'data/icudtl.dat', 'data/flutter_assets/AssetManifest.bin', 'velopack-LICENSE.txt') +
        @(Get-AmeClassificationRuntimePayloadNames) + @(Get-AmeMsvcRuntimePayloadPaths)) {
        $target = Join-Path $current $file
        [void][IO.Directory]::CreateDirectory([IO.Path]::GetDirectoryName($target))
        [IO.File]::WriteAllText($target, $file, [Text.UTF8Encoding]::new($false))
    }
    $manifest = '<package><metadata><id>CedarflakeAme</id><version>1.2.3</version><mainExe>cedarflake_ame.exe</mainExe><os>win</os><rid>win-x64</rid><machineArchitecture>x64</machineArchitecture></metadata></package>'
    [IO.File]::WriteAllText((Join-Path $current 'sq.version'), $manifest, [Text.UTF8Encoding]::new($false))
    $archive = Join-Path $scratch $name
    [IO.Compression.ZipFile]::CreateFromDirectory((Split-Path -Parent $root), $archive)
    $arguments = @{ Tag = $tag; PubspecPath = $pubspec; CargoManifestPath = $cargo }
    $verify = Join-Path $PSScriptRoot 'release_verify_portable_archive.ps1'
    & $verify -ArchivePath $archive @arguments
    $missingNames = @('.portable', 'Update.exe', 'Cedarflake Ame.exe', 'current/cedarflake_ame_update.exe',
        'current/cedarflake_ame_journal_broker.exe', 'current/rust_lib_cedarflake_ame.dll') +
        @((@(Get-AmeClassificationRuntimePayloadNames) + @(Get-AmeMsvcRuntimePayloadPaths)) | ForEach-Object { "current/$_" })
    foreach ($missing in $missingNames) {
        $directory = Join-Path $scratch ([guid]::NewGuid().ToString('N'))
        [void][IO.Directory]::CreateDirectory($directory)
        $changed = Join-Path $directory $name
        Copy-Item -LiteralPath $archive -Destination $changed
        $zip = [IO.Compression.ZipFile]::Open($changed, [IO.Compression.ZipArchiveMode]::Update)
        try { $zip.GetEntry("Cedarflake-Ame/$missing").Delete() } finally { $zip.Dispose() }
        Assert-PortableRejection { & $verify -ArchivePath $changed @arguments } `
            "*missing required entry: Cedarflake-Ame/$missing*" "Missing portable payload was admitted: $missing"
    }
    $unsigned = Join-Path $PSScriptRoot 'release_verify_portable_unsigned.ps1'
    Assert-PortableRejection { & $unsigned -ArchivePath $archive @arguments } `
        '*Unsigned portable executable unexpectedly carries an Authenticode signature*' 'Signed fixture passed unsigned verification'
    $signed = Join-Path $PSScriptRoot 'release_verify_portable_signatures.ps1'
    Assert-PortableRejection { & $signed -ArchivePath $archive -ExpectedBrokerPublisher 'CN=Invalid' @arguments } `
        '*signer does not match*' 'Portable signature verification accepted the wrong publisher'
    $bytes = [IO.File]::ReadAllBytes((Join-Path $current 'cedarflake_ame.exe'))
    $bytes[4096] = $bytes[4096] -bxor 1
    [IO.File]::WriteAllBytes((Join-Path $current 'cedarflake_ame.exe'), $bytes)
    $tampered = Join-Path $scratch 'tampered'
    [void][IO.Directory]::CreateDirectory($tampered)
    $tamperedArchive = Join-Path $tampered $name
    [IO.Compression.ZipFile]::CreateFromDirectory((Split-Path -Parent $root), $tamperedArchive)
    Assert-PortableRejection { & $signed -ArchivePath $tamperedArchive -ExpectedBrokerPublisher $signature.SignerCertificate.Subject @arguments } `
        '*valid Authenticode signature*' 'Portable signature verification accepted a tampered signed application'
    foreach ($unsafe in @('../outside.txt', 'Cedarflake-Ame/current/file:stream', 'Cedarflake-Ame/current/CON.txt', 'Cedarflake-Ame/current/file.')) {
        $directory = Join-Path $scratch ([guid]::NewGuid().ToString('N'))
        [void][IO.Directory]::CreateDirectory($directory)
        $changed = Join-Path $directory $name
        $zip = [IO.Compression.ZipFile]::Open($changed, [IO.Compression.ZipArchiveMode]::Create)
        try { [void]$zip.CreateEntry($unsafe) } finally { $zip.Dispose() }
        Assert-PortableRejection { & $verify -ArchivePath $changed @arguments } '*unsafe or duplicate path*' 'Unsafe archive path was admitted'
    }
    foreach ($case in @(
        @{ Name = 'Cedarflake-Ame/current/sq.version'; Content = $manifest.Replace('1.2.3', '1.2.4'); Error = '*release identity*' },
        @{ Name = 'Cedarflake-Ame/.portable'; Content = 'installed'; Error = '*marker must be empty*' },
        @{ Name = 'Cedarflake-Ame/.portable/'; Content = ''; Error = '*missing required entry: Cedarflake-Ame/.portable*' },
        @{ Name = 'Cedarflake-Ame/unexpected.exe'; Content = 'unexpected'; Error = '*unexpected root*' },
        @{ Name = ('Cedarflake-Ame/current/' + (Get-AmeClassificationModelsManifest).bundleDirectory + '/model.onnx'); Content = 'model'; Error = '*model*' }
    )) {
        $directory = Join-Path $scratch ([guid]::NewGuid().ToString('N'))
        [void][IO.Directory]::CreateDirectory($directory)
        $changed = Join-Path $directory $name
        Copy-Item -LiteralPath $archive -Destination $changed
        $zip = [IO.Compression.ZipFile]::Open($changed, [IO.Compression.ZipArchiveMode]::Update)
        try {
            $entry = $zip.GetEntry($case.Name.TrimEnd('/'))
            if ($null -ne $entry) { $entry.Delete() }
            $writer = [IO.StreamWriter]::new($zip.CreateEntry($case.Name).Open(), [Text.UTF8Encoding]::new($false))
            try { $writer.Write($case.Content) } finally { $writer.Dispose() }
        } finally { $zip.Dispose() }
        Assert-PortableRejection { & $verify -ArchivePath $changed @arguments } $case.Error 'Portable identity or model omission regression was admitted'
    }
    Write-Host 'Portable archive layout, integrity, signature and unsafe-path fixtures passed'
}
