$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
Add-Type -AssemblyName System.IO.Compression.FileSystem

function Test-AmeMsixPayloadFile {
    param([Parameter(Mandatory)][string]$Name)
    if ($Name -cin @('cedarflake_ame_update.exe', 'cedarflake_ame_journal_broker.exe')) { return $false }
    if ($Name -match '(^|/)(AppxManifest\.xml|AppxSignature\.p7x|resources\.pri|Update\.exe|sq\.version|AppxBlockMap\.xml|\[Content_Types\]\.xml)$' -or
        $Name.StartsWith('Assets/', [StringComparison]::OrdinalIgnoreCase)) {
        throw 'Source payload already contains package-managed files'
    }
    return $true
}

function Assert-AmeMsixArchive {
    param([Parameter(Mandatory)][string]$Path, [Parameter(Mandatory)][object[]]$Files)
    $expected = [Collections.Generic.Dictionary[string, object]]::new([StringComparer]::Ordinal)
    foreach ($file in $Files) { $expected.Add([string]$file.relativePath, $file) }
    $seen = [Collections.Generic.HashSet[string]]::new([StringComparer]::OrdinalIgnoreCase)
    $archive = [IO.Compression.ZipFile]::OpenRead($Path)
    try {
        if ($archive.Entries.Count -gt ($expected.Count + 3)) { throw 'MSIX contains unexpected files' }
        foreach ($entry in $archive.Entries) {
            $name = $entry.FullName.Replace('\', '/')
            $segments = $name.Split('/')
            if (-not $seen.Add($name) -or $name.Contains(':') -or
                @($segments | Where-Object { $_ -in @('', '.', '..') -or $_.EndsWith('.') -or $_.EndsWith(' ') }).Count -gt 0) {
                throw 'MSIX contains duplicate or unsafe paths'
            }
            if ($expected.ContainsKey($name)) {
                $file = $expected[$name]
                if ($entry.Length -ne $file.length) { throw "MSIX payload length differs: $name" }
                $stream = $entry.Open()
                $hash = [Security.Cryptography.SHA256]::Create()
                try { $digest = [BitConverter]::ToString($hash.ComputeHash($stream)).Replace('-', '') }
                finally { $hash.Dispose(); $stream.Dispose() }
                if ($digest -ine $file.sha256) { throw "MSIX payload hash differs: $name" }
            } elseif ($name -cnotin @('AppxBlockMap.xml', '[Content_Types].xml', 'AppxMetadata/CodeIntegrity.cat')) {
                throw "Unexpected MSIX entry: $name"
            }
        }
        foreach ($name in $expected.Keys) {
            if (-not $seen.Contains($name)) { throw "MSIX is missing $name" }
        }
        foreach ($name in @('AppxBlockMap.xml', '[Content_Types].xml')) {
            if (-not $seen.Contains($name)) { throw "MSIX is missing package metadata: $name" }
        }
    } finally { $archive.Dispose() }
}
