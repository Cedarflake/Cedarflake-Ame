$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
. (Join-Path $PSScriptRoot 'release_classification_runtime.ps1')

function Get-AmeMsvcRuntimePayloadNames {
    return @('msvcp140.dll', 'vcruntime140.dll', 'vcruntime140_1.dll')
}

function Get-AmeModelMsvcRuntimePayloadNames {
    return @(Get-AmeMsvcRuntimePayloadNames) + @('msvcp140_1.dll')
}

function Get-AmeMsvcRuntimePayloadPaths {
    Get-AmeModelMsvcRuntimePayloadNames
    foreach ($accelerator in @($false, $true)) {
        $manifest = Get-AmeClassificationRuntimeManifest -Accelerator:$accelerator
        foreach ($name in Get-AmeModelMsvcRuntimePayloadNames) {
            "$($manifest.bundleDirectory)/$name"
        }
    }
}

function Get-AmeMsvcRuntimeCompanionFiles {
    param([Parameter(Mandatory = $true)][string]$BundlePath)

    foreach ($name in Get-AmeModelMsvcRuntimePayloadNames) {
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
