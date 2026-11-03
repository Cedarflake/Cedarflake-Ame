import "package:flutter_riverpod/flutter_riverpod.dart";

import "../domain/library_view_preferences.dart";
import "ports/library_view_preference_store.dart";

final initialLibraryViewPreferencesProvider = Provider<LibraryViewPreferences>(
  (ref) => const LibraryViewPreferences(),
);

final libraryViewPreferenceStoreProvider = Provider<LibraryViewPreferenceStore>(
  (ref) => throw StateError(
    "LibraryViewPreferenceStore must be bound during application startup",
  ),
);
