$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
. (Join-Path $PSScriptRoot 'quality_unsigned_windows_payload.ps1')

# Velopack is MIT licensed. The release asset's length and SHA-256 are pinned from
# https://api.github.com/repos/velopack/velopack/releases/tags/1.2.161 (asset 598072248).
$script:AmeUpdateToolchain = [ordered]@{
    Version = '1.2.161'
    FileName = 'vpk.1.2.161.nupkg'
    Url = 'https://github.com/velopack/velopack/releases/download/1.2.161/vpk.1.2.161.nupkg'
    Length = 146884263L
    Sha256 = '2b56ce117f803fc70c103cb423bd040e395e40370f6ff6e10818e9ff9c26a323'
}

function Assert-AmeUpdateToolchainArchive {
    param([Parameter(Mandatory)][string]$PackagePath)

    Assert-AmeUnsignedBuildPath $PackagePath
    if ((Get-Item -LiteralPath $PackagePath).Length -ne $script:AmeUpdateToolchain.Length -or
        (Get-FileHash -LiteralPath $PackagePath -Algorithm SHA256).Hash -ine $script:AmeUpdateToolchain.Sha256) {
        throw 'Velopack toolchain package differs from its pinned release asset'
    }
}

function Get-AmeUpdateArchiveEntries {
    param([Parameter(Mandatory)][IO.Compression.ZipArchive]$Archive)

    if ($Archive.Entries.Count -eq 0 -or $Archive.Entries.Count -gt 10000) {
        throw 'Velopack toolchain archive entry count exceeds its bound'
    }
    $names = [Collections.Generic.HashSet[string]]::new([StringComparer]::OrdinalIgnoreCase)
    $length = 0L
    foreach ($entry in $Archive.Entries) {
        $name = $entry.FullName.Replace('\', '/')
        $segments = $name.TrimEnd('/').Split('/')
        if ([IO.Path]::IsPathRooted($name) -or
            @($segments | Where-Object { $_ -in @('', '.', '..') -or $_ -match '[<>:"|?*\x00-\x1F]' -or $_ -match '[ .]$' }).Count -gt 0 -or
            -not $names.Add($name) -or (($entry.ExternalAttributes -shr 16) -band 0xF000) -eq 0xA000) {
            throw 'Velopack toolchain archive contains an unsafe or duplicate path'
        }
        $length += $entry.Length
        if ($length -gt 1GB) { throw 'Velopack toolchain archive exceeds its expansion bound' }
        if (-not $name.EndsWith('/')) { $entry }
    }
}

function Assert-AmeUpdateToolchainDirectory {
    param([Parameter(Mandatory)][string]$PackagePath, [Parameter(Mandatory)][string]$Directory)

    Assert-AmeUpdateToolchainArchive $PackagePath
    Assert-AmeUnsignedBuildPath $Directory
    Add-Type -AssemblyName System.IO.Compression.FileSystem
    $archive = [IO.Compression.ZipFile]::OpenRead($PackagePath)
    try {
        $entries = @(Get-AmeUpdateArchiveEntries $archive)
        $actual = @(Get-ChildItem -LiteralPath $Directory -Recurse -Force)
        if (@($actual | Where-Object { ($_.Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0 }).Count -ne 0 -or
            @($actual | Where-Object { -not $_.PSIsContainer }).Count -ne $entries.Count) {
            throw 'Velopack toolchain extraction contains unlisted or redirected files'
        }
        foreach ($entry in $entries) {
            $path = Join-Path $Directory $entry.FullName
            if (-not (Test-Path -LiteralPath $path -PathType Leaf) -or
                (Get-Item -LiteralPath $path).Length -ne $entry.Length) {
                throw 'Velopack toolchain extraction is incomplete'
            }
            $stream = $entry.Open()
            $hash = [Security.Cryptography.SHA256]::Create()
            try { $expected = [Convert]::ToHexString($hash.ComputeHash($stream)) }
            finally { $hash.Dispose(); $stream.Dispose() }
            if ((Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash -cne $expected) {
                throw 'Extracted Velopack tool differs from its pinned package'
            }
        }
    } finally { $archive.Dispose() }
}

function Get-AmeUpdateTool {
    param([Parameter(Mandatory)][string]$PackagePath, [Parameter(Mandatory)][string]$Directory)

    Assert-AmeUpdateToolchainDirectory -PackagePath $PackagePath -Directory $Directory
    $tool = Join-Path $Directory 'tools/net8.0/any/vpk.dll'
    $dotnet = (Get-Command dotnet -CommandType Application -ErrorAction Stop | Select-Object -First 1).Source
    $version = [Diagnostics.FileVersionInfo]::GetVersionInfo($tool).ProductVersion
    if ($version -cne '1.2.161+92d6a1c') {
        throw 'Velopack packaging tool does not match version 1.2.161'
    }
    return [pscustomobject]@{ Dotnet = $dotnet; Assembly = $tool }
}

function Get-AmeUpdateLicense {
    $path = Join-Path (Split-Path -Parent $PSScriptRoot) 'assets/licenses/application_updates.txt'
    return [IO.File]::ReadAllText($path, [Text.Encoding]::UTF8)
}