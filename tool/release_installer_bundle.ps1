$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

function New-AmeInstallerBundle {
    param(
        [Parameter(Mandatory = $true)][string]$Wix,
        [Parameter(Mandatory = $true)][string]$BootstrapperExtension,
        [Parameter(Mandatory = $true)][string]$PackagePath,
        [Parameter(Mandatory = $true)][string]$Version,
        [Parameter(Mandatory = $true)][string]$OutputDirectory,
        [Parameter(Mandatory = $true)][string]$Repository
    )

    Add-Type -AssemblyName System.Drawing
    $iconPath = Join-Path $Repository "windows/runner/resources/app_icon.ico"
    $icon = [Drawing.Icon]::new($iconPath, 64, 64)
    $splash = $icon.ToBitmap()
    $splashPath = Join-Path $OutputDirectory "splash.bmp"
    try { $splash.Save($splashPath, [Drawing.Imaging.ImageFormat]::Bmp) } finally {
        $splash.Dispose()
        $icon.Dispose()
    }
    $setup = Join-Path $OutputDirectory "Cedarflake-Ame-v$Version-windows-x64-setup.exe"
    if (Test-Path -LiteralPath $setup) { throw "Preserve the existing setup executable" }
    Invoke-AmeChecked $Wix @("-acceptEula", "wix7", "build", "-arch", "x64", "-wx", "-ct", "1",
        "-ext", $BootstrapperExtension, "-d", "ProductVersion=$Version", "-d", "PackagePath=$PackagePath",
        "-d", "ApplicationIcon=$iconPath", "-d", "SplashPath=$splashPath",
        "-d", "WixLicense=$(Join-Path $Repository 'windows/installer/licenses/wix.txt')",
        "-intermediatefolder", (Join-Path $OutputDirectory "bundle-intermediate"), "-out", $setup,
        (Join-Path $Repository "windows/installer/bundle.wxs")) | Out-Host
    return $setup
}
