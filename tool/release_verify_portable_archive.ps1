[CmdletBinding()]
param(
    [Parameter(Mandatory)][string]$ArchivePath,
    [Parameter(Mandatory)][string]$Tag,
    [ValidateSet('x64', 'arm64')][string]$Architecture = 'x64',
    [string]$PubspecPath,
    [string]$CargoManifestPath
)

$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'quality_common.ps1')
. (Join-Path $PSScriptRoot 'release_portable_payload.ps1')
& (Join-Path $PSScriptRoot 'release_validate_version.ps1') -Tag $Tag `
    -PubspecPath $PubspecPath -CargoManifestPath $CargoManifestPath
$path = [IO.Path]::GetFullPath($ArchivePath)
$expected = "Cedarflake-Ame-$Tag-windows-$Architecture-portable.zip"
if ([IO.Path]::GetFileName($path) -cne $expected) { throw "Portable archive name must be $expected" }
Add-Type -AssemblyName System.IO.Compression.FileSystem
$archive = [IO.Compression.ZipFile]::OpenRead($path)
try {
    $entries = Get-AmePortableArchiveEntries $archive
    Assert-AmePortableLayout -Entries $entries -Version $Tag.Substring(1) -Architecture $Architecture
    $buffer = [byte[]]::new(81920)
    foreach ($entry in $entries.Values) {
        if ($entry.FullName.EndsWith('/')) { continue }
        $stream = $entry.Open()
        try {
            $bytes = 0L
            while (($count = $stream.Read($buffer, 0, $buffer.Length)) -gt 0) { $bytes += $count }
            if ($bytes -ne $entry.Length) { throw 'Portable archive entry could not be read completely' }
        } finally { $stream.Dispose() }
    }
} finally { $archive.Dispose() }
Write-Host "Portable archive is valid: $path"
