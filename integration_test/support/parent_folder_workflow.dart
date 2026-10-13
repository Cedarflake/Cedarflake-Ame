import "dart:convert";
import "dart:io";

import "package:cedarflake_ame/app/ame_app.dart";
import "package:cedarflake_ame/features/library/application/library_catalog.dart";
import "package:cedarflake_ame/features/library/application/library_controller.dart";
import "package:cedarflake_ame/features/library/application/library_synchronization.dart";
import "package:cedarflake_ame/features/library/domain/library_models.dart";
import "package:cedarflake_ame/features/library/domain/library_state.dart";
import "package:cedarflake_ame/features/library/presentation/unified_library_screen.dart";
import "package:flutter/material.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:flutter_test/flutter_test.dart";

void registerParentFolderWorkflowTests(
  LibrarySynchronization Function() readSynchronization,
) {
  testWidgets(
    "complete parent import replaces its child and retains the current folder",
    (tester) async {
      final source = await Directory(
        "${Directory.current.path}/build",
      ).createTemp("integration-parent-");
      final child = await Directory("${source.path}/旅行").create();
      final nested = await Directory("${child.path}/海边").create();
      final bytes = base64Decode(
        "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNk+A8AAQUBAScY42YAAAAASUVORK5CYII=",
      );
      final inner = File("${nested.path}/pixel.png");
      final outer = File("${source.path}/outer.png");
      await inner.writeAsBytes(bytes, flush: true);
      await outer.writeAsBytes(bytes, flush: true);
      addTearDown(() async {
        if (await source.exists()) await source.delete(recursive: true);
      });
      await tester.pumpWidget(
        ProviderScope(
          overrides: [
            librarySynchronizationProvider.overrideWithValue(
              readSynchronization(),
            ),
          ],
          child: const AmeApp(),
        ),
      );
      final container = ProviderScope.containerOf(
        tester.element(find.byType(UnifiedLibraryScreen)),
      );
      final controller = container.read(libraryControllerProvider.notifier);
      await controller.scanDirectory(child.path);
      await _pumpUntil(
        tester,
        () =>
            container.read(libraryControllerProvider).status ==
            LibraryStatus.completed,
      );
      final before = container.read(libraryControllerProvider);
      expect(before.roots, hasLength(1));
      expect(before.assets, hasLength(1));
      final childRoot = before.roots.single;
      final childAsset = before.assets.single;
      final query = LibraryGalleryQuery(
        rootId: childRoot.id,
        folderRelativePath: "海边",
        includeDescendants: false,
        searchText: "pixel",
        sortKey: LibraryGallerySortKey.fileName,
        sortDirection: LibraryGallerySortDirection.ascending,
      );
      expect(await controller.updateQuery(query), isTrue);
      await controller.scanDirectory(source.path);
      await _pumpUntil(tester, () {
        final current = container.read(libraryControllerProvider);
        if (current.status == LibraryStatus.failed) {
          throw TestFailure("Parent import failed: ${current.errorMessage}");
        }
        return current.status == LibraryStatus.completed &&
            current.roots.length == 1 &&
            current.roots.single.id != childRoot.id;
      });
      final after = container.read(libraryControllerProvider);
      final parent = after.roots.single;
      expect(parent.assetCount, 2);
      expect(parent.activeScanId, isNot(childRoot.activeScanId));
      expect(
        after.query,
        query.copyWith(rootId: parent.id, folderRelativePath: "旅行/海边"),
      );
      expect(after.assets.single.assetId, childAsset.assetId);
      expect(find.byKey(const Key("library-photo-wall")), findsOneWidget);
      await _pumpUntil(
        tester,
        () =>
            controller
                .resolvePreview(
                  container.read(libraryControllerProvider).assets.single,
                )
                .previewStatus ==
            LibraryPreviewStatus.ready,
      );
      expect(await controller.updateQuery(const LibraryGalleryQuery()), isTrue);
      final combined = container.read(libraryControllerProvider);
      expect(combined.assets, hasLength(2));
      expect(
        combined.assets.map((asset) => asset.assetId).toSet(),
        hasLength(2),
      );
      expect(combined.roots, hasLength(1));
      expect(await inner.readAsBytes(), bytes);
      expect(await outer.readAsBytes(), bytes);
      await tester.pumpWidget(const SizedBox.shrink());
      await tester.pumpAndSettle();
      const catalog = RustLibraryCatalog();
      final reopened = await catalog.load(
        maxItems: libraryCatalogWindow,
        query: const LibraryGalleryQuery(),
      );
      expect(reopened.roots.single.id, parent.id);
      expect(reopened.assets, hasLength(2));
      expect(await catalog.unregisterRoot(parent.id), isTrue);
      expect(await inner.readAsBytes(), bytes);
      expect(await outer.readAsBytes(), bytes);
    },
  );
}

Future<void> _pumpUntil(WidgetTester tester, bool Function() condition) async {
  final deadline = DateTime.now().add(const Duration(seconds: 30));
  while (!condition()) {
    if (DateTime.now().isAfter(deadline)) {
      throw TestFailure("Parent-folder workflow did not settle");
    }
    await tester.pump(const Duration(milliseconds: 25));
  }
  await tester.pump();
}
