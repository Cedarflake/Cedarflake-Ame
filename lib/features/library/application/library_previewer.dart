import "package:flutter_riverpod/flutter_riverpod.dart";

import "../../../src/rust/api/preview.dart" as rust_api;
import "../../../src/rust/domain.dart" as rust_domain;
import "../domain/library_models.dart";
import "library_catalog.dart";

abstract interface class LibraryPreviewer {
  Future<LibraryAsset> materialize({
    required String locationId,
    required String expectedRootId,
    required String expectedScanId,
    required LibrarySourceRevisionEvidence? expectedSourceRevision,
    required BigInt expectedSourceGeneration,
    required int previewEdge,
    bool force = false,
    Iterable<String> protectedLocationIds = const [],
  });
}

class RustLibraryPreviewer implements LibraryPreviewer {
  const RustLibraryPreviewer();

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
    }
  }
}

class LibraryPreviewFailure implements Exception {
  const LibraryPreviewFailure({required this.code, required this.message});

  final String code;
  final String message;

  @override
  String toString() => "$code: $message";
}

final libraryPreviewerProvider = Provider<LibraryPreviewer>((ref) {
  return const RustLibraryPreviewer();
});
