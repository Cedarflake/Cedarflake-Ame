import "../../../../src/rust/api/preview.dart" as rust_api;
import "../../../../src/rust/domain.dart" as rust_domain;
import "../../domain/library_models.dart";
import "library_asset_mapping.dart";
import "../../application/ports/library_foreground_read_activity.dart";

import "../../application/ports/library_previewer.dart";

class RustLibraryPreviewer implements LibraryPreviewer {
  const RustLibraryPreviewer({required this.foregroundReads});

  final LibraryForegroundReadActivity foregroundReads;

  @override
  Future<LibraryAsset> materialize({
    required String locationId,
    required String expectedRootId,
    required String expectedScanId,
    required LibrarySourceRevisionEvidence? expectedSourceRevision,
    required BigInt expectedSourceGeneration,
    required int previewEdge,
    bool force = false,
    Iterable<String> protectedLocationIds = const [],
  }) async {
    final foreground = foregroundReads.acquire();
    try {
      final asset = await rust_api.materializeLibraryPreview(
        request: rust_domain.PreviewRequest(
          locationId: locationId,
          expectedRootId: expectedRootId,
          expectedScanId: expectedScanId,
          expectedSourceRevision: expectedSourceRevision == null
              ? null
              : rust_domain.SourceRevisionEvidence(
                  scheme: expectedSourceRevision.scheme,
                  value: expectedSourceRevision.value,
                ),
          expectedSourceGeneration: expectedSourceGeneration,
          previewEdge: previewEdge,
          retryFailed: force,
          protectedLocationIds: protectedLocationIds.toList(growable: false),
        ),
      );
      return mapRustLibraryAsset(asset);
    } on Object catch (error) {
      if (error case rust_domain.ScanError(:final code, :final message)) {
        throw LibraryPreviewFailure(code: code, message: message);
      }
      throw LibraryPreviewFailure(
        code: "bridge_preview_failed",
        message: error.toString(),
      );
    } finally {
      foreground.close();
    }
  }
}
