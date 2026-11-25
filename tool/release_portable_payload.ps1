$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
. (Join-Path $PSScriptRoot 'release_classification_runtime.ps1')
. (Join-Path $PSScriptRoot 'release_classification_models.ps1')
. (Join-Path $PSScriptRoot 'release_msvc_runtime.ps1')

function Get-AmePortableArchiveEntries {
    param([Parameter(Mandatory)][IO.Compression.ZipArchive]$Archive)

    if ($Archive.Entries.Count -eq 0 -or $Archive.Entries.Count -gt 8192) {
        throw 'Portable archive item count is outside its bound'
    }
    $entries = [Collections.Generic.Dictionary[string, IO.Compression.ZipArchiveEntry]]::new([StringComparer]::OrdinalIgnoreCase)
    $total = 0L
    foreach ($entry in $Archive.Entries) {
        $name = $entry.FullName.Replace('\', '/')
        $segments = $name.TrimEnd('/').Split('/')
        if ([IO.Path]::IsPathRooted($name) -or $segments.Count -gt 32 -or
            @($segments | Where-Object {
                $_ -in @('', '.', '..') -or $_ -match '[<>:"|?*\x00-\x1F]' -or $_ -match '[ .]$' -or
                $_ -match '^(?i:CON|PRN|AUX|NUL|COM[1-9]|LPT[1-9])(?:\.|$)'
            }).Count -gt 0 -or $entries.ContainsKey($name.TrimEnd('/')) -or
            (($entry.ExternalAttributes -shr 16) -band 0xF000) -eq 0xA000 -or
            ($entry.ExternalAttributes -band 0x400) -ne 0) {
            throw "Portable archive contains an unsafe or duplicate path: $name"
        }
        $entries.Add($name.TrimEnd('/'), $entry)
        $total += $entry.Length
        if ($total -gt 1GB) { throw 'Portable archive exceeds the 1 GiB uncompressed safety limit' }
    }
    foreach ($name in $entries.Keys) {
        $parent = $name
        while ($parent.Contains('/')) {
            $parent = $parent.Substring(0, $parent.LastIndexOf('/'))
            if ($entries.ContainsKey($parent) -and -not $entries[$parent].FullName.EndsWith('/')) {
                throw 'Portable archive file masks a parent directory'
            }
        }
    }
    return ,$entries
}

function Assert-AmePortableVersionEntry {
    param([Parameter(Mandatory)][IO.Compression.ZipArchiveEntry]$Entry,
        [Parameter(Mandatory)][string]$Version, [ValidateSet('x64', 'arm64')][string]$Architecture)

    if ($Entry.Length -eq 0 -or $Entry.Length -gt 64KB) { throw 'Portable version metadata exceeds its bound' }
    $settings = [Xml.XmlReaderSettings]::new()
    $settings.DtdProcessing = [Xml.DtdProcessing]::Prohibit
    $settings.MaxCharactersInDocument = 64KB
    $stream = $Entry.Open()
    $reader = [Xml.XmlReader]::Create($stream, $settings)
    try {
        $document = [Xml.XmlDocument]::new()
        $document.XmlResolver = $null
        $document.Load($reader)
        $metadata = $document.DocumentElement.metadata
        if ($metadata.id -cne 'CedarflakeAme' -or $metadata.version -cne $Version -or
            $metadata.mainExe -cne 'cedarflake_ame.exe' -or $metadata.os -cne 'win' -or
            $metadata.machineArchitecture -cne $Architecture -or $metadata.rid -cne "win-$Architecture") {
            throw 'Portable version metadata differs from its release identity'
        }
    } finally { $reader.Dispose(); $stream.Dispose() }
}

function Assert-AmePortableLayout {
    param([Parameter(Mandatory)][Collections.Generic.Dictionary[string, IO.Compression.ZipArchiveEntry]]$Entries,
        [Parameter(Mandatory)][string]$Version, [ValidateSet('x64', 'arm64')][string]$Architecture = 'x64',
        [string]$Prefix = 'Cedarflake-Ame/')

    $required = @('.portable', 'Update.exe', 'Cedarflake Ame.exe', 'current/sq.version',
        'current/velopack-LICENSE.txt', 'current/cedarflake_ame.exe',
        'current/cedarflake_ame_update.exe', 'current/cedarflake_ame_journal_broker.exe',
        'current/cedarflake_ame_classification_worker.exe', 'current/rust_lib_cedarflake_ame.dll',
        'current/flutter_windows.dll', 'current/data/app.so', 'current/data/icudtl.dat') +
        @((@(Get-AmeClassificationRuntimePayloadNames -Architecture $Architecture) +
            @(Get-AmeMsvcRuntimePayloadPaths -Architecture $Architecture)) | ForEach-Object { "current/$_" })
    foreach ($name in $required) {
        if (-not $Entries.ContainsKey($Prefix + $name) -or $Entries[$Prefix + $name].FullName.EndsWith('/')) {
            throw "Portable archive is missing required entry: $Prefix$name"
        }
    }
    if ($Entries[$Prefix + '.portable'].Length -ne 0) { throw 'Portable marker must be empty' }
    $modelPrefix = $Prefix + 'current/' + (Get-AmeClassificationModelsManifest).bundleDirectory + '/'
    foreach ($name in $Entries.Keys) {
        if ($name -ceq $Prefix.TrimEnd('/')) { continue }
        if (-not $name.StartsWith($Prefix, [StringComparison]::Ordinal) -or
            $name.StartsWith($modelPrefix, [StringComparison]::OrdinalIgnoreCase) -or
            $name -match '(?i)(?:^|/)\.msi-installed$') {
            throw "Portable archive contains an unexpected root, model or installed-layout entry: $name"
        }
        $relative = $name.Substring($Prefix.Length)
        if ($relative.Split('/')[0] -cnotin @('current', '.portable', 'Update.exe', 'Cedarflake Ame.exe')) {
            throw "Portable archive contains an unexpected root entry: $name"
        }
    }
    if (@($Entries.Keys | Where-Object { $_.StartsWith($Prefix + 'current/data/flutter_assets/') }).Count -eq 0) {
        throw 'Portable archive does not contain Flutter assets'
    }
    Assert-AmePortableVersionEntry -Entry $Entries[$Prefix + 'current/sq.version'] -Version $Version -Architecture $Architecture
}

function Get-AmePortableEntryHash {
    param([Parameter(Mandatory)][IO.Compression.ZipArchiveEntry]$Entry)
    $stream = $Entry.Open()
    $hash = [Security.Cryptography.SHA256]::Create()
    try { return [Convert]::ToHexString($hash.ComputeHash($stream)).ToLowerInvariant() }
    finally { $hash.Dispose(); $stream.Dispose() }
}

function Assert-AmePortablePayloadEntries {
    param([Parameter(Mandatory)][Collections.Generic.Dictionary[string, IO.Compression.ZipArchiveEntry]]$Entries,
        [Parameter(Mandatory)][object[]]$Files, [Parameter(Mandatory)][string]$Prefix,
        [string[]]$GeneratedNames = @())

    $expected = [Collections.Generic.HashSet[string]]::new([StringComparer]::OrdinalIgnoreCase)
    foreach ($file in $Files) {
        $name = $Prefix + $file.relativePath
        [void]$expected.Add($name)
        if (-not $Entries.ContainsKey($name) -or $Entries[$name].Length -ne $file.length -or
            (Get-AmePortableEntryHash $Entries[$name]) -ine $file.sha256) {
            throw "Portable package changed verified candidate payload: $($file.relativePath)"
        }
    }
    foreach ($name in $GeneratedNames) { [void]$expected.Add($Prefix + $name) }
    foreach ($name in $Entries.Keys) {
        if ($name.StartsWith($Prefix, [StringComparison]::Ordinal) -and
            -not $Entries[$name].FullName.EndsWith('/') -and -not $expected.Contains($name)) {
            throw "Portable package contains an unlisted payload file: $name"
        }
    }
}

function Add-AmePortableEnvelope {
    param([Parameter(Mandatory)][string]$InputPath, [Parameter(Mandatory)][string]$OutputPath)
    $source = [IO.Compression.ZipFile]::OpenRead($InputPath)
    try {
        $entries = Get-AmePortableArchiveEntries $source
        $target = [IO.Compression.ZipFile]::Open($OutputPath, [IO.Compression.ZipArchiveMode]::Create)
        try {
            foreach ($entry in $entries.Values) {
                $output = $target.CreateEntry('Cedarflake-Ame/' + $entry.FullName,
                    [IO.Compression.CompressionLevel]::Optimal)
                $inputStream = $entry.Open()
                $outputStream = $output.Open()
                try { $inputStream.CopyTo($outputStream) }
                finally { $outputStream.Dispose(); $inputStream.Dispose() }
            }
        } finally { $target.Dispose() }
    } finally { $source.Dispose() }
}
