$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

function Assert-AmeUnsignedBuildPath {
    param([Parameter(Mandatory = $true)][string]$Path)

    $current = [System.IO.Path]::GetFullPath($Path)
    while (-not [string]::IsNullOrEmpty($current)) {
        if (Test-Path -LiteralPath $current) {
            $item = Get-Item -LiteralPath $current -Force
            if (($item.Attributes -band [System.IO.FileAttributes]::ReparsePoint) -ne 0) {
                throw "Unsigned build storage must not traverse a reparse point"
            }
        }
        $current = [System.IO.Path]::GetDirectoryName($current)
    }
}

function Assert-AmeUnsignedX64Image {
    param([Parameter(Mandatory = $true)][string]$Path)

    $stream = [System.IO.File]::OpenRead($Path)
    try {
        $reader = [System.IO.BinaryReader]::new($stream)
        try {
            if ($stream.Length -lt 90 -or $reader.ReadUInt16() -ne 0x5A4D) {
                throw "Unsigned runtime payload is not a complete PE image"
            }
            $stream.Position = 0x3C
            $offset = $reader.ReadUInt32()
            if ($offset -lt 64 -or $offset -gt $stream.Length - 26) {
                throw "Unsigned runtime payload has an invalid PE header offset"
            }
            $stream.Position = $offset
            if ($reader.ReadUInt32() -ne 0x4550 -or $reader.ReadUInt16() -ne 0x8664) {
                throw "Unsigned runtime payload must be a Windows x64 PE image"
            }
            $stream.Position = $offset + 20
            $optionalHeaderSize = $reader.ReadUInt16()
            if ($optionalHeaderSize -lt 2 -or $offset + 24 + $optionalHeaderSize -gt $stream.Length) {
                throw "Unsigned runtime payload has an incomplete optional PE header"
            }
            $stream.Position = $offset + 24
            if ($reader.ReadUInt16() -ne 0x20B) {
                throw "Unsigned runtime payload must use the PE32+ image format"
            }
        } finally {
            $reader.Dispose()
        }
    } finally {
        $stream.Dispose()
    }
}

function Get-AmeUnsignedRustDependencyCount {
    param(
        [Parameter(Mandatory = $true)][string]$DependencyFile,
        [Parameter(Mandatory = $true)][string]$BuiltLibrary,
        [Parameter(Mandatory = $true)][string]$RequiredSource
    )

    Assert-AmeUnsignedBuildPath $DependencyFile
    $dependencyItem = Get-Item -LiteralPath $DependencyFile
    if ($dependencyItem.Length -eq 0 -or $dependencyItem.Length -gt 1MB) {
        throw "Unsigned Rust dependency evidence is empty or exceeds its bound"
    }
    $text = [System.IO.File]::ReadAllText($dependencyItem.FullName)
    $rule = (($text -replace '\\\r?\n', '') -split '\r?\n', 2)[0]
    $separator = $rule.IndexOf(": ", [System.StringComparison]::Ordinal)
    if ($separator -lt 0) {
        throw "Unsigned Rust dependency evidence is malformed"
    }
    $dependencies = [regex]::Matches($rule.Substring($separator + 2), '(?:\\ |[^\s])+')
    if ($dependencies.Count -eq 0 -or $dependencies.Count -gt 10000) {
        throw "Unsigned Rust dependency count is outside its bound"
    }
    $builtTime = (Get-Item -LiteralPath $BuiltLibrary).LastWriteTimeUtc
    $required = [System.IO.Path]::GetFullPath($RequiredSource)
    $foundRequired = $false
    foreach ($match in $dependencies) {
        $path = $match.Value.Replace('\ ', ' ')
        if (-not [System.IO.Path]::IsPathRooted($path) -or
            -not (Test-Path -LiteralPath $path -PathType Leaf)) {
            throw "Unsigned Rust dependency evidence names a missing or relative source"
        }
        Assert-AmeUnsignedBuildPath $path
        $dependency = Get-Item -LiteralPath $path
        if ($dependency.LastWriteTimeUtc -gt $builtTime) {
            throw "Unsigned Rust library is older than its current dependencies"
        }
        $foundRequired = $foundRequired -or $dependency.FullName.Equals(
            $required, [System.StringComparison]::OrdinalIgnoreCase
        )
    }
    if (-not $foundRequired) {
        throw "Unsigned Rust dependency evidence does not cover the application crate"
    }
    return $dependencies.Count
}

function Get-AmeUnsignedWindowsPayloadFacts {
    param(
        [Parameter(Mandatory = $true)][string]$BundlePath,
        [Parameter(Mandatory = $true)][string]$BuiltLibrary,
        [Parameter(Mandatory = $true)][string]$DependencyFile,
        [Parameter(Mandatory = $true)][string]$RequiredRustSource,
        [Parameter(Mandatory = $true)][string]$BuiltBroker
    )

    foreach ($path in @($BundlePath, $BuiltLibrary, $BuiltBroker)) {
        Assert-AmeUnsignedBuildPath $path
    }
    $root = (Get-Item -LiteralPath $BundlePath -Force).FullName.TrimEnd('\', '/')
    $pending = [System.Collections.Generic.Stack[string]]::new()
    $pending.Push($root)
    $files = [System.Collections.Generic.List[object]]::new()
    $names = [System.Collections.Generic.HashSet[string]]::new([StringComparer]::OrdinalIgnoreCase)
    $count = 0
    $totalBytes = 0L
    while ($pending.Count -gt 0) {
        $directory = [System.IO.DirectoryInfo]::new($pending.Pop())
        foreach ($item in $directory.EnumerateFileSystemInfos()) {
            $count++
            if ($count -gt 4096 -or
                ($item.Attributes -band [System.IO.FileAttributes]::ReparsePoint) -ne 0) {
                throw "Unsigned Windows payload exceeds its entry bound or contains a reparse point"
            }
            if (($item.Attributes -band [System.IO.FileAttributes]::Directory) -ne 0) {
                $pending.Push($item.FullName)
                continue
            }
            $totalBytes += $item.Length
            if ($totalBytes -gt 2GB) {
                throw "Unsigned Windows payload exceeds 2 GiB"
            }
            $relativePath = $item.FullName.Substring($root.Length + 1).Replace('\', '/')
            if (-not $names.Add($relativePath)) {
                throw "Unsigned Windows payload contains a duplicate path"
            }
            if ($item.Extension -in @('.exe', '.dll')) {
                Assert-AmeUnsignedX64Image $item.FullName
            }
            $files.Add([pscustomobject]@{
                relativePath = $relativePath
                length = $item.Length
                sha256 = (Get-FileHash -LiteralPath $item.FullName -Algorithm SHA256).Hash.ToLowerInvariant()
            })
        }
    }
    foreach ($name in @(
        'cedarflake_ame.exe', 'cedarflake_ame_journal_broker.exe',
        'rust_lib_cedarflake_ame.dll', 'flutter_windows.dll',
        'data/app.so', 'data/icudtl.dat'
    )) {
        $requiredFile = @($files | Where-Object { $_.relativePath -ceq $name })
        if ($requiredFile.Count -ne 1 -or $requiredFile[0].length -eq 0) {
            throw "Unsigned Windows payload is missing required runtime file: $name"
        }
    }
    if (@($files | Where-Object { $_.relativePath.StartsWith('data/flutter_assets/') }).Count -eq 0) {
        throw "Unsigned Windows payload does not contain Flutter assets"
    }
    foreach ($pair in @(
        @('rust_lib_cedarflake_ame.dll', $BuiltLibrary),
        @('cedarflake_ame_journal_broker.exe', $BuiltBroker)
    )) {
        $builtHash = (Get-FileHash -LiteralPath $pair[1] -Algorithm SHA256).Hash.ToLowerInvariant()
        $packaged = @($files | Where-Object { $_.relativePath -ceq $pair[0] })
        if ($packaged.Count -ne 1 -or $packaged[0].sha256 -cne $builtHash) {
            throw "Unsigned packaged binary differs from its current build: $($pair[0])"
        }
    }
    $dependencyCount = Get-AmeUnsignedRustDependencyCount `
        -DependencyFile $DependencyFile -BuiltLibrary $BuiltLibrary -RequiredSource $RequiredRustSource
    return [pscustomobject]@{
        configuration = 'Release'
        machine = 'windows-x64'
        fileCount = $files.Count
        totalBytes = $totalBytes
        rustDependencyCount = $dependencyCount
        files = @($files | Sort-Object relativePath)
    }
}
