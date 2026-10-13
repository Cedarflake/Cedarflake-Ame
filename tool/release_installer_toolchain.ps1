$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

function Assert-AmeInstallerWixPackage {
    param(
        [Parameter(Mandatory = $true)][string]$PackagePath,
        [Parameter(Mandatory = $true)][string]$Directory,
        [Parameter(Mandatory = $true)][string]$ExpectedHash
    )

    if ((Get-FileHash -LiteralPath $PackagePath -Algorithm SHA256).Hash -cne $ExpectedHash) {
        throw "WiX package does not match the pinned 7.0.0 artifact"
    }
    Add-Type -AssemblyName System.IO.Compression.FileSystem
    $directory = [IO.Path]::GetFullPath($Directory).TrimEnd('\', '/')
    $archive = [IO.Compression.ZipFile]::OpenRead($PackagePath)
    try {
        if ($archive.Entries.Count -gt 2000) { throw "WiX package entry limit exceeded" }
        $bytes = 0L
        foreach ($entry in $archive.Entries) {
            $bytes += $entry.Length
            if ($bytes -gt 300MB -or [IO.Path]::IsPathRooted($entry.FullName) -or
                $entry.FullName -match '(^|[\\/])\.\.([\\/]|$)|:') {
                throw "WiX package has an invalid path or expansion size"
            }
            if ($entry.FullName.EndsWith('/')) { continue }
            $path = [IO.Path]::GetFullPath((Join-Path $directory $entry.FullName))
            if (-not $path.StartsWith($directory + '\', [StringComparison]::OrdinalIgnoreCase) -or
                -not (Test-Path -LiteralPath $path -PathType Leaf) -or
                ((Get-Item -LiteralPath $path -Force).Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0) {
                throw "WiX SDK extraction is incomplete or redirected"
            }
            $inputStream = $entry.Open()
            $sha256 = [Security.Cryptography.SHA256]::Create()
            try {
                $digest = -join @($sha256.ComputeHash($inputStream) | ForEach-Object { $_.ToString("X2") })
            } finally {
                $sha256.Dispose()
                $inputStream.Dispose()
            }
            if ((Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash -cne $digest) {
                throw "Extracted WiX SDK file differs from its pinned package"
            }
        }
    } finally { $archive.Dispose() }
}

function Get-AmeInstallerWixTool {
    param(
        [Parameter(Mandatory = $true)][string]$PackagePath,
        [Parameter(Mandatory = $true)][string]$SdkDirectory
    )

    Assert-AmeInstallerWixPackage -PackagePath $PackagePath -Directory $SdkDirectory `
        -ExpectedHash "AF8F72FB2550E9C2CF00B6EAF5E3ED811514AA2B3F344DFA342540AF39676979"
    $executable = Join-Path $SdkDirectory "tools/net472/x64/wix.exe"
    $version = @(& $executable --version)
    if ($LASTEXITCODE -ne 0 -or $version.Count -ne 1 -or $version[0] -cne "7.0.0+b8977d6") {
        throw "WiX compiler version does not match the pinned toolchain"
    }
    return $executable
}

function Get-AmeInstallerWixUi {
    param(
        [Parameter(Mandatory = $true)][string]$PackagePath,
        [Parameter(Mandatory = $true)][string]$Directory
    )

    Assert-AmeInstallerWixPackage -PackagePath $PackagePath -Directory $Directory `
        -ExpectedHash "1716C2CBED4613A7291B5775630CF4D0DCA597568BA1128CEC5FDA1903F05BD1"
    return Join-Path $Directory "wixext7/WixToolset.UI.wixext.dll"
}

function Get-AmeInstallerWixBootstrapper {
    param(
        [Parameter(Mandatory = $true)][string]$PackagePath,
        [Parameter(Mandatory = $true)][string]$Directory
    )

    Assert-AmeInstallerWixPackage -PackagePath $PackagePath -Directory $Directory `
        -ExpectedHash "0F30C29F9E53A6C6ACDEA2CE0D1F0833153883FA3D16ED8DFF75005EDD7FE876"
    return Join-Path $Directory "wixext7/WixToolset.BootstrapperApplications.wixext.dll"
}
