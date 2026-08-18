import "package:flutter/material.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";

import "app/ame_app.dart";
import "app/bootstrap/ame_bootstrap_failure.dart";
import "app/bootstrap/rust_library_loader.dart";
import "app/window/ame_window_actions.dart";
import "app/window/ame_shutdown_coordinator.dart";
import "app/window/window_manager_actions.dart";
import "features/library/application/library_catalog.dart";
import "features/library/application/library_controller.dart";
import "features/library/application/library_synchronization.dart";
import "features/library/application/library_view_preferences.dart";
import "features/library/domain/library_models.dart";
import "features/library/domain/library_state.dart";
import "features/settings/application/ame_preferences.dart";
import "features/settings/adapters/shared_preferences_ame_store.dart";

Future<void> main() async {
  WidgetsFlutterBinding.ensureInitialized();
  final preferenceStore = SharedPreferencesAmeStore();
  final shutdownCoordinator = AmeShutdownCoordinator();
  final amePreferences = await preferenceStore.loadAmePreferences();
  final viewPreferences = await preferenceStore.loadLibraryViewPreferences();
  final windowActions = await initializeAmeWindow(
    preferenceStore,
    shutdownCoordinator,
  );

  try {
    await initializeRustLibrary();
    const catalog = RustLibraryCatalog();
    final query = LibraryGalleryQuery(
      sortKey: viewPreferences.sortKey,
      sortDirection: viewPreferences.sortDirection,
    );
    final snapshot = await catalog.load(
      maxItems: libraryCatalogWindow,
      query: query,
    );
    final initialState = LibraryState.fromSnapshot(snapshot, query: query);
    final synchronization = RustLibrarySynchronization();
    shutdownCoordinator.register(synchronization.stop);
    await synchronization.start();
    runApp(
      ProviderScope(
        overrides: [
          ameWindowActionsProvider.overrideWithValue(windowActions),
          initialAmePreferencesProvider.overrideWithValue(amePreferences),
          amePreferenceStoreProvider.overrideWithValue(preferenceStore),
          initialLibraryViewPreferencesProvider.overrideWithValue(
            viewPreferences,
          ),
          libraryCatalogProvider.overrideWithValue(catalog),
          librarySynchronizationProvider.overrideWithValue(synchronization),
          initialLibraryStateProvider.overrideWithValue(initialState),
          libraryViewPreferenceStoreProvider.overrideWithValue(preferenceStore),
        ],
        child: const AmeApp(),
      ),
    );
  } on Object catch (error) {
    runApp(
      ProviderScope(
        overrides: [ameWindowActionsProvider.overrideWithValue(windowActions)],
        child: AmeBootstrapFailure(error: error, preferences: amePreferences),
      ),
    );
  }
}
