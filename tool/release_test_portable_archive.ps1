$ErrorActionPreference = "Stop"
. (Join-Path $PSScriptRoot "quality_common.ps1")

$repositoryRoot = Get-AmeRepositoryRoot
$buildRoot = [System.IO.Path]::GetFullPath((Join-Path $repositoryRoot "build"))
$scratchRoot = [System.IO.Path]::GetFullPath(
    (Join-Path $buildRoot "release-portable-test-$PID")
)
$buildPrefix = "$buildRoot$([System.IO.Path]::DirectorySeparatorChar)"
if (-not $scratchRoot.StartsWith(
    $buildPrefix,
    [System.StringComparison]::OrdinalIgnoreCase
)) {
    throw "Portable release test storage must remain inside the build directory"
}

$releaseRoot = Join-Path $scratchRoot "fixture-release"
$outputDirectory = Join-Path $scratchRoot "artifacts"
$pubspecFixture = Join-Path $scratchRoot "pubspec.yaml"
$cargoFixture = Join-Path $scratchRoot "Cargo.toml"
$packageScript = Join-Path $PSScriptRoot "release_package_portable_windows.ps1"
$verifyScript = Join-Path $PSScriptRoot "release_verify_portable_archive.ps1"
$tag = "v1.2.3"
$archiveName = "Cedarflake-Ame-$tag-windows-x64-portable.zip"
$utf8 = [System.Text.UTF8Encoding]::new($false)

try {
    New-Item -ItemType Directory `
        -Path (Join-Path $releaseRoot "data\flutter_assets") `
        -Force | Out-Null
    [System.IO.File]::WriteAllText(
        $pubspecFixture,
        "name: fixture`nversion: 1.2.3+4`n",
        $utf8
    )
    [System.IO.File]::WriteAllText(
        $cargoFixture,
        "[package]`nname = `"fixture`"`nversion = `"1.2.3`"`n",
        $utf8
    )
    foreach ($relativePath in @(
        "cedarflake_ame.exe",
        "rust_lib_cedarflake_ame.dll",
        "flutter_windows.dll",
        "data\app.so",
        "data\icudtl.dat",
        "data\flutter_assets\AssetManifest.bin"
    )) {
        $fixturePath = Join-Path $releaseRoot $relativePath
        [System.IO.File]::WriteAllText($fixturePath, $relativePath, $utf8)
    }

    $archivePath = & $packageScript `
        -Tag $tag `
        -ReleaseRoot $releaseRoot `
        -OutputDirectory $outputDirectory `
        -PubspecPath $pubspecFixture `
        -CargoManifestPath $cargoFixture | Select-Object -Last 1
    $expectedArchivePath = Join-Path $outputDirectory $archiveName
    if ($archivePath -cne $expectedArchivePath) {
        throw "Portable packager returned an unexpected path: $archivePath"
    }
    if (-not (Test-Path -LiteralPath $archivePath -PathType Leaf)) {
        throw "Portable packager did not create the expected archive"
    }

    $missingRuntimeRoot = Join-Path $scratchRoot "missing-runtime"
    New-Item -ItemType Directory `
        -Path (Join-Path $missingRuntimeRoot "Cedarflake-Ame\data\flutter_assets") `
        -Force | Out-Null
    foreach ($relativePath in @(
        "cedarflake_ame.exe",
        "flutter_windows.dll",
        "data\app.so",
        "data\icudtl.dat",
        "data\flutter_assets\AssetManifest.bin"
    )) {
        $fixturePath = Join-Path $missingRuntimeRoot "Cedarflake-Ame\$relativePath"
        [System.IO.File]::WriteAllText($fixturePath, $relativePath, $utf8)
    }
    $invalidArchivePath = Join-Path $scratchRoot $archiveName
    Add-Type -AssemblyName System.IO.Compression.FileSystem
    [System.IO.Compression.ZipFile]::CreateFromDirectory(
        $missingRuntimeRoot,
        $invalidArchivePath
    )
    $missingRuntimeRejected = $false
    try {
        & $verifyScript `
            -ArchivePath $invalidArchivePath `
            -Tag $tag `
            -PubspecPath $pubspecFixture `
            -CargoManifestPath $cargoFixture
    } catch {
        $missingRuntimeRejected = $true
    }
    if (-not $missingRuntimeRejected) {
        throw "Portable archive verification accepted a missing runtime DLL"
    }

    $unsafeArchivePath = Join-Path $scratchRoot "unsafe\$archiveName"
    New-Item -ItemType Directory `
        -Path ([System.IO.Path]::GetDirectoryName($unsafeArchivePath)) `
        -Force | Out-Null
    $unsafeArchive = [System.IO.Compression.ZipFile]::Open(
        $unsafeArchivePath,
        [System.IO.Compression.ZipArchiveMode]::Create
    )
    try {
        $unsafeArchive.CreateEntry("../outside.txt") | Out-Null
    } finally {
        $unsafeArchive.Dispose()
    }
    $unsafePathRejected = $false
    try {
        & $verifyScript `
            -ArchivePath $unsafeArchivePath `
            -Tag $tag `
            -PubspecPath $pubspecFixture `
            -CargoManifestPath $cargoFixture
    } catch {
        $unsafePathRejected = $true
    }
    if (-not $unsafePathRejected) {
        throw "Portable archive verification accepted a path traversal entry"
    }
} finally {
    if (Test-Path -LiteralPath $scratchRoot) {
        Remove-Item -LiteralPath $scratchRoot -Recurse -Force
    }
}
