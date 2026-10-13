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
function Set-AmeVersionFixtures {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Version
    )

    [System.IO.File]::WriteAllText(
        $pubspecFixture,
        "name: fixture`nversion: $Version+4`n",
        $utf8
    )
    [System.IO.File]::WriteAllText(
        $cargoFixture,
        "[package]`nname = `"fixture`"`nversion = `"$Version`"`n",
        $utf8
    )
}

function Assert-AmeVersionValidationFailure {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Tag,
        [Parameter(Mandatory = $true)]
        [string]$ExpectedMessage
    )

    try {
        & $validationScript `
            -Tag $Tag `
            -PubspecPath $pubspecFixture `
            -CargoManifestPath $cargoFixture
    } catch {
        if ($_.Exception.Message -cne $ExpectedMessage) {
            throw "Release validation failed at the wrong boundary: $($_.Exception.Message)"
        }
        return
    }
    throw "Release validation did not reject the invalid version at its owning boundary"
}

try {
    New-Item -ItemType Directory -Path $scratchRoot -Force | Out-Null
    Set-AmeVersionFixtures -Version "1.2.3-rc.1"

    & $validationScript `
        -Tag "v1.2.3-rc.1" `
        -PubspecPath $pubspecFixture `
        -CargoManifestPath $cargoFixture

    foreach ($validVersion in @("1.2.3-0", "1.2.3-0A")) {
        Set-AmeVersionFixtures -Version $validVersion
        & $validationScript `
            -Tag "v$validVersion" `
            -PubspecPath $pubspecFixture `
            -CargoManifestPath $cargoFixture
    }

    Set-AmeVersionFixtures -Version "1.2.3-rc.1"

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

    foreach ($invalidTag in @("v1.2.3-01", "v1.2.3-rc.01")) {
        Set-AmeVersionFixtures -Version $invalidTag.Substring(1)
        Assert-AmeVersionValidationFailure `
            -Tag $invalidTag `
            -ExpectedMessage "Release tag must use v-prefixed semantic versioning: $invalidTag"
    }

    Set-AmeVersionFixtures -Version "1.2.3-rc.1"
    [System.IO.File]::WriteAllText(
        $pubspecFixture,
        "name: fixture`nversion: 1.2.3-01+4`n",
        $utf8
    )
    Assert-AmeVersionValidationFailure `
        -Tag "v1.2.3-rc.1" `
        -ExpectedMessage "pubspec.yaml does not contain one supported application version"

    Set-AmeVersionFixtures -Version "1.2.3-rc.1"
    [System.IO.File]::WriteAllText(
        $cargoFixture,
        "[package]`nname = `"fixture`"`nversion = `"1.2.3-rc.01`"`n",
        $utf8
    )
    Assert-AmeVersionValidationFailure `
        -Tag "v1.2.3-rc.1" `
        -ExpectedMessage "Cargo.toml package table does not contain one supported version"
} finally {
    if (Test-Path -LiteralPath $scratchRoot) {
        Remove-Item -LiteralPath $scratchRoot -Recurse -Force
    }
}
