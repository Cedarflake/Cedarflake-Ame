import "package:flutter_riverpod/flutter_riverpod.dart";

import "ports/library_image_clipboard.dart";

final libraryImageClipboardProvider = Provider<LibraryImageClipboard>((ref) {
  throw StateError(
    "LibraryImageClipboard must be bound during application startup",
  );
});
