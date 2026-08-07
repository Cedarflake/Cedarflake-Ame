import "package:flutter/material.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";

import "app/ame_app.dart";
import "features/library/application/library_catalog.dart";
import "features/library/application/library_controller.dart";
import "features/library/domain/library_models.dart";
import "features/library/domain/library_state.dart";
import "src/rust/frb_generated.dart";

Future<void> main() async {
  WidgetsFlutterBinding.ensureInitialized();

  try {
    await RustLib.init();
    const catalog = RustLibraryCatalog();
    const query = LibraryGalleryQuery();
    final snapshot = await catalog.load(
      maxItems: libraryCatalogWindow,
      query: query,
    );
    final initialState = LibraryState.fromSnapshot(snapshot, query: query);
    runApp(
      ProviderScope(
        overrides: [
          libraryCatalogProvider.overrideWithValue(catalog),
          initialLibraryStateProvider.overrideWithValue(initialState),
        ],
        child: const AmeApp(),
      ),
    );
  } on Object catch (error) {
    runApp(AmeBootstrapFailure(error: error));
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
      home: Scaffold(
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
                  SelectableText(error.toString(), textAlign: TextAlign.center),
                ],
              ),
            ),
          ),
        ),
      ),
    );
  }
}
