import "package:flutter_riverpod/flutter_riverpod.dart";

import "../../../src/rust/api/catalog.dart" as rust_api;
import "../../../src/rust/domain.dart" as rust_domain;
import "../domain/library_models.dart";
import "library_catalog.dart";

abstract interface class LibraryScanner {
  Future<RecoverableLibraryScan?> loadRecoverableScan();

  Future<RecoverableLibraryScan?> loadPausedScan();

  Stream<LibraryScanUpdate> scan({
    required String scanId,
    required String rootPath,
    required int? itemLimit,
    required int? entryLimit,
    required int previewEdge,
  });

  bool cancel(String scanId);

  bool pause(String scanId);
}

class RustLibraryScanner implements LibraryScanner {
  const RustLibraryScanner();

  @override
  Future<RecoverableLibraryScan?> loadRecoverableScan() async {
    return _loadStoredScan(
      rust_api.loadRecoverableLibraryScan,
      "bridge_recoverable_scan_load_failed",
    );
  }

  @override
  Future<RecoverableLibraryScan?> loadPausedScan() async {
    return _loadStoredScan(
      rust_api.loadPausedLibraryScan,
      "bridge_paused_scan_load_failed",
    );
  }

  Future<RecoverableLibraryScan?> _loadStoredScan(
    rust_domain.RecoverableScan? Function() load,
    String fallbackCode,
  ) async {
    try {
      final scan = load();
      if (scan == null) {
        return null;
      }
      return RecoverableLibraryScan(
        scanId: scan.scanId,
        rootPath: scan.rootPath,
        displayRootPath: scan.displayRootPath,
        itemLimit: scan.maxItems,
        entryLimit: scan.maxEntries,
        previewEdge: scan.previewEdge,
        visitedEntries: scan.visitedEntries.toInt(),
        acceptedItems: scan.acceptedItems.toInt(),
        issueCount: scan.issueCount.toInt(),
      );
    } on Object catch (error) {
      if (error case rust_domain.ScanError(:final code, :final message)) {
        throw LibraryScanFailure(code: code, message: message);
      }
      throw LibraryScanFailure(code: fallbackCode, message: error.toString());
    }
  }

  @override
  Stream<LibraryScanUpdate> scan({
    required String scanId,
    required String rootPath,
    required int? itemLimit,
    required int? entryLimit,
    required int previewEdge,
  }) {
    return rust_api
        .scanLibrary(
          request: rust_domain.ScanRequest(
            scanId: scanId,
            rootPath: rootPath,
            maxItems: itemLimit,
            maxEntries: entryLimit,
            previewEdge: previewEdge,
          ),
        )
        .map(_mapEvent)
        .handleError((Object error) {
          if (error case rust_domain.ScanError(:final code, :final message)) {
            throw LibraryScanFailure(code: code, message: message);
          }
          throw LibraryScanFailure(
            code: "bridge_scan_failed",
            message: error.toString(),
          );
        });
  }

  @override
  bool cancel(String scanId) {
    return rust_api.cancelLibraryScan(scanId: scanId);
  }

  @override
  bool pause(String scanId) {
    return rust_api.pauseLibraryScan(scanId: scanId);
  }

  LibraryScanUpdate _mapEvent(rust_domain.ScanEvent event) {
    return switch (event) {
      rust_domain.ScanEvent_Started(
        :final scanId,
        :final rootPath,
        :final itemLimit,
        :final entryLimit,
      ) =>
        LibraryScanStarted(
          scanId: scanId,
          rootPath: rootPath,
          itemLimit: itemLimit,
          entryLimit: entryLimit,
        ),
      rust_domain.ScanEvent_Progress(
        :final visitedEntries,
        :final acceptedItems,
        :final issueCount,
      ) =>
        LibraryScanProgress(
          visitedEntries: visitedEntries.toInt(),
          acceptedItems: acceptedItems.toInt(),
          issueCount: issueCount.toInt(),
        ),
      rust_domain.ScanEvent_AssetDiscovered(:final asset) =>
        LibraryAssetDiscovered(mapRustLibraryAsset(asset)),
      rust_domain.ScanEvent_Issue(:final issue) => LibraryIssueDiscovered(
        LibraryIssue(
          code: issue.code,
          message: issue.message,
          path: issue.path,
        ),
      ),
      rust_domain.ScanEvent_Completed(
        :final assetCount,
        :final issueCount,
        :final catalogPath,
        :final wasLimited,
      ) =>
        LibraryScanCompleted(
          assetCount: assetCount.toInt(),
          issueCount: issueCount.toInt(),
          catalogPath: catalogPath,
          wasLimited: wasLimited,
        ),
      rust_domain.ScanEvent_Cancelled(
        :final acceptedItems,
        :final issueCount,
      ) =>
        LibraryScanCancelled(
          acceptedItems: acceptedItems.toInt(),
          issueCount: issueCount.toInt(),
        ),
      rust_domain.ScanEvent_Paused(
        :final visitedEntries,
        :final acceptedItems,
        :final issueCount,
      ) =>
        LibraryScanPaused(
          visitedEntries: visitedEntries.toInt(),
          acceptedItems: acceptedItems.toInt(),
          issueCount: issueCount.toInt(),
        ),
      rust_domain.ScanEvent_Stale(:final acceptedItems, :final issueCount) =>
        LibraryScanStale(
          acceptedItems: acceptedItems.toInt(),
          issueCount: issueCount.toInt(),
        ),
    };
  }
}

final libraryScannerProvider = Provider<LibraryScanner>((ref) {
  return const RustLibraryScanner();
});
