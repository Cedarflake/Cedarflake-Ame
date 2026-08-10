import "package:flutter_riverpod/flutter_riverpod.dart";

import "../../../src/rust/api/preview.dart" as rust_api;
import "../../../src/rust/domain.dart" as rust_domain;
import "../domain/library_models.dart";
import "library_catalog.dart";

abstract interface class LibraryPreviewer {
  Future<LibraryAsset> materialize({
    required String locationId,
    required int previewEdge,
    bool retry = false,
  });
}

class RustLibraryPreviewer implements LibraryPreviewer {
  const RustLibraryPreviewer();

  @override
  Future<LibraryAsset> materialize({
    required String locationId,
    required int previewEdge,
    bool retry = false,
  }) async {
    try {
      final asset = await rust_api.materializeLibraryPreview(
        request: rust_domain.PreviewRequest(
          locationId: locationId,
          previewEdge: previewEdge,
          retryFailed: retry,
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
