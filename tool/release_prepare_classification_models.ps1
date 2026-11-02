[CmdletBinding()]
param(
    [Parameter(Mandatory)][string]$OutputDirectory,
    [string]$SourceDirectory
)
$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'release_classification_models.ps1')
$manifest = Get-AmeClassificationModelsManifest
New-AmeClassificationModelsDirectory $OutputDirectory $SourceDirectory $manifest
Write-Host "Verified installer model $($manifest.package)"
