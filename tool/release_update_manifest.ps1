$ErrorActionPreference = 'Stop'

function Get-AmeUpdateAsset {
    param([Parameter(Mandatory)][string]$Path)
    $file = Get-Item -LiteralPath $Path -Force
    if ($file.PSIsContainer -or ($file.Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0 -or
        $file.Length -le 0 -or $file.Length -gt 2GB -or
        $file.Name -cnotmatch '^[A-Za-z0-9][A-Za-z0-9._-]{0,179}$' -or $file.Name.Contains('..')) {
        throw 'Application update asset is not an admitted regular package'
    }
    [ordered]@{
        file_name = $file.Name
        sha256 = (Get-FileHash -LiteralPath $file.FullName -Algorithm SHA256).Hash.ToLowerInvariant()
        size = $file.Length
    }
}

function Write-AmeUpdateManifest {
    param(
        [Parameter(Mandatory)][string]$Tag,
        [Parameter(Mandatory)][string]$PubspecPath,
        [Parameter(Mandatory)][string]$CargoManifestPath,
        [Parameter(Mandatory)][ValidateSet('x64', 'arm64')][string]$Architecture,
        [Parameter(Mandatory)][string]$InstallerPath,
        [Parameter(Mandatory)][string]$PortablePackagePath,
        [Parameter(Mandatory)][string]$OutputDirectory
    )
    & (Join-Path $PSScriptRoot 'release_validate_version.ps1') -Tag $Tag `
        -PubspecPath $PubspecPath -CargoManifestPath $CargoManifestPath | Out-Null
    $version = $Tag.Substring(1)
    $match = [regex]::Match([IO.File]::ReadAllText($PubspecPath, [Text.Encoding]::UTF8),
        '(?m)^version:\s*["'']?' + [regex]::Escape($version) + '\+(?<build>[1-9][0-9]*)["'']?\s*$')
    if (-not $match.Success -or [long]$match.Groups['build'].Value -gt 65535) {
        throw 'Application updates require an MSI-compatible positive build number'
    }
    $parts = $version.Split('.')
    if ([long]$parts[0] -gt 255 -or [long]$parts[1] -gt 255) {
        throw 'Application update version exceeds Windows Installer limits'
    }
    $installed = Get-AmeUpdateAsset $InstallerPath
    $portable = Get-AmeUpdateAsset $PortablePackagePath
    if ($installed.file_name -cne "Cedarflake-Ame-$Tag-windows-$Architecture.msi" -or
        $portable.file_name -cne "CedarflakeAme-$version-full.nupkg") {
        throw 'Application update packages do not match their release identity'
    }
    $root = [IO.Path]::GetFullPath($OutputDirectory)
    $build = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '../build')) + [IO.Path]::DirectorySeparatorChar
    if (-not $root.StartsWith($build, [StringComparison]::OrdinalIgnoreCase)) {
        throw 'Update metadata must be written inside repository build storage'
    }
    [void][IO.Directory]::CreateDirectory($root)
    $path = Join-Path $root "ame-update-windows-$Architecture.json"
    if (Test-Path -LiteralPath $path) { throw 'Update metadata output already exists' }
    $manifest = [ordered]@{
        schema = 1; version = $version; build = [long]$match.Groups['build'].Value
        architecture = $Architecture; installed = $installed; portable = $portable
    }
    [IO.File]::WriteAllText($path, ($manifest | ConvertTo-Json -Depth 4) + "`n", [Text.UTF8Encoding]::new($false))
    return $path
}

function Assert-AmeUpdateManifest {
    param(
        [Parameter(Mandatory)][string]$Directory,
        [Parameter(Mandatory)][string]$Tag,
        [Parameter(Mandatory)][ValidateSet('x64', 'arm64')][string]$Architecture,
        [Parameter(Mandatory)][string]$ExpectedSha256
    )
    $path = Join-Path $Directory "ame-update-windows-$Architecture.json"
    $file = Get-Item -LiteralPath $path -Force
    if ($file.Length -le 0 -or $file.Length -gt 16384 -or $file.PSIsContainer -or
        ($file.Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0 -or
        $ExpectedSha256 -cnotmatch '^[0-9a-f]{64}$' -or
        (Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash.ToLowerInvariant() -cne $ExpectedSha256) {
        throw 'Application update manifest differs from its package provenance'
    }
    $manifest = Get-Content -LiteralPath $path -Raw -Encoding utf8 | ConvertFrom-Json
    $version = $Tag.Substring(1)
    if ($manifest.schema -ne 1 -or $manifest.version -cne $version -or
        $manifest.architecture -cne $Architecture -or $manifest.build -lt 1 -or $manifest.build -gt 65535 -or
        $manifest.installed.file_name -cne "Cedarflake-Ame-$Tag-windows-$Architecture.msi" -or
        $manifest.portable.file_name -cne "CedarflakeAme-$version-full.nupkg") {
        throw 'Application update manifest has a different release identity'
    }
    foreach ($asset in @($manifest.installed, $manifest.portable)) {
        $actual = Get-AmeUpdateAsset (Join-Path $Directory $asset.file_name)
        if ($actual.sha256 -cne $asset.sha256 -or $actual.size -ne $asset.size) {
            throw 'Application update package differs from its manifest'
        }
    }
    return $manifest
}
