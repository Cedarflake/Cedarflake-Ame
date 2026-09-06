[CmdletBinding()]
param(
    [ValidateSet(
        "all",
        "static",
        "flutter",
        "windows_scan",
        "windows_accessibility"
    )]
    [string]$Component = "all"
)

$ErrorActionPreference = "Stop"
. (Join-Path $PSScriptRoot "quality_common.ps1")

$repositoryRoot = Get-AmeRepositoryRoot
$toolchain = Get-AmeToolchain
$toolLock = Enter-AmeRepositoryToolLock
Push-Location $repositoryRoot

try {
    if ($Component -in @("all", "static")) {
        & (Join-Path $PSScriptRoot "quality_lint.ps1")
        Invoke-AmeChecked $toolchain.Cargo @(
            "test",
            "--locked",
            "--manifest-path",
            "rust\Cargo.toml",
            "--all-targets",
            "--all-features",
            "--",
            "--test-threads=1"
        )
    }
    if ($Component -in @("all", "flutter")) {
        & (Join-Path $PSScriptRoot "quality_test_flutter.ps1")
    }
    if ($Component -in @("all", "windows_scan")) {
        & (Join-Path $PSScriptRoot "integration_test_windows.ps1")
    }
    if ($Component -in @("all", "windows_accessibility")) {
        & (Join-Path $PSScriptRoot "integration_test_windows_accessibility.ps1")
    }
    if ($Component -in @("all", "static")) {
        $rustHashLine = Select-String -LiteralPath "rust\src\frb_generated.rs" -Pattern (
            "FLUTTER_RUST_BRIDGE_CODEGEN_CONTENT_HASH"
        )
        $dartHashLine = Select-String -LiteralPath "lib\src\rust\frb_generated.dart" -Pattern (
            "rustContentHash =>"
        )
        $rustHash = [regex]::Match($rustHashLine.Line, "=\s*(-?\d+)").Groups[1].Value
        $dartHash = [regex]::Match($dartHashLine.Line, "=>\s*(-?\d+)").Groups[1].Value
        if (-not $rustHash -or $rustHash -ne $dartHash) {
            throw "Generated Rust and Dart bridge hashes do not match"
        }

        $rustCatalogApiText = [System.IO.File]::ReadAllText(
            (Join-Path $repositoryRoot "rust\src\api\catalog.rs"),
            [System.Text.Encoding]::UTF8
        )
        $dartCatalogApiText = [System.IO.File]::ReadAllText(
            (Join-Path $repositoryRoot "lib\src\rust\api\catalog.dart"),
            [System.Text.Encoding]::UTF8
        )
        $rustStorageApiText = [System.IO.File]::ReadAllText(
            (Join-Path $repositoryRoot "rust\src\api\storage.rs"),
            [System.Text.Encoding]::UTF8
        )
        $dartStorageApiText = [System.IO.File]::ReadAllText(
            (Join-Path $repositoryRoot "lib\src\rust\api\storage.dart"),
            [System.Text.Encoding]::UTF8
        )
        $generatedDartText = [System.IO.File]::ReadAllText(
            (Join-Path $repositoryRoot "lib\src\rust\frb_generated.dart"),
            [System.Text.Encoding]::UTF8
        )
        $generatedRustText = [System.IO.File]::ReadAllText(
            (Join-Path $repositoryRoot "rust\src\frb_generated.rs"),
            [System.Text.Encoding]::UTF8
        )
        foreach ($bridgeContract in @(
            @{
                RustName = "load_library_gallery_timeline"
                DartName = "loadLibraryGalleryTimeline"
                GeneratedName = "crateApiCatalogLoadLibraryGalleryTimeline"
                ReturnType = "GalleryTimeline"
                RustApiText = $rustCatalogApiText
                DartApiText = $dartCatalogApiText
                WireModule = "catalog"
            },
            @{
                RustName = "remove_library_root"
                DartName = "removeLibraryRoot"
                GeneratedName = "crateApiCatalogRemoveLibraryRoot"
                ReturnType = "bool"
                RustApiText = $rustCatalogApiText
                DartApiText = $dartCatalogApiText
                WireModule = "catalog"
            },
            @{
                RustName = "start_catalog_database_reclamation"
                DartName = "startCatalogDatabaseReclamation"
                GeneratedName = "crateApiStorageStartCatalogDatabaseReclamation"
                ReturnType = "CatalogReclamationSnapshot"
                RustApiText = $rustStorageApiText
                DartApiText = $dartStorageApiText
                WireModule = "storage"
            },
            @{
                RustName = "load_catalog_database_reclamation"
                DartName = "loadCatalogDatabaseReclamation"
                GeneratedName = "crateApiStorageLoadCatalogDatabaseReclamation"
                ReturnType = "CatalogReclamationSnapshot"
                RustApiText = $rustStorageApiText
                DartApiText = $dartStorageApiText
                WireModule = "storage"
            },
            @{
                RustName = "cancel_catalog_database_reclamation"
                DartName = "cancelCatalogDatabaseReclamation"
                GeneratedName = "crateApiStorageCancelCatalogDatabaseReclamation"
                ReturnType = "bool"
                RustApiText = $rustStorageApiText
                DartApiText = $dartStorageApiText
                WireModule = "storage"
            },
            @{
                RustName = "load_storage_status"
                DartName = "loadStorageStatus"
                GeneratedName = "crateApiStorageLoadStorageStatus"
                ReturnType = "StorageStatus"
                RustApiText = $rustStorageApiText
                DartApiText = $dartStorageApiText
                WireModule = "storage"
            },
            @{
                RustName = "update_storage_settings"
                DartName = "updateStorageSettings"
                GeneratedName = "crateApiStorageUpdateStorageSettings"
                ReturnType = "StorageStatus"
                RustApiText = $rustStorageApiText
                DartApiText = $dartStorageApiText
                WireModule = "storage"
            }
        )) {
            $rustFunction = [regex]::Match(
                $bridgeContract.RustApiText,
                (
                    '(?ms)(?<attributes>(?:#\[[^\]]+\]\s*)*)pub fn {0}\s*\(' -f
                    [regex]::Escape($bridgeContract.RustName)
                )
            )
            if (-not $rustFunction.Success) {
                throw "Rust bridge source is missing $($bridgeContract.RustName)"
            }
            if ($rustFunction.Groups["attributes"].Value -match 'frb\s*\(\s*sync\s*\)') {
                throw "Rust bridge source made $($bridgeContract.RustName) synchronous"
            }

            $dartApiPattern = '(?ms)Future<{0}>\s+{1}\s*\(' -f @(
                [regex]::Escape($bridgeContract.ReturnType),
                [regex]::Escape($bridgeContract.DartName)
            )
            if ($bridgeContract.DartApiText -notmatch $dartApiPattern) {
                throw "Dart API wrapper is not asynchronous for $($bridgeContract.DartName)"
            }

            $generatedDartPattern = (
                '(?ms)@override\s+Future<{0}>\s+{1}\s*\(.*?' +
                'return handler\.executeNormal\s*\('
            ) -f @(
                [regex]::Escape($bridgeContract.ReturnType),
                [regex]::Escape($bridgeContract.GeneratedName)
            )
            if ($generatedDartText -notmatch $generatedDartPattern) {
                throw (
                    "Generated Dart bridge is not Future/executeNormal for " +
                    $bridgeContract.GeneratedName
                )
            }

            $wireName = (
                "wire__crate__api__$($bridgeContract.WireModule)__" +
                "$($bridgeContract.RustName)_impl"
            )
            $generatedRustFunction = [regex]::Match(
                $generatedRustText,
                (
                    '(?ms)^fn {0}\s*\(.*?(?=^fn wire__|\z)' -f
                    [regex]::Escape($wireName)
                )
            )
            if (
                -not $generatedRustFunction.Success -or
                $generatedRustFunction.Value -notmatch 'wrap_normal::' -or
                $generatedRustFunction.Value -notmatch 'FfiCallMode::Normal'
            ) {
                throw "Generated Rust bridge is not FfiCallMode::Normal for $wireName"
            }
        }

        Invoke-AmeChecked $toolchain.Git @("diff", "HEAD", "--check", "--")
    }
} finally {
    Pop-Location
    Exit-AmeRepositoryToolLock $toolLock
}
