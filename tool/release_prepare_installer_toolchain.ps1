[CmdletBinding()]
param([Parameter(Mandatory)][string]$OutputDirectory)

$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'quality_common.ps1')
. (Join-Path $PSScriptRoot 'quality_unsigned_windows_payload.ps1')
. (Join-Path $PSScriptRoot 'release_installer_toolchain.ps1')
Add-Type -AssemblyName System.IO.Compression.FileSystem

$output = [IO.Path]::GetFullPath($OutputDirectory).TrimEnd('\', '/')
$build = Join-Path (Get-AmeRepositoryRoot) 'build'
if (-not $output.StartsWith($build + '\', [StringComparison]::OrdinalIgnoreCase)) {
    throw 'Installer toolchain storage must remain beneath repository build'
}
Assert-AmeUnsignedBuildPath $output
[void][IO.Directory]::CreateDirectory($output)
$paths = [ordered]@{}
foreach ($role in $script:AmeInstallerWixPackages.Keys) {
    $package = $script:AmeInstallerWixPackages[$role]
    $name = "$($package.Id).7.0.0.nupkg"
    $archive = Join-Path $output $name
    $directory = Join-Path $output $role
    Assert-AmeUnsignedBuildPath $archive
    Assert-AmeUnsignedBuildPath $directory
    if (-not (Test-Path -LiteralPath $archive)) {
        Invoke-WebRequest -Uri "https://api.nuget.org/v3-flatcontainer/$($package.Id)/7.0.0/$name" `
            -OutFile $archive -TimeoutSec 180
    }
    if ((Get-FileHash -LiteralPath $archive -Algorithm SHA256).Hash -cne $package.Sha256) {
        throw "Installer toolchain package differs from its pin: $role"
    }
    if (-not (Test-Path -LiteralPath $directory)) {
        [IO.Compression.ZipFile]::ExtractToDirectory($archive, $directory)
    }
    Assert-AmeInstallerWixPackage -PackagePath $archive -Directory $directory -ExpectedHash $package.Sha256
    $paths["Wix${role}PackagePath"] = $archive
    $directoryParameter = if ($role -ceq 'Sdk') { 'WixSdkDirectory' } else { "Wix${role}Directory" }
    $paths[$directoryParameter] = $directory
}
return $paths
