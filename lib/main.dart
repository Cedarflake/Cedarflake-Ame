import "package:flutter/material.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";

import "adapters/rust_library_loader.dart";
import "adapters/shared_preferences_ame_store.dart";
import "adapters/window_manager_actions.dart";
import "app/ame_app.dart";
import "app/ame_window_frame.dart";
import "app/window/ame_window_actions.dart";
import "features/library/application/library_catalog.dart";
import "features/library/application/library_controller.dart";
import "features/library/application/library_view_preferences.dart";
import "features/library/domain/library_models.dart";
import "features/library/domain/library_state.dart";
import "features/settings/application/ame_preferences.dart";

Future<void> main() async {
  WidgetsFlutterBinding.ensureInitialized();
  final preferenceStore = SharedPreferencesAmeStore();
  final amePreferences = await preferenceStore.loadAmePreferences();
  final viewPreferences = await preferenceStore.loadLibraryViewPreferences();
  final windowActions = await initializeAmeWindow(preferenceStore);

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
        child: AmeBootstrapFailure(error: error),
      ),
    );
  }
}

class AmeBootstrapFailure extends StatelessWidget {
  const AmeBootstrapFailure({required this.error, super.key});

  final Object error;

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      debugShowCheckedModeBanner: false,
      theme: ThemeData(
        colorScheme: ColorScheme.fromSeed(seedColor: const Color(0xFF6750A4)),
        useMaterial3: true,
      ),
      home: AmeWindowFrame(
        child: Scaffold(
          body: Center(
            child: ConstrainedBox(
              constraints: const BoxConstraints(maxWidth: 560),
              child: Padding(
                padding: const EdgeInsets.all(32),
                child: Column(
                  mainAxisSize: MainAxisSize.min,
                  children: [
                    const Icon(Icons.error_outline, size: 48),
                    const SizedBox(height: 20),
                    Text(
                      "Cedarflake Ame could not start",
                      style: Theme.of(context).textTheme.headlineSmall,
                    ),
                    const SizedBox(height: 12),
                    SelectableText(
                      error.toString(),
                      textAlign: TextAlign.center,
                    ),
                  ],
                ),
              ),
            ),
          ),
        ),
      ),
    );
  }
}
