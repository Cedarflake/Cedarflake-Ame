[CmdletBinding()]
param([Parameter(Mandatory)][string]$OutputDirectory)

$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'quality_common.ps1')
. (Join-Path $PSScriptRoot 'release_update_toolchain.ps1')
Add-Type -AssemblyName System.IO.Compression.FileSystem

$output = [IO.Path]::GetFullPath($OutputDirectory).TrimEnd('\', '/')
$build = Join-Path (Get-AmeRepositoryRoot) 'build'
if (-not $output.StartsWith($build + '\', [StringComparison]::OrdinalIgnoreCase)) {
    throw 'Update toolchain storage must remain beneath repository build'
}
Assert-AmeUnsignedBuildPath $output
[void][IO.Directory]::CreateDirectory($output)
$archivePath = Join-Path $output $script:AmeUpdateToolchain.FileName
$directory = Join-Path $output 'vpk'
$leasePath = Join-Path $output '.prepare.lock'
Assert-AmeUnsignedBuildPath $leasePath
$lease = [IO.FileStream]::new($leasePath, [IO.FileMode]::OpenOrCreate,
    [IO.FileAccess]::ReadWrite, [IO.FileShare]::None, 4096, [IO.FileOptions]::DeleteOnClose)
try {
    Assert-AmeUnsignedBuildPath $archivePath
    if (-not (Test-Path -LiteralPath $archivePath)) {
        $partial = $archivePath + '.partial'
        Assert-AmeUnsignedBuildPath $partial
        Invoke-WebRequest -Uri $script:AmeUpdateToolchain.Url -OutFile $partial -Resume -TimeoutSec 300
        Assert-AmeUpdateToolchainArchive $partial
        Move-Item -LiteralPath $partial -Destination $archivePath
    }
    Assert-AmeUpdateToolchainArchive $archivePath
    Assert-AmeUnsignedBuildPath $directory
    if (-not (Test-Path -LiteralPath $directory)) {
        $archive = [IO.Compression.ZipFile]::OpenRead($archivePath)
        try { Get-AmeUpdateArchiveEntries $archive | Out-Null } finally { $archive.Dispose() }
        [IO.Compression.ZipFile]::ExtractToDirectory($archivePath, $directory)
    }
    Assert-AmeUpdateToolchainDirectory -PackagePath $archivePath -Directory $directory
} finally { $lease.Dispose() }
return @{ UpdateToolPackagePath = $archivePath; UpdateToolDirectory = $directory }
