[CmdletBinding()]
param(
    [Parameter(Mandatory)][string]$OutputDirectory,
    [string]$SourceDirectory,
    [string]$MetadataDirectory
)
$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'release_classification_models.ps1')
$manifest = Get-AmeClassificationModelsManifest -MetadataDirectory $MetadataDirectory
$notices = if ($MetadataDirectory) { Join-Path $MetadataDirectory 'installation' } else { '' }
New-AmeClassificationModelsDirectory $OutputDirectory $SourceDirectory $manifest -NoticeDirectory $notices
Write-Host "Verified installer model $($manifest.package)"
