import "package:cedarflake_ame/features/library/application/library_controller.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_source_navigation_tile.dart";
import "package:flutter/material.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:flutter_test/flutter_test.dart";

Future<void> verifyCoveredFolderWorkflow(
  WidgetTester tester,
  ProviderContainer container, {
  required String childPath,
  required String relativePath,
}) async {
  final before = container.read(libraryControllerProvider);
  final parent = before.roots.single;
  final asset = before.assets.single;
  final controller = container.read(libraryControllerProvider.notifier);

  await controller.scanDirectory(childPath);
  final deadline = DateTime.now().add(const Duration(seconds: 30));
  while (true) {
    final current = container.read(libraryControllerProvider);
    if (!current.isProcessing &&
        current.query.rootId == parent.id &&
        current.query.folderRelativePath == relativePath) {
      break;
    }
    if (DateTime.now().isAfter(deadline)) {
      throw TestFailure(
        "Covered folder selection did not settle: ${current.status.name}",
      );
    }
    await tester.pump(const Duration(milliseconds: 25));
  }
  await tester.pump();

  final covered = container.read(libraryControllerProvider);
  expect(covered.roots, hasLength(1));
  expect(covered.roots.single.id, parent.id);
  expect(covered.roots.single.activeScanId, parent.activeScanId);
  expect(covered.assets, hasLength(1));
  expect(covered.assets.single.assetId, asset.assetId);
  expect(covered.assets.single.locationId, asset.locationId);
  expect(covered.query.includeDescendants, isTrue);
  expect(covered.taskKind, isNull);
  expect(covered.hasRetainedScan, isFalse);
  expect(find.byType(PendingLibrarySourceTile), findsNothing);
  expect(find.byKey(const Key("library-photo-wall")), findsOneWidget);

  expect(await controller.updateQuery(before.query), isTrue);
  await tester.pump();
  final restored = container.read(libraryControllerProvider);
  expect(restored.roots, hasLength(1));
  expect(restored.assets.single.assetId, asset.assetId);
  expect(restored.assets.single.locationId, asset.locationId);
  expect(restored.roots.single.activeScanId, parent.activeScanId);
}
