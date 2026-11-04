import "package:flutter_riverpod/flutter_riverpod.dart";

import "ports/library_platform_actions.dart";

final libraryPlatformActionsProvider = Provider<LibraryPlatformActions>((ref) {
  throw StateError(
    "LibraryPlatformActions must be bound during application startup",
  );
});
