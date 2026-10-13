import "package:cedarflake_ame/features/library/application/library_scanner.dart";
import "package:cedarflake_ame/features/library/application/library_update_controller.dart";
import "package:cedarflake_ame/features/library/domain/library_models.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:flutter_test/flutter_test.dart";

import "../support/concurrent_library_scanner.dart";

void main() {
  for (final cancel in [false, true]) {
    test(
      "updates publish checkpoint progress and retain exact terminal counts (cancel: $cancel)",
      () async {
        const root = LibraryRoot(
          id: "root",
          path: r"C:\Pictures",
          displayPath: r"C:\Pictures",
          activeScanId: "published",
          createdUnixMs: 1,
          assetCount: 103,
          issueCount: 0,
          availability: LibraryRootAvailability.available,
        );
        final scanner = ConcurrentLibraryScanner();
        var refreshes = 0;
        final container = ProviderContainer(
          overrides: [
            libraryScannerProvider.overrideWithValue(scanner),
            libraryUpdateConfiguredRootsProvider.overrideWithValue([root]),
            libraryUpdatePrimaryBusyProvider.overrideWithValue(false),
            libraryUpdateCatalogRefreshProvider.overrideWithValue(() async {
              refreshes += 1;
            }),
          ],
        );
        addTearDown(container.dispose);
        addTearDown(scanner.dispose);
        final controller = container.read(
          libraryUpdateControllerProvider.notifier,
        );
        controller.startUpdates([root.id]);
        final scanId = scanner.scanIdForRootPath(root.path);
        final notifications = <LibraryRootUpdateTask>[];
        final subscription = container.listen(libraryUpdateControllerProvider, (
          _,
          next,
        ) {
          notifications.add(next.tasksByRootId[root.id]!);
        });
        addTearDown(subscription.close);

        for (var index = 1; index <= 100; index += 1) {
          scanner.add(scanId, LibraryAssetDiscovered(_asset(index, scanId)));
          if (index % 25 == 0) {
            scanner.add(
              scanId,
              LibraryScanProgress(
                visitedEntries: index + 1,
                acceptedItems: index,
                issueCount: 0,
              ),
            );
          }
        }
        await Future<void>.delayed(Duration.zero);

        expect(
          notifications,
          hasLength(4),
          reason:
              "image payloads must not invalidate the retained gallery for every scanned file",
        );
        expect(notifications.map((task) => task.acceptedItems), [
          25,
          50,
          75,
          100,
        ]);
        expect(notifications.last.visitedEntries, 101);

        if (cancel) {
          controller.cancel(root.id);
          expect(scanner.cancelledScanIds, [scanId]);
          expect(notifications.last.phase, LibraryRootUpdatePhase.cancelling);
        }
        for (var index = 101; index <= 103; index += 1) {
          scanner.add(scanId, LibraryAssetDiscovered(_asset(index, scanId)));
        }
        await Future<void>.delayed(Duration.zero);
        expect(notifications.last.acceptedItems, 100);
        if (cancel) {
          scanner.add(
            scanId,
            const LibraryScanCancelled(acceptedItems: 103, issueCount: 0),
          );
        } else {
          scanner.add(
            scanId,
            const LibraryScanCompleted(
              assetCount: 103,
              issueCount: 0,
              catalogPath: "catalog.sqlite3",
              wasLimited: false,
            ),
          );
        }
        await scanner.close(scanId);
        await Future<void>.delayed(Duration.zero);
        final terminal = container
            .read(libraryUpdateControllerProvider)
            .tasksByRootId[root.id];
        expect(terminal?.acceptedItems, 103);
        expect(
          terminal?.phase,
          cancel
              ? LibraryRootUpdatePhase.cancelled
              : LibraryRootUpdatePhase.completed,
        );
        expect(refreshes, cancel ? 0 : 1);
      },
    );
  }
}

LibraryAsset _asset(int index, String scanId) => LibraryAsset(
  assetId: "asset-$index",
  locationId: "location-$index",
  rootId: "root",
  activeScanId: scanId,
  sourcePath: "C:/Pictures/$index.png",
  displayPath: "C:/Pictures/$index.png",
  relativePath: "$index.png",
  previewPath: "",
  fileSize: BigInt.from(100),
  modifiedUnixMs: 1,
  sourceRevision: null,
  sourceGeneration: BigInt.one,
  width: 100,
  height: 100,
);
