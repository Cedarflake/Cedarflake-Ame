import "../../../src/rust/api/viewer_source.dart" as rust_api;
import "../../../src/rust/application/viewer_source.dart" as rust_source;
import "../../../src/rust/domain.dart" as rust_domain;
import "../domain/library_models.dart";
import "library_foreground_reads.dart";
import "library_source_read_scheduler.dart";
import "library_source_reader.dart";

final librarySourceReadScheduler = LibrarySourceReadScheduler(
  reader: const RustLibrarySourceReader(),
);

class RustLibrarySourceReader implements LibrarySourceReader {
  const RustLibrarySourceReader();

  @override
  Future<LibrarySourceReadLease> acquire(LibraryAsset asset) async {
    final foreground = libraryForegroundReads.acquire();
    try {
      final revision = asset.sourceRevision;
      final lease = await rust_api.acquireViewerSource(
        request: rust_source.ViewerSourceRequest(
          locationId: asset.locationId,
          expectedRootId: asset.rootId,
          expectedScanId: asset.activeScanId,
          expectedSourceRevision: revision == null
              ? null
              : rust_domain.SourceRevisionEvidence(
                  scheme: revision.scheme,
                  value: revision.value,
                ),
          expectedSourceGeneration: asset.sourceGeneration,
        ),
      );
      try {
        return _RustSourceReadLease(
          await lease.sourcePath(),
          lease,
          foreground,
        );
      } on Object {
        await lease.close();
        rethrow;
      }
    } on Object catch (error) {
      foreground.close();
      if (error case rust_domain.ScanError(:final code, :final message)) {
        throw LibrarySourceReadFailure(code, message);
      }
      throw LibrarySourceReadFailure(
        "viewer_source_bridge_failed",
        "The original-image reader is unavailable: $error",
      );
    }
  }
}

class _RustSourceReadLease implements LibrarySourceReadLease {
  _RustSourceReadLease(this.sourcePath, this._lease, this._foreground);

  @override
  final String sourcePath;
  final rust_api.ViewerSourceReadLease _lease;
  final LibraryForegroundReadLease _foreground;
  Future<void>? _closing;

  @override
  Future<void> close() => _closing ??= _close();

  Future<void> _close() async {
    try {
      await _lease.close();
    } finally {
      _foreground.close();
    }
  }
}
