[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)][string]$PackagePath,
    [Parameter(Mandatory = $true)][string]$ReceiptPath,
    [Parameter(Mandatory = $true)][string]$WixSdkPackagePath,
    [Parameter(Mandatory = $true)][string]$WixSdkDirectory
)

$ErrorActionPreference = "Stop"
. (Join-Path $PSScriptRoot 'release_installer_toolchain.ps1')
. (Join-Path $PSScriptRoot 'release_installer_database.ps1')
. (Join-Path $PSScriptRoot 'release_installer_policy.ps1')
. (Join-Path $PSScriptRoot 'release_test_installer_policy.ps1')

$null = Get-AmeInstallerWixTool -PackagePath $WixSdkPackagePath -SdkDirectory $WixSdkDirectory
$receipt = Get-Content -LiteralPath $ReceiptPath -Raw -Encoding utf8 | ConvertFrom-Json
if ($receipt.format -cne 'cedarflake-ame-msi-v1') { throw 'Unrecognized installer receipt' }
$lease = [IO.FileStream]::new([IO.Path]::GetFullPath($PackagePath), [IO.FileMode]::Open,
    [IO.FileAccess]::Read, [IO.FileShare]::Read)
try {
    if ($lease.Length -le 0 -or $lease.Length -gt 2GB -or
        (Get-FileHash -LiteralPath $PackagePath -Algorithm SHA256).Hash -cne $receipt.packageSha256) {
        throw 'Installer package differs from its receipt or exceeds its bound'
    }
    $model = Read-AmeInstallerDatabase -PackagePath ([IO.Path]::GetFullPath($PackagePath)) `
        -DtfAssemblyPath (Join-Path $WixSdkDirectory 'tools/net472/WixToolset.Dtf.WindowsInstaller.dll')
    Assert-AmeInstallerPolicy $model $receipt
    Test-AmeInstallerPolicyRegressions $model $receipt
    Write-Host 'Installer database verified; system installation and lifecycle acceptance remain separate'
} finally { $lease.Dispose() }
