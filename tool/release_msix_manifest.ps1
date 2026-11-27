$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
. (Join-Path $PSScriptRoot 'release_installer_version.ps1')

function Get-AmeMsixVersion {
    param([Parameter(Mandatory)][string]$Tag, [Parameter(Mandatory)][string]$ApplicationVersion)
    $installer = [version](Get-AmeInstallerVersion -Tag $Tag -ApplicationVersion $ApplicationVersion)
    return '{0}.{1}.{2}.0' -f ($installer.Major + 1), $installer.Minor, $installer.Build
}

function Get-AmeMsixIdentity {
    Get-Content -LiteralPath (Join-Path $PSScriptRoot '../windows/msix/identity.json') -Raw -Encoding UTF8 | ConvertFrom-Json
}

function New-AmeMsixManifest {
    param(
        [Parameter(Mandatory)][string]$Path,
        [Parameter(Mandatory)][string]$Version,
        [ValidateSet('x64', 'arm64')][string]$Architecture = 'x64'
    )
    if ($Version -cnotmatch '^[1-9][0-9]*\.[0-9]+\.[1-9][0-9]*\.0$') { throw 'Invalid Store version' }
    $parsed = [version]$Version
    if ($parsed.Major -gt 65535 -or $parsed.Minor -gt 65535 -or $parsed.Build -gt 65535) { throw 'Store version exceeds its component limit' }
    $identity = Get-AmeMsixIdentity
    $values = @{}
    foreach ($key in @('name', 'publisher', 'publisherDisplayName', 'displayName', 'applicationId')) {
        $values[$key] = [Security.SecurityElement]::Escape([string]$identity.$key)
    }
    $xml = @"
<?xml version="1.0" encoding="utf-8"?>
<Package xmlns="http://schemas.microsoft.com/appx/manifest/foundation/windows10"
         xmlns:uap="http://schemas.microsoft.com/appx/manifest/uap/windows10"
         xmlns:uap10="http://schemas.microsoft.com/appx/manifest/uap/windows10/10"
         xmlns:rescap="http://schemas.microsoft.com/appx/manifest/foundation/windows10/restrictedcapabilities"
         IgnorableNamespaces="uap uap10 rescap">
  <Identity Name="$($values.name)" Publisher="$($values.publisher)" Version="$Version" ProcessorArchitecture="$Architecture" />
  <Properties>
    <DisplayName>$($values.displayName)</DisplayName>
    <PublisherDisplayName>$($values.publisherDisplayName)</PublisherDisplayName>
    <Logo>Assets\StoreLogo.png</Logo>
  </Properties>
  <Dependencies><TargetDeviceFamily Name="Windows.Desktop" MinVersion="10.0.22000.0" MaxVersionTested="10.0.26100.0" /></Dependencies>
  <Resources><Resource Language="en-US" /><Resource Language="zh-Hans" /><Resource Language="ja-JP" /></Resources>
  <Applications>
    <Application Id="$($values.applicationId)" Executable="cedarflake_ame.exe" uap10:RuntimeBehavior="packagedClassicApp" uap10:TrustLevel="mediumIL">
      <uap:VisualElements DisplayName="$($values.displayName)" Description="Cedarflake Ame" BackgroundColor="transparent"
          Square150x150Logo="Assets\Square150x150Logo.png" Square44x44Logo="Assets\Square44x44Logo.png" />
    </Application>
  </Applications>
  <Capabilities><rescap:Capability Name="runFullTrust" /></Capabilities>
</Package>
"@
    [IO.File]::WriteAllText($Path, $xml.Replace("`r`n", "`n"), [Text.UTF8Encoding]::new($false))
}

function Get-AmeMsixSdkTool {
    param([Parameter(Mandatory)][ValidateSet('makeappx.exe', 'makepri.exe', 'signtool.exe')][string]$Name)
    $kit = Get-ItemProperty -LiteralPath 'HKLM:\SOFTWARE\Microsoft\Windows Kits\Installed Roots' -Name KitsRoot10
    $path = Join-Path $kit.KitsRoot10 "bin/10.0.26100.0/x64/$Name"
    if (-not (Test-Path -LiteralPath $path -PathType Leaf)) { throw "Windows SDK 10.0.26100.0 tool is required: $Name" }
    return $path
}
