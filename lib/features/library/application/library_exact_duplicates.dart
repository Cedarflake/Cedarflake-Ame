import "package:flutter_riverpod/flutter_riverpod.dart";

import "ports/library_exact_duplicates.dart";

final libraryExactDuplicatesProvider = Provider<LibraryExactDuplicates>(
  (ref) => throw StateError(
    "LibraryExactDuplicates must be bound during application startup",
  ),
);
