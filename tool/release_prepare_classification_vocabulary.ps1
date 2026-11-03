[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)][string]$OutputDirectory,
    [string]$SourceDirectory
)

$ErrorActionPreference = "Stop"
. (Join-Path $PSScriptRoot "release_classification_vocabulary.ps1")

$output = [IO.Path]::GetFullPath($OutputDirectory).TrimEnd('\', '/')
$build = [IO.Path]::GetFullPath((Join-Path (Split-Path -Parent $PSScriptRoot) "build"))
if (-not $output.StartsWith("$build\", [StringComparison]::OrdinalIgnoreCase)) {
    throw "Classification vocabulary must remain below repository build storage"
}
if (Test-Path -LiteralPath $output) {
    Assert-AmeClassificationVocabularyDirectory $output
    return
}

$arguments = @(
    (Join-Path $PSScriptRoot "release_classification_vocabulary.py"), "--destination", $output
)
if ([string]::IsNullOrWhiteSpace($SourceDirectory)) {
    $SourceDirectory = Join-Path $build "classification-vocabulary-sources"
    $arguments += "--download"
}
$arguments += @("--source-directory", $SourceDirectory)
Invoke-AmeClassificationVocabularyPython $arguments
