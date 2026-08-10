$ErrorActionPreference = "Stop"
. (Join-Path $PSScriptRoot "quality_common.ps1")

$repositoryRoot = Get-AmeRepositoryRoot
$buildRoot = [System.IO.Path]::GetFullPath((Join-Path $repositoryRoot "build"))
$scratchRoot = [System.IO.Path]::GetFullPath(
    (Join-Path $buildRoot "release-version-test-$PID")
)
$repositoryPrefix = "$buildRoot$([System.IO.Path]::DirectorySeparatorChar)"
if (-not $scratchRoot.StartsWith(
    $repositoryPrefix,
    [System.StringComparison]::OrdinalIgnoreCase
)) {
    throw "Release version test storage must remain inside the build directory"
}

$validationScript = Join-Path $PSScriptRoot "release_validate_version.ps1"
$pubspecFixture = Join-Path $scratchRoot "pubspec.yaml"
$cargoFixture = Join-Path $scratchRoot "Cargo.toml"
$utf8 = [System.Text.UTF8Encoding]::new($false)

try {
    New-Item -ItemType Directory -Path $scratchRoot -Force | Out-Null
    [System.IO.File]::WriteAllText(
        $pubspecFixture,
        "name: fixture`nversion: 1.2.3-rc.1+4`n",
        $utf8
    )
    [System.IO.File]::WriteAllText(
        $cargoFixture,
        "[package]`nname = `"fixture`"`nversion = `"1.2.3-rc.1`"`n",
        $utf8
    )

    & $validationScript `
        -Tag "v1.2.3-rc.1" `
        -PubspecPath $pubspecFixture `
        -CargoManifestPath $cargoFixture

    $mismatchRejected = $false
    try {
        & $validationScript `
            -Tag "v1.2.4" `
            -PubspecPath $pubspecFixture `
            -CargoManifestPath $cargoFixture
    } catch {
        $mismatchRejected = $true
    }
    if (-not $mismatchRejected) {
        throw "Release version validation accepted mismatched versions"
    }

    $invalidTagRejected = $false
    try {
        & $validationScript `
            -Tag "release-1.2.3-rc.1" `
            -PubspecPath $pubspecFixture `
            -CargoManifestPath $cargoFixture
    } catch {
        $invalidTagRejected = $true
    }
    if (-not $invalidTagRejected) {
        throw "Release version validation accepted an unsupported tag"
    }
} finally {
    if (Test-Path -LiteralPath $scratchRoot) {
        Remove-Item -LiteralPath $scratchRoot -Recurse -Force
    }
}
