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
$verifyScript = Join-Path $PSScriptRoot "release_verify_portable_archive.ps1"
$signatureVerifyScript = Join-Path $PSScriptRoot "release_verify_portable_signatures.ps1"
$productionSynchronizationSource = Join-Path `
    $repositoryRoot `
    "rust\src\application\library_synchronization\production.rs"
$journalTransportSource = Join-Path `
    $repositoryRoot `
    "rust\src\journal_broker\windows\transport.rs"
$productionSynchronizationText = [IO.File]::ReadAllText(
    $productionSynchronizationSource,
    [Text.Encoding]::UTF8
)
$journalTransportText = [IO.File]::ReadAllText(
    $journalTransportSource,
    [Text.Encoding]::UTF8
)
if ($productionSynchronizationText -notmatch 'portable_production_never_invokes_the_journal_factory' -or
    $productionSynchronizationText -notmatch 'PersistentChangeJournalLiveOnlyReason::PortableDistribution') {
    throw "Portable production must remain LiveOnly before journal factory activation"
}
$portableGuardIndex = $journalTransportText.IndexOf(
    "current_process_has_installed_client_identity",
    [StringComparison]::Ordinal
)
$serviceStartIndex = $journalTransportText.IndexOf(
    "let started_service = demand_start_service()?;",
    [StringComparison]::Ordinal
)
if ($portableGuardIndex -lt 0 -or
    $serviceStartIndex -lt 0 -or
    $portableGuardIndex -ge $serviceStartIndex) {
    throw "Portable broker rejection must precede every SCM and pipe activation side effect"
}
$tag = "v1.2.3"
$archiveName = "Cedarflake-Ame-$tag-windows-x64-portable.zip"
$utf8 = [System.Text.UTF8Encoding]::new($false)
$signedFixtureSource = Join-Path $env:SystemRoot "System32\notepad.exe"
$signedFixtureSignature = Get-AuthenticodeSignature -LiteralPath $signedFixtureSource
if (
    $signedFixtureSignature.Status -ne [System.Management.Automation.SignatureStatus]::Valid -or
    $null -eq $signedFixtureSignature.SignerCertificate
) {
    throw "The Windows signed fixture is unavailable"
}
$signedFixturePublisher = $signedFixtureSignature.SignerCertificate.Subject

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
        "rust_lib_cedarflake_ame.dll",
        "flutter_windows.dll",
        "data\app.so",
        "data\icudtl.dat",
        "data\flutter_assets\AssetManifest.bin"
    )) {
        $fixturePath = Join-Path $releaseRoot $relativePath
        [System.IO.File]::WriteAllText($fixturePath, $relativePath, $utf8)
    }
    Copy-Item `
        -LiteralPath $signedFixtureSource `
        -Destination (Join-Path $releaseRoot "cedarflake_ame.exe")
    Copy-Item `
        -LiteralPath $signedFixtureSource `
        -Destination (Join-Path $releaseRoot "cedarflake_ame_journal_broker.exe")

    Add-Type -AssemblyName System.IO.Compression.FileSystem
    New-Item -ItemType Directory -Path $outputDirectory -Force | Out-Null
    $portableFixtureRoot = Join-Path $scratchRoot "portable-fixture\Cedarflake-Ame"
    New-Item -ItemType Directory -Path $portableFixtureRoot -Force | Out-Null
    foreach ($entry in @(Get-ChildItem -LiteralPath $releaseRoot -Force)) {
        Copy-Item `
            -LiteralPath $entry.FullName `
            -Destination $portableFixtureRoot `
            -Recurse `
            -Force
    }
    $expectedArchivePath = Join-Path $outputDirectory $archiveName
    [System.IO.Compression.ZipFile]::CreateFromDirectory(
        ([System.IO.Path]::GetDirectoryName($portableFixtureRoot)),
        $expectedArchivePath
    )
    & $verifyScript `
        -ArchivePath $expectedArchivePath `
        -Tag $tag `
        -PubspecPath $pubspecFixture `
        -CargoManifestPath $cargoFixture
    $archivePath = $expectedArchivePath
    if (-not (Test-Path -LiteralPath $archivePath -PathType Leaf)) {
        throw "Portable verifier did not retain the valid fixture archive"
    }
    $wrongPublisherRejected = $false
    try {
        & $signatureVerifyScript `
            -ArchivePath $expectedArchivePath `
            -Tag $tag `
            -ExpectedBrokerPublisher "CN=Cedarflake Ame Invalid Fixture Publisher" `
            -PubspecPath $pubspecFixture `
            -CargoManifestPath $cargoFixture
    } catch {
        $wrongPublisherRejected = $true
    }
    if (-not $wrongPublisherRejected) {
        throw "Portable signature verification accepted the wrong publisher"
    }
    if (@(Get-ChildItem `
        -LiteralPath $buildRoot `
        -Directory `
        -Filter "portable-signature-verify-$PID-*" `
        -ErrorAction SilentlyContinue).Count -ne 0) {
        throw "Portable signature verification retained extraction state after rejection"
    }

    $tamperedFixtureRoot = Join-Path $scratchRoot "tampered-fixture\Cedarflake-Ame"
    New-Item -ItemType Directory -Path $tamperedFixtureRoot -Force | Out-Null
    foreach ($entry in @(Get-ChildItem -LiteralPath $releaseRoot -Force)) {
        Copy-Item `
            -LiteralPath $entry.FullName `
            -Destination $tamperedFixtureRoot `
            -Recurse `
            -Force
    }
    $tamperedApplication = Join-Path $tamperedFixtureRoot "cedarflake_ame.exe"
    $tamperedBytes = [System.IO.File]::ReadAllBytes($tamperedApplication)
    $tamperIndex = [Math]::Min(4096, $tamperedBytes.Length - 1)
    $tamperedBytes[$tamperIndex] = $tamperedBytes[$tamperIndex] -bxor 0x01
    [System.IO.File]::WriteAllBytes($tamperedApplication, $tamperedBytes)
    $tamperedArchiveDirectory = Join-Path $scratchRoot "tampered-archive"
    New-Item -ItemType Directory -Path $tamperedArchiveDirectory | Out-Null
    $tamperedArchivePath = Join-Path $tamperedArchiveDirectory $archiveName
    [System.IO.Compression.ZipFile]::CreateFromDirectory(
        ([System.IO.Path]::GetDirectoryName($tamperedFixtureRoot)),
        $tamperedArchivePath
    )
    $tamperedSignatureRejected = $false
    try {
        & $signatureVerifyScript `
            -ArchivePath $tamperedArchivePath `
            -Tag $tag `
            -ExpectedBrokerPublisher $signedFixturePublisher `
            -PubspecPath $pubspecFixture `
            -CargoManifestPath $cargoFixture
    } catch {
        $tamperedSignatureRejected = $true
    }
    if (-not $tamperedSignatureRejected) {
        throw "Portable signature verification accepted a tampered signed application"
    }
    if (@(Get-ChildItem `
        -LiteralPath $buildRoot `
        -Directory `
        -Filter "portable-signature-verify-$PID-*" `
        -ErrorAction SilentlyContinue).Count -ne 0) {
        throw "Portable signature verification retained extraction state after tamper rejection"
    }

    $missingBrokerRoot = Join-Path $scratchRoot "missing-broker"
    New-Item -ItemType Directory `
        -Path (Join-Path $missingBrokerRoot "Cedarflake-Ame\data\flutter_assets") `
        -Force | Out-Null
    foreach ($relativePath in @(
        "cedarflake_ame.exe",
        "rust_lib_cedarflake_ame.dll",
        "flutter_windows.dll",
        "data\app.so",
        "data\icudtl.dat",
        "data\flutter_assets\AssetManifest.bin"
    )) {
        $fixturePath = Join-Path $missingBrokerRoot "Cedarflake-Ame\$relativePath"
        [System.IO.File]::WriteAllText($fixturePath, $relativePath, $utf8)
    }
    $missingBrokerArchiveDirectory = Join-Path $scratchRoot "missing-broker-archive"
    New-Item -ItemType Directory -Path $missingBrokerArchiveDirectory -Force | Out-Null
    $missingBrokerArchivePath = Join-Path $missingBrokerArchiveDirectory $archiveName
    [System.IO.Compression.ZipFile]::CreateFromDirectory(
        $missingBrokerRoot,
        $missingBrokerArchivePath
    )
    $missingBrokerRejected = $false
    try {
        & $verifyScript `
            -ArchivePath $missingBrokerArchivePath `
            -Tag $tag `
            -PubspecPath $pubspecFixture `
            -CargoManifestPath $cargoFixture
    } catch {
        $missingBrokerRejected = $true
    }
    if (-not $missingBrokerRejected) {
        throw "Portable archive verification accepted a missing journal broker"
    }

    $missingRuntimeRoot = Join-Path $scratchRoot "missing-runtime"
    New-Item -ItemType Directory `
        -Path (Join-Path $missingRuntimeRoot "Cedarflake-Ame\data\flutter_assets") `
        -Force | Out-Null
    foreach ($relativePath in @(
        "cedarflake_ame.exe",
        "cedarflake_ame_journal_broker.exe",
        "flutter_windows.dll",
        "data\app.so",
        "data\icudtl.dat",
        "data\flutter_assets\AssetManifest.bin"
    )) {
        $fixturePath = Join-Path $missingRuntimeRoot "Cedarflake-Ame\$relativePath"
        [System.IO.File]::WriteAllText($fixturePath, $relativePath, $utf8)
    }
    $invalidArchivePath = Join-Path $scratchRoot $archiveName
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
