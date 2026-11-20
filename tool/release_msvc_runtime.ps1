$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
. (Join-Path $PSScriptRoot 'release_classification_runtime.ps1')

function Get-AmeMsvcRuntimePayloadNames {
    param([ValidateSet('x64', 'arm64')][string]$Architecture = 'x64')
    if ($Architecture -eq 'arm64') { return @('msvcp140.dll', 'vcruntime140.dll') }
    return @('msvcp140.dll', 'vcruntime140.dll', 'vcruntime140_1.dll')
}

function Get-AmeModelMsvcRuntimePayloadNames {
    param([ValidateSet('x64', 'arm64')][string]$Architecture = 'x64')
    return @(Get-AmeMsvcRuntimePayloadNames -Architecture $Architecture) + @('msvcp140_1.dll')
}

function Get-AmeMsvcRuntimePayloadPaths {
    param([ValidateSet('x64', 'arm64')][string]$Architecture = 'x64')
    $names = Get-AmeModelMsvcRuntimePayloadNames -Architecture $Architecture
    $names
    foreach ($accelerator in @($false, $true)) {
        $manifest = Get-AmeClassificationRuntimeManifest -Accelerator:$accelerator -Architecture $Architecture
        foreach ($name in $names) {
            "$($manifest.bundleDirectory)/$name"
        }
    }
}

function Get-AmeMsvcRuntimeCompanionFiles {
    param(
        [Parameter(Mandatory = $true)][string]$BundlePath,
        [ValidateSet('x64', 'arm64')][string]$Architecture = 'x64'
    )

    foreach ($name in Get-AmeModelMsvcRuntimePayloadNames -Architecture $Architecture) {
        $path = Join-Path $BundlePath $name
        Assert-AmeClassificationRuntimeLocalPath $path
        $file = Get-Item -LiteralPath $path
        [pscustomobject]@{
            name = $name
            length = $file.Length
            sha256 = (Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash
        }
    }
}
