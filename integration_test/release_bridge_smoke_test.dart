import "dart:io";

import "package:cedarflake_ame/features/library/application/library_catalog.dart";
import "package:cedarflake_ame/features/library/domain/library_models.dart";
import "package:cedarflake_ame/src/rust/frb_generated.dart";
import "package:flutter_rust_bridge/flutter_rust_bridge_for_generated.dart";
import "package:flutter_test/flutter_test.dart";
import "package:integration_test/integration_test.dart";

void main() {
  IntegrationTestWidgetsFlutterBinding.ensureInitialized();

  setUpAll(() {
    final configuredPath =
        Platform.environment["CEDARFLAKE_AME_TEST_LIBRARY_PATH"];
    if (configuredPath == null || configuredPath.isEmpty) {
      throw StateError("CEDARFLAKE_AME_TEST_LIBRARY_PATH is required");
    }
    return RustLib.init(
      externalLibrary: ExternalLibrary.open(
        File(configuredPath).absolute.path,
        debugInfo: "aliased packaged Windows release library",
      ),
    );
  });

  testWidgets(
    "loads the configured catalog through the packaged release bridge",
    (tester) async {
      const catalog = RustLibraryCatalog();
      final snapshot = await catalog.load(
        maxItems: libraryCatalogWindow,
        query: const LibraryGalleryQuery(),
      );

      expect(snapshot.catalogPath, isNotEmpty);
      expect(snapshot.queryId, isNotEmpty);
      expect(snapshot.assets.length, lessThanOrEqualTo(libraryCatalogWindow));
      expect(snapshot.previousCursor, isNull);

      final rootIds = snapshot.roots.map((root) => root.id).toSet();
      expect(rootIds.length, snapshot.roots.length);
      for (final asset in snapshot.assets) {
        expect(rootIds, contains(asset.rootId));
      }

      final nextCursor = snapshot.nextCursor;
      if (nextCursor != null) {
        expect(nextCursor.revision, snapshot.revision);
        expect(nextCursor.queryId, snapshot.queryId);
      }
    },
  );
}
