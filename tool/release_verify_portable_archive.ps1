[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [string]$ArchivePath,
    [Parameter(Mandatory = $true)]
    [string]$Tag,
    [string]$PubspecPath,
    [string]$CargoManifestPath
)

$ErrorActionPreference = "Stop"
. (Join-Path $PSScriptRoot "quality_common.ps1")

& (Join-Path $PSScriptRoot "release_validate_version.ps1") `
    -Tag $Tag `
    -PubspecPath $PubspecPath `
    -CargoManifestPath $CargoManifestPath

$resolvedArchivePath = [System.IO.Path]::GetFullPath($ArchivePath)
if (-not (Test-Path -LiteralPath $resolvedArchivePath -PathType Leaf)) {
    throw "Portable archive was not found: $resolvedArchivePath"
}

$expectedArchiveName = "Cedarflake-Ame-$Tag-windows-x64-portable.zip"
if ([System.IO.Path]::GetFileName($resolvedArchivePath) -cne $expectedArchiveName) {
    throw "Portable archive name must be $expectedArchiveName"
}

Add-Type -AssemblyName System.IO.Compression.FileSystem
$archive = [System.IO.Compression.ZipFile]::OpenRead($resolvedArchivePath)
try {
    if ($archive.Entries.Count -eq 0) {
        throw "Portable archive is empty"
    }

    $entryNames = [System.Collections.Generic.HashSet[string]]::new(
        [System.StringComparer]::OrdinalIgnoreCase
    )
    [Int64]$totalUncompressedBytes = 0
    [Int64]$maximumUncompressedBytes = 1GB
    foreach ($entry in $archive.Entries) {
        $entryName = $entry.FullName.Replace("\", "/")
        if (
            [string]::IsNullOrWhiteSpace($entryName) -or
            $entryName.StartsWith("/", [System.StringComparison]::Ordinal) -or
            $entryName -match "^[A-Za-z]:" -or
            $entryName.Split("/") -contains ".."
        ) {
            throw "Portable archive contains an unsafe path: $($entry.FullName)"
        }
        if (-not $entryName.StartsWith(
            "Cedarflake-Ame/",
            [System.StringComparison]::Ordinal
        )) {
            throw "Portable archive entry is outside Cedarflake-Ame/: $entryName"
        }
        if (-not $entryNames.Add($entryName)) {
            throw "Portable archive contains a duplicate path: $entryName"
        }
        $totalUncompressedBytes += $entry.Length
        if ($totalUncompressedBytes -gt $maximumUncompressedBytes) {
            throw "Portable archive exceeds the 1 GiB uncompressed safety limit"
        }
    }

    $requiredEntries = @(
        "Cedarflake-Ame/cedarflake_ame.exe",
        "Cedarflake-Ame/rust_lib_cedarflake_ame.dll",
        "Cedarflake-Ame/flutter_windows.dll",
        "Cedarflake-Ame/data/app.so",
        "Cedarflake-Ame/data/icudtl.dat"
    )
    foreach ($requiredEntry in $requiredEntries) {
        if (-not $entryNames.Contains($requiredEntry)) {
            throw "Portable archive is missing required entry: $requiredEntry"
        }
    }

    $hasFlutterAsset = $entryNames.Where({
        $_.StartsWith(
            "Cedarflake-Ame/data/flutter_assets/",
            [System.StringComparison]::OrdinalIgnoreCase
        ) -and -not $_.EndsWith("/", [System.StringComparison]::Ordinal)
    }).Count -gt 0
    if (-not $hasFlutterAsset) {
        throw "Portable archive does not contain Flutter assets"
    }

    $buffer = [byte[]]::new(81920)
    foreach ($entry in $archive.Entries) {
        $normalizedEntryName = $entry.FullName.Replace("\", "/")
        if ($normalizedEntryName.EndsWith("/", [System.StringComparison]::Ordinal)) {
            continue
        }
        $entryStream = $entry.Open()
        try {
            [Int64]$bytesRead = 0
            while (($readCount = $entryStream.Read($buffer, 0, $buffer.Length)) -gt 0) {
                $bytesRead += $readCount
            }
            if ($bytesRead -ne $entry.Length) {
                throw "Portable archive entry could not be read completely: $($entry.FullName)"
            }
        } finally {
            $entryStream.Dispose()
        }
    }
} finally {
    $archive.Dispose()
}

Write-Host "Portable archive is valid: $resolvedArchivePath"
