[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [string]$Tag,
    [string]$PubspecPath,
    [string]$CargoManifestPath
)

$ErrorActionPreference = "Stop"
. (Join-Path $PSScriptRoot "quality_common.ps1")

$repositoryRoot = Get-AmeRepositoryRoot
if ([string]::IsNullOrWhiteSpace($PubspecPath)) {
    $PubspecPath = Join-Path $repositoryRoot "pubspec.yaml"
}
if ([string]::IsNullOrWhiteSpace($CargoManifestPath)) {
    $CargoManifestPath = Join-Path $repositoryRoot "rust\Cargo.toml"
}

$semanticVersionPattern = (
    "(?:0|[1-9][0-9]*)\." +
    "(?:0|[1-9][0-9]*)\." +
    "(?:0|[1-9][0-9]*)" +
    "(?:-[0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*)?"
)
$tagMatch = [regex]::Match($Tag, "^v(?<version>$semanticVersionPattern)$")
if (-not $tagMatch.Success) {
    throw "Release tag must use v-prefixed semantic versioning: $Tag"
}

if (-not (Test-Path -LiteralPath $PubspecPath -PathType Leaf)) {
    throw "Flutter manifest was not found: $PubspecPath"
}
if (-not (Test-Path -LiteralPath $CargoManifestPath -PathType Leaf)) {
    throw "Rust manifest was not found: $CargoManifestPath"
}

$pubspecContent = [System.IO.File]::ReadAllText($PubspecPath)
$pubspecVersionPattern = (
    '(?m)^version:\s*["'']?(?<version>{0})' +
    '(?:\+[0-9A-Za-z.-]+)?["'']?\s*$'
) -f $semanticVersionPattern
$pubspecMatch = [regex]::Match(
    $pubspecContent,
    $pubspecVersionPattern
)
if (-not $pubspecMatch.Success) {
    throw "pubspec.yaml does not contain one supported application version"
}

$cargoContent = [System.IO.File]::ReadAllText($CargoManifestPath)
$packageMatch = [regex]::Match(
    $cargoContent,
    "(?ms)^\[package\]\s*(?<body>.*?)(?=^\[|\z)"
)
if (-not $packageMatch.Success) {
    throw "Cargo.toml does not contain a package table"
}
$cargoVersionMatch = [regex]::Match(
    $packageMatch.Groups["body"].Value,
    ('(?m)^version\s*=\s*"(?<version>{0})"\s*$' -f $semanticVersionPattern)
)
if (-not $cargoVersionMatch.Success) {
    throw "Cargo.toml package table does not contain one supported version"
}

$tagVersion = $tagMatch.Groups["version"].Value
$pubspecVersion = $pubspecMatch.Groups["version"].Value
$cargoVersion = $cargoVersionMatch.Groups["version"].Value
if ($tagVersion -cne $pubspecVersion -or $tagVersion -cne $cargoVersion) {
    throw (
        "Release version mismatch: tag=$tagVersion, " +
        "pubspec=$pubspecVersion, cargo=$cargoVersion"
    )
}

Write-Host "Release version is consistent: $tagVersion"
