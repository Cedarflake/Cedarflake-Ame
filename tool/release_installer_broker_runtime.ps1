$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
. (Join-Path $PSScriptRoot 'release_msvc_runtime.ps1')

function Add-AmeInstallerBrokerRuntime {
    param(
        [Parameter(Mandatory = $true)][Xml.XmlElement]$Fragment,
        [Parameter(Mandatory = $true)][string]$BundlePath,
        [ValidateSet('x64', 'arm64')][string]$Architecture = 'x64'
    )

    $directory = Add-AmeInstallerXmlElement $Fragment 'DirectoryRef' @{ Id = 'AmeBrokerFolder' }
    $group = Add-AmeInstallerXmlElement $Fragment 'ComponentGroup' @{ Id = 'AmeBrokerRuntimeFiles' }
    $brokerSid = 'S-1-5-80-2098581772-366539847-2170527201-1913264099-3006267874'
    foreach ($name in (Get-AmeMsvcRuntimePayloadNames -Architecture $Architecture)) {
        $id = 'AmeBrokerRuntime_' + $name.Replace('.', '_')
        $component = Add-AmeInstallerXmlElement $directory 'Component' @{
            Id = $id; Guid = Get-AmeInstallerGuid "broker-runtime/$name" $Architecture; Bitness = 'always64'
        }
        $file = Add-AmeInstallerXmlElement $component 'File' @{
            Id = $id + '_file'; Source = Join-Path $BundlePath $name; KeyPath = 'yes'
        }
        [void](Add-AmeInstallerXmlElement $file 'PermissionEx' @{
            Sddl = $script:AmeInstallerFileSddl + "(A;;0x1200A9;;;$brokerSid)"
        })
        [void](Add-AmeInstallerXmlElement $group 'ComponentRef' @{ Id = $id })
    }
}
