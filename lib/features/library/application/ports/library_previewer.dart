import "../../domain/library_models.dart";

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

class LibraryPreviewFailure implements Exception {
  const LibraryPreviewFailure({required this.code, required this.message});

  final String code;
  final String message;

  @override
  String toString() => "$code: $message";
}
