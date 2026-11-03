import "package:flutter_riverpod/flutter_riverpod.dart";

import "ports/library_scanner.dart";

final libraryScannerProvider = Provider<LibraryScanner>((ref) {
  throw StateError("LibraryScanner must be bound during application startup");
});
