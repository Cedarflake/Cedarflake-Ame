$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
. (Join-Path $PSScriptRoot 'release_msix_manifest.ps1')
. (Join-Path $PSScriptRoot 'release_msix_payload.ps1')

function Read-AmeStorePackageReceipt {
    param([Parameter(Mandatory)][string]$Path)
    if ((Get-Item -LiteralPath $Path).Length -gt 1MB) { throw 'Store package receipt is too large' }
    $receipt = Get-Content -LiteralPath $Path -Raw -Encoding utf8 | ConvertFrom-Json
    $identity = Get-AmeMsixIdentity
    if ($receipt.format -cne 'cedarflake-ame-msix-v1' -or $receipt.dirtyWorktree -ne $false -or
        $receipt.sourceEvidenceFormat -cne 'cedarflake-ame-unsigned-v1' -or
        $receipt.sourceCommit -cnotmatch '^[0-9a-f]{40}$' -or $receipt.sha256 -cnotmatch '^[0-9a-f]{64}$' -or
        $receipt.architecture -cne 'x64' -or $receipt.identity.name -cne $identity.name -or
        $receipt.identity.publisher -cne $identity.publisher -or $receipt.identity.storeId -cne $identity.storeId) {
        throw 'Store publication requires an admitted, clean x64 release candidate'
    }
    if ($receipt.releaseTag -cnotmatch '^v[0-9]+\.[0-9]+\.[0-9]+(?:-[0-9A-Za-z.-]+)?$' -or
        $receipt.fileName -cne "Cedarflake-Ame-$($receipt.releaseTag)-windows-x64.msix" -or
        (Get-AmeMsixVersion $receipt.releaseTag $receipt.applicationVersion) -cne $receipt.packageVersion) {
        throw 'Store package version and release identity differ'
    }
    $names = [Collections.Generic.HashSet[string]]::new([StringComparer]::OrdinalIgnoreCase)
    foreach ($file in @($receipt.payload) + @($receipt.generatedPayload)) {
        $name = [string]$file.relativePath
        if (-not $names.Add($name) -or $name -match '(^/|\\|:|(^|/)\.\.?(/|$))' -or
            $file.length -lt 0 -or $file.sha256 -cnotmatch '^[0-9a-fA-F]{64}$') {
            throw 'Store receipt contains an invalid payload identity'
        }
    }
    if (-not $names.Contains('cedarflake_ame.exe') -or -not $names.Contains('data/app.so')) {
        throw 'Store receipt is missing the application build'
    }
    return $receipt
}

function Read-AmeStoreXmlEntry {
    param([IO.Compression.ZipArchive]$Archive, [string]$Name)
    $entries = @($Archive.Entries | Where-Object { $_.FullName.Replace('\', '/') -ceq $Name })
    if ($entries.Count -ne 1 -or $entries[0].Length -gt 1MB) { throw "Package metadata is missing or ambiguous: $Name" }
    $settings = [Xml.XmlReaderSettings]::new()
    $settings.DtdProcessing = [Xml.DtdProcessing]::Prohibit
    $settings.XmlResolver = $null
    $stream = $entries[0].Open()
    $reader = [Xml.XmlReader]::Create($stream, $settings)
    try {
        $xml = [Xml.XmlDocument]::new()
        $xml.XmlResolver = $null
        $xml.Load($reader)
        return ,$xml
    } finally { $reader.Dispose(); $stream.Dispose() }
}

function Assert-AmeStoreApplicationArchive {
    param([string]$Path, [object]$Receipt)
    $archive = [IO.Compression.ZipFile]::OpenRead($Path)
    try {
        $manifest = Read-AmeStoreXmlEntry $archive 'AppxManifest.xml'
        $identity = $manifest.Package.Identity
        if ($identity.Name -cne $Receipt.identity.name -or $identity.Publisher -cne $Receipt.identity.publisher -or
            $identity.Version -cne $Receipt.packageVersion -or $identity.ProcessorArchitecture -cne $Receipt.architecture) {
            throw 'Downloaded Store package does not match the requested identity, version and architecture'
        }
        $expected = [Collections.Generic.Dictionary[string, object]]::new([StringComparer]::Ordinal)
        foreach ($file in $Receipt.payload) { $expected.Add($file.relativePath, $file) }
        $seen = [Collections.Generic.HashSet[string]]::new([StringComparer]::OrdinalIgnoreCase)
        $total = 0L
        if ($archive.Entries.Count -gt 8192) { throw 'Store package exceeds its entry bound' }
        foreach ($entry in $archive.Entries) {
            $name = $entry.FullName.Replace('\', '/')
            $total += $entry.Length
            if (-not $seen.Add($name) -or $total -gt 3GB -or $name -match '(^/|:|(^|/)\.\.?(/|$))') {
                throw 'Store package contains duplicate, unsafe or oversized entries'
            }
            if ($expected.ContainsKey($name)) {
                $file = $expected[$name]
                if ($entry.Length -ne $file.length) { throw "Store payload length differs: $name" }
                $stream = $entry.Open()
                try { $hash = [Convert]::ToHexString([Security.Cryptography.SHA256]::HashData($stream)) }
                finally { $stream.Dispose() }
                if ($hash -ine $file.sha256) { throw "Store payload differs from the admitted application build: $name" }
            } elseif ([IO.Path]::GetExtension($name) -in @('.exe', '.dll', '.so')) {
                throw "Store package contains an unadmitted executable: $name"
            }
        }
        foreach ($name in $expected.Keys) {
            if (-not $seen.Contains($name)) { throw "Store package is missing admitted application content: $name" }
        }
    } finally { $archive.Dispose() }
}

function Assert-AmeMicrosoftStoreSignature {
    param([string]$Path)
    $signTool = Get-AmeMsixSdkTool 'signtool.exe'
    & $signTool verify /pa /all $Path | Out-Host
    if ($LASTEXITCODE -ne 0) { throw 'Windows rejected the package signature or integrity' }
    $archive = [IO.Compression.ZipFile]::OpenRead($Path)
    try {
        $signature = $archive.GetEntry('AppxSignature.p7x')
        if ($null -eq $signature -or $signature.Length -gt 1MB) { throw 'Store signature is absent or oversized' }
        $memory = [IO.MemoryStream]::new()
        $stream = $signature.Open()
        try { $stream.CopyTo($memory); $bytes = $memory.ToArray() }
        finally { $stream.Dispose(); $memory.Dispose() }
        if ($bytes.Length -lt 5 -or [Text.Encoding]::ASCII.GetString($bytes, 0, 4) -cne 'PKCX') {
            throw 'Package signature framing is invalid'
        }
        $cms = [Security.Cryptography.Pkcs.SignedCms]::new()
        $cms.Decode([byte[]]$bytes[4..($bytes.Length - 1)])
        if ($cms.SignerInfos.Count -ne 1 -or
            $cms.SignerInfos[0].Certificate.Issuer -notmatch '(^|,\s*)CN=Microsoft Windows Store PCA(?: [0-9]+)?(,|$)') {
            throw 'Package was not signed by the Microsoft Windows Store authority'
        }
    } finally { $archive.Dispose() }
}

function Assert-AmeStoreDownload {
    param([string]$Path, [object]$Receipt, [string]$ScratchDirectory)
    Assert-AmeMicrosoftStoreSignature $Path
    $extension = [IO.Path]::GetExtension($Path).ToLowerInvariant()
    if ($extension -in @('.msix', '.appx')) {
        Assert-AmeStoreApplicationArchive $Path $Receipt
        return
    }
    if ($extension -notin @('.msixbundle', '.appxbundle')) { throw 'Unsupported Store download format' }
    $archive = [IO.Compression.ZipFile]::OpenRead($Path)
    try {
        $bundle = Read-AmeStoreXmlEntry $archive 'AppxMetadata/AppxBundleManifest.xml'
        if ($bundle.Bundle.Identity.Name -cne $Receipt.identity.name -or
            $bundle.Bundle.Identity.Publisher -cne $Receipt.identity.publisher) { throw 'Store bundle identity differs' }
        $applications = @($bundle.Bundle.Packages.Package | Where-Object {
            $_.Type -ceq 'application' -and $_.Architecture -ceq $Receipt.architecture
        })
        if ($applications.Count -ne 1 -or $applications[0].Version -cne $Receipt.packageVersion -or
            $applications[0].FileName -cnotmatch '^[A-Za-z0-9_.-]+\.(msix|appx)$') {
            throw 'Store bundle has no unique application at the requested version'
        }
        $entries = @($archive.Entries | Where-Object FullName -CEQ $applications[0].FileName)
        if ($entries.Count -ne 1 -or $entries[0].Length -gt 2GB) { throw 'Store bundle application is invalid' }
        $inner = Join-Path $ScratchDirectory $applications[0].FileName
        [IO.Compression.ZipFileExtensions]::ExtractToFile($entries[0], $inner, $false)
        Assert-AmeStoreApplicationArchive $inner $Receipt
    } finally { $archive.Dispose() }
}
