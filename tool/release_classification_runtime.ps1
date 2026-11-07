$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

function Get-AmeClassificationRuntimeManifest {
    param([switch]$Accelerator)
    $name = if ($Accelerator) { "accelerator_distribution.json" } else { "distribution.json" }
    $path = Join-Path (Split-Path -Parent $PSScriptRoot) "rust/src/adapters/classification_runtime/$name"
    return [IO.File]::ReadAllText($path, [Text.Encoding]::UTF8) | ConvertFrom-Json
}

function Get-AmeClassificationRuntimePayloadNames {
    foreach ($accelerator in @($false, $true)) {
        $manifest = Get-AmeClassificationRuntimeManifest -Accelerator:$accelerator
        foreach ($file in $manifest.files) {
            "$($manifest.bundleDirectory)/$($file.name)"
        }
    }
}

function Assert-AmeClassificationRuntimeLocalPath {
    param([Parameter(Mandatory = $true)][string]$Path)

    $current = [IO.Path]::GetFullPath($Path)
    while ($current) {
        if (Test-Path -LiteralPath $current) {
            $attributes = [IO.File]::GetAttributes($current)
            $unavailable = [IO.FileAttributes]::ReparsePoint -bor [IO.FileAttributes]::Offline -bor 0x40000 -bor 0x400000
            if (([long]$attributes -band [long]$unavailable) -ne 0) {
                throw "Classification runtime paths cannot traverse reparse points or cloud placeholders"
            }
        }
        $current = [IO.Path]::GetDirectoryName($current)
    }
}

function Assert-AmeClassificationRuntimeArtifact {
    param(
        [Parameter(Mandatory = $true)][string]$Path,
        [Parameter(Mandatory = $true)][object]$Expected
    )

    Assert-AmeClassificationRuntimeLocalPath $Path
    $file = Get-Item -LiteralPath $Path -Force
    if ($file.PSIsContainer -or $file.Length -ne $Expected.length -or
        (Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash -ine $Expected.sha256) {
        throw "Classification runtime artifact differs from its pinned length or digest: $($file.Name)"
    }
}

function Assert-AmeClassificationRuntimeDirectory {
    param(
        [Parameter(Mandatory = $true)][string]$Path,
        [Parameter(Mandatory = $true)][object]$Manifest
    )

    Assert-AmeClassificationRuntimeLocalPath $Path
    $entries = @(Get-ChildItem -LiteralPath $Path -Force)
    if ($entries.Count -ne $Manifest.files.Count) {
        throw "Classification runtime directory must contain exactly the admitted payload"
    }
    foreach ($file in $Manifest.files) {
        Assert-AmeClassificationRuntimeArtifact (Join-Path $Path $file.name) $file
    }
}

function Expand-AmeClassificationRuntimeArchive {
    param(
        [Parameter(Mandatory = $true)][string]$ArchivePath,
        [Parameter(Mandatory = $true)][string]$Destination,
        [Parameter(Mandatory = $true)][object]$Manifest
    )

    Assert-AmeClassificationRuntimeArtifact $ArchivePath $Manifest.archive
    Assert-AmeClassificationRuntimeLocalPath $Destination
    if (Test-Path -LiteralPath $Destination) {
        throw "Runtime extraction requires a new directory"
    }
    Add-Type -AssemblyName System.IO.Compression
    Add-Type -AssemblyName System.IO.Compression.FileSystem
    $archive = [IO.Compression.ZipFile]::OpenRead($ArchivePath)
    try {
        [void][IO.Directory]::CreateDirectory($Destination)
        foreach ($file in $Manifest.files) {
            $entryName = $Manifest.archive.prefix + $file.entry
            $entries = @($archive.Entries | Where-Object { $_.FullName -ceq $entryName })
            if ($entries.Count -ne 1 -or $entries[0].Length -ne $file.length) {
                throw "Runtime archive has a missing, duplicate or wrong-sized entry: $entryName"
            }
            $path = Join-Path $Destination $file.name
            $inputStream = $null
            $outputStream = $null
            try {
                $inputStream = $entries[0].Open()
                $outputStream = [IO.File]::Open($path, [IO.FileMode]::CreateNew, [IO.FileAccess]::Write)
                $buffer = [byte[]]::new(65536)
                $total = 0L
                while (($read = $inputStream.Read($buffer, 0, $buffer.Length)) -gt 0) {
                    $total += $read
                    if ($total -gt $file.length) { throw "Runtime archive entry exceeds its admitted size" }
                    $outputStream.Write($buffer, 0, $read)
                }
            } finally {
                if ($null -ne $outputStream) { $outputStream.Dispose() }
                if ($null -ne $inputStream) { $inputStream.Dispose() }
            }
            Assert-AmeClassificationRuntimeArtifact $path $file
        }
    } finally {
        $archive.Dispose()
    }
    Assert-AmeClassificationRuntimeDirectory $Destination $Manifest
}
