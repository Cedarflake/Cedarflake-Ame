import "../domain/library_models.dart";
import "../domain/library_state.dart";
import "library_catalog.dart";
import "library_scan_source_admission.dart";
import "library_scanner.dart";

sealed class LibraryScanCompletionResolution {
  const LibraryScanCompletionResolution();
}

class LibraryResolvedScanUpdate extends LibraryScanCompletionResolution {
  const LibraryResolvedScanUpdate(this.update);
  final LibraryScanUpdate update;
}

class LibraryResolvedPausedScan extends LibraryScanCompletionResolution {
  const LibraryResolvedPausedScan(this.scan);
  final RecoverableLibraryScan scan;
}

/// Retains the original source scope until a drained scan's durable outcome is known.
class LibraryScanCompletionRecovery {
  const LibraryScanCompletionRecovery({
    required this.scanId,
    required this.didStart,
    required this.sourceAdmission,
    required this.snapshot,
  });

  final String scanId;
  final bool didStart;
  final LibraryScanSourceAdmission sourceAdmission;
  final LibraryPrimaryScanSnapshot snapshot;

  Future<LibraryScanCompletionResolution> resolve({
    required LibraryCatalog catalog,
    required LibraryScanner scanner,
    required LibraryGalleryQuery query,
  }) async {
    final current = await catalog.load(
      maxItems: libraryCatalogWindow,
      query: query,
    );
    final published = current.roots.where(
      (root) => root.activeScanId == scanId,
    );
    if (didStart && published.isNotEmpty) {
      final root = published.first;
      final itemLimit = snapshot.itemLimit;
      final entryLimit = snapshot.entryLimit;
      final wasLimited =
          (itemLimit != null && root.assetCount >= itemLimit) ||
          (entryLimit != null && snapshot.visitedEntries >= entryLimit);
      return LibraryResolvedScanUpdate(
        LibraryScanCompleted(
          assetCount: root.assetCount,
          issueCount: root.issueCount,
          catalogPath: current.catalogPath,
          wasLimited: wasLimited,
          rootConsolidation: sourceAdmission.recoverPublishedConsolidation(
            root,
            current.roots,
          ),
        ),
      );
    }
    final paused = await scanner.loadPausedScan();
    if (paused != null && paused.scanId == scanId) {
      return LibraryResolvedPausedScan(paused);
    }
    if (snapshot.status == LibraryStatus.cancelling) {
      return LibraryResolvedScanUpdate(
        LibraryScanCancelled(
          acceptedItems: snapshot.stagedAssetCount,
          issueCount: snapshot.issueCount,
        ),
      );
    }
    return const LibraryResolvedScanUpdate(
      LibraryScanFailed(
        code: "scan_ended_without_completion",
        message: "The scan ended without a completion event",
      ),
    );
  }
}
