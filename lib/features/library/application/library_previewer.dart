import "package:flutter_riverpod/flutter_riverpod.dart";

import "ports/library_previewer.dart";

final libraryPreviewerProvider = Provider<LibraryPreviewer>((ref) {
  throw StateError("LibraryPreviewer must be bound during application startup");
});
