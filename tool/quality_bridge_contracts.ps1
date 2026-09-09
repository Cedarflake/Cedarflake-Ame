. (Join-Path $PSScriptRoot "quality_bridge_source.ps1")

class AmeAsyncBridgeContract {
    [string]$Module
    [string]$RustName
    [string]$DartName
    [string]$GeneratedName
    [string]$ReturnType
    [string]$Owner

    AmeAsyncBridgeContract([string]$module, [string]$rust, [string]$dart,
        [string]$generated, [string]$returnType, [string]$owner) {
        $this.Module = $module
        $this.RustName = $rust
        $this.DartName = $dart
        $this.GeneratedName = $generated
        $this.ReturnType = $returnType
        $this.Owner = $owner
    }
}

function Get-AmeAsyncBridgeContracts {
    $rows = @(
        @("catalog", "load_library_query_snapshot", "loadLibraryQuerySnapshot", "crateApiCatalogLoadLibraryQuerySnapshot", "GalleryQuerySnapshot", ""),
        @("catalog", "load_library_folder_page", "loadLibraryFolderPage", "crateApiCatalogLoadLibraryFolderPage", "LibraryFolderPage", ""),
        @("catalog", "load_recoverable_library_scan", "loadRecoverableLibraryScan", "crateApiCatalogLoadRecoverableLibraryScan", "RecoverableScan?", ""),
        @("catalog", "load_paused_library_scan", "loadPausedLibraryScan", "crateApiCatalogLoadPausedLibraryScan", "RecoverableScan?", ""),
        @("catalog", "cancel_retained_library_scan", "cancelRetainedLibraryScan", "crateApiCatalogCancelRetainedLibraryScan", "void", ""),
        @("catalog", "load_library_gallery_timeline", "loadLibraryGalleryTimeline", "crateApiCatalogLoadLibraryGalleryTimeline", "GalleryTimeline", ""),
        @("catalog", "remove_library_root", "removeLibraryRoot", "crateApiCatalogRemoveLibraryRoot", "bool", ""),
        @("storage", "start_catalog_database_reclamation", "startCatalogDatabaseReclamation", "crateApiStorageStartCatalogDatabaseReclamation", "CatalogReclamationSnapshot", ""),
        @("storage", "load_catalog_database_reclamation", "loadCatalogDatabaseReclamation", "crateApiStorageLoadCatalogDatabaseReclamation", "CatalogReclamationSnapshot", ""),
        @("storage", "cancel_catalog_database_reclamation", "cancelCatalogDatabaseReclamation", "crateApiStorageCancelCatalogDatabaseReclamation", "bool", ""),
        @("storage", "load_storage_status", "loadStorageStatus", "crateApiStorageLoadStorageStatus", "StorageStatus", ""),
        @("storage", "update_storage_settings", "updateStorageSettings", "crateApiStorageUpdateStorageSettings", "StorageStatus", ""),
        @("viewer_source", "acquire_viewer_source", "acquireViewerSource", "crateApiViewerSourceAcquireViewerSource", "ViewerSourceReadLease", ""),
        @("viewer_source", "source_path", "sourcePath", "crateApiViewerSourceViewerSourceReadLeaseSourcePath", "String", "ViewerSourceReadLease"),
        @("viewer_source", "close", "close", "crateApiViewerSourceViewerSourceReadLeaseClose", "void", "ViewerSourceReadLease")
    )
    foreach ($row in $rows) {
        [AmeAsyncBridgeContract]::new($row[0], $row[1], $row[2], $row[3], $row[4], $row[5])
    }
}

function Assert-AmeAsyncBridgeContract {
    param(
        [Parameter(Mandatory = $true)][AmeAsyncBridgeContract]$Contract,
        [Parameter(Mandatory = $true)][string]$RustApiCode,
        [Parameter(Mandatory = $true)][string]$DartApiCode,
        [Parameter(Mandatory = $true)][string]$GeneratedDartCode,
        [Parameter(Mandatory = $true)][string]$GeneratedRustCode
    )

    $rust = Get-AmeBridgeMethod $RustApiCode $Contract.RustName RustApi $Contract.Owner
    if ($rust.Attributes -match '\bfrb\s*\([^)]*\bsync\b') {
        throw "Rust bridge source made $($Contract.RustName) synchronous"
    }
    $dart = Get-AmeBridgeMethod $DartApiCode $Contract.DartName DartApi $Contract.Owner
    $generated = Get-AmeBridgeMethod $GeneratedDartCode $Contract.GeneratedName DartGenerated
    $futurePattern = '\bFuture\s*<\s*{0}\s*>\s+' -f [regex]::Escape($Contract.ReturnType)
    if ($dart.Signature -notmatch $futurePattern -or $generated.Signature -notmatch $futurePattern) {
        throw "Dart bridge return type is not Future<$($Contract.ReturnType)> for $($Contract.DartName)"
    }
    if ($generated.Body -notmatch '^\{\s*return\s+handler\s*\.\s*executeNormal\s*\(' -or
        [regex]::Matches($generated.Body, '\bhandler\s*\.\s*executeNormal\s*\(').Count -ne 1 -or
        $generated.Body -match '\b(?:executeSync|SyncTask)\b') {
        throw "Generated Dart method is not exclusively executeNormal for $($Contract.GeneratedName)"
    }
    $delegatePattern = '^=>\s*RustLib\s*\.\s*instance\s*\.\s*api\s*\.\s*{0}\s*\(' -f
        [regex]::Escape($Contract.GeneratedName)
    if ($Contract.Owner) {
        $implementation = Get-AmeBridgeMethod $GeneratedDartCode $Contract.DartName DartApi "$($Contract.Owner)Impl"
        $opaqueDelegatePattern = $delegatePattern + '\s*that\s*:\s*this\s*,?\s*\)\s*;$'
        if ($implementation.Signature -notmatch $futurePattern -or
            $implementation.Body -notmatch $opaqueDelegatePattern) {
            throw "Opaque Dart implementation does not delegate its exact lease: $($Contract.Owner).$($Contract.DartName)"
        }
    } elseif ($dart.Body -notmatch $delegatePattern) {
        throw "Dart API wrapper does not call its exact generated method: $($Contract.DartName)"
    }
    $wireStem = if ($Contract.Owner) { "$($Contract.Owner)_$($Contract.RustName)" } else { $Contract.RustName }
    $wireName = "wire__crate__api__$($Contract.Module)__$($wireStem)_impl"
    $wire = Get-AmeBridgeMethod $GeneratedRustCode $wireName RustWire
    if ($wire.Body -notmatch '^\{\s*FLUTTER_RUST_BRIDGE_HANDLER\s*\.\s*wrap_normal\s*::' -or
        [regex]::Matches($wire.Body, '\bwrap_normal\s*::').Count -ne 1 -or
        [regex]::Matches($wire.Body, '\bFfiCallMode\s*::\s*Normal\b').Count -ne 1 -or
        $wire.Body -match '\b(?:wrap_sync|FfiCallMode\s*::\s*Sync)\b') {
        throw "Generated Rust method is not exclusively FfiCallMode::Normal for $wireName"
    }
}
