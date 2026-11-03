import "package:flutter_riverpod/flutter_riverpod.dart";

import "ports/library_source_reader.dart";

final librarySourceReaderProvider = Provider<LibrarySourceReader>((ref) {
  throw StateError(
    "LibrarySourceReader must be bound during application startup",
  );
});
