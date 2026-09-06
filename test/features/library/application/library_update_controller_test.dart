import "dart:async";

import "package:cedarflake_ame/features/library/application/library_catalog.dart";
import "package:cedarflake_ame/features/library/application/library_controller.dart";
import "package:cedarflake_ame/features/library/application/library_scan_shutdown.dart";
import "package:cedarflake_ame/features/library/application/library_scan_execution.dart";
import "package:cedarflake_ame/features/library/application/library_scanner.dart";
import "package:cedarflake_ame/features/library/application/library_update_controller.dart";
import "package:cedarflake_ame/features/library/domain/library_models.dart";
import "package:cedarflake_ame/features/library/domain/library_state.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:flutter_test/flutter_test.dart";

void main() {
  for (final duringCommittedReload in [false, true]) {
    test(
      "independent update refresh waits for root removal (committed reload: $duringCommittedReload)",
      () async {
        final roots = [
          _root("root-a", "C:\\A"),
          _root("root-b", "C:\\B"),
          _root("root-c", "C:\\C"),
        ];
        final scanner = _ConcurrentLibraryScanner();
        final catalog = _RemovingUpdateCatalog(roots);
        final initial = catalog.snapshot;
        final container = ProviderContainer(
          overrides: [
            libraryScannerProvider.overrideWithValue(scanner),
            libraryCatalogProvider.overrideWithValue(catalog),
            initialLibraryStateProvider.overrideWithValue(
              LibraryState.fromSnapshot(
                initial,
                query: const LibraryGalleryQuery(rootId: "root-c"),
              ).copyWith(
                timeline: LibraryTimeline(
                  revision: initial.revision,
                  queryId: initial.queryId,
                  totalItems: 3,
                  buckets: const [],
                ),
              ),
            ),
          ],
        );
        addTearDown(scanner.dispose);
        addTearDown(container.dispose);
        final publishedAfterCommit = <LibraryState>[];
        final subscription = container.listen(libraryControllerProvider, (
          _,
          state,
        ) {
          if (catalog.removalCommitted) {
            publishedAfterCommit.add(state);
          }
        });
        addTearDown(subscription.close);
        final updates = container.read(
          libraryUpdateControllerProvider.notifier,
        );
        final library = container.read(libraryControllerProvider.notifier);
        updates.startUpdates(["root-a", "root-b"]);
        expect(scanner.startedRootPaths, ["C:\\A", "C:\\B"]);
        final removal = library.unregisterRoot(roots.last);
        expect(catalog.unregisterCalls, ["root-c"]);
        if (duringCommittedReload) {
          catalog.commitRemoval();
          await Future<void>.delayed(Duration.zero);
          expect(
            container.read(libraryControllerProvider).isRemovalCommitted,
            isTrue,
          );
          expect(catalog.loads, 1);
        }

        catalog.publishRootUpdate();
        final scanA = scanner.scanIdForRootPath("C:\\A");
        scanner.add(
          scanA,
          const LibraryScanCompleted(
            assetCount: 1,
            issueCount: 0,
            catalogPath: "catalog.sqlite3",
            wasLimited: false,
          ),
        );
        await scanner.close(scanA);
        await Future<void>.delayed(Duration.zero);
        expect(
          container
              .read(libraryUpdateControllerProvider)
              .tasksByRootId["root-a"]
              ?.phase,
          LibraryRootUpdatePhase.refreshing,
        );
        expect(
          container.read(libraryControllerProvider).status,
          LibraryStatus.removing,
        );
        expect(catalog.loads, duringCommittedReload ? 1 : 0);
        expect(
          scanner.activeScanIds,
          contains(scanner.scanIdForRootPath("C:\\B")),
        );

        if (!duringCommittedReload) {
          catalog.commitRemoval();
          await Future<void>.delayed(Duration.zero);
        }
        catalog.completeRemovalPage();
        expect(await removal, isTrue);
        await Future<void>.delayed(Duration.zero);
        await Future<void>.delayed(Duration.zero);
        final state = container.read(libraryControllerProvider);
        expect(catalog.unregisterCalls, ["root-c"]);
        expect(catalog.loads, 2);
        expect(state.status, LibraryStatus.completed);
        expect(state.taskKind, isNull);
        expect(state.rootRemovalCompletionSequence, 1);
        expect(state.query.rootId, isNull);
        expect(state.roots.map((root) => root.id), ["root-a", "root-b"]);
        expect(state.assets.first.sourceGeneration, BigInt.two);
        expect(
          publishedAfterCommit.every(
            (state) => state.roots.every((root) => root.id != "root-c"),
          ),
          isTrue,
        );
        expect(
          container
              .read(libraryUpdateControllerProvider)
              .tasksByRootId["root-a"]
              ?.phase,
          LibraryRootUpdatePhase.completed,
        );
      },
    );
  }

  test("coordinates exclusive primary work with independent update roots", () {
    final coordinator = LibraryScanExecutionCoordinator();
    final primaryOwner = Object();

    expect(coordinator.tryAcquireUpdate("root-a"), isTrue);
    expect(coordinator.tryAcquireUpdate("root-a"), isFalse);
    expect(coordinator.tryAcquireUpdate("root-b"), isTrue);
    expect(coordinator.tryAcquirePrimary(primaryOwner), isFalse);

    coordinator.releaseUpdate("root-a");
    coordinator.releaseUpdate("root-b");
    expect(coordinator.tryAcquirePrimary(primaryOwner), isTrue);
    expect(coordinator.tryAcquirePrimary(primaryOwner), isTrue);
    expect(coordinator.tryAcquireUpdate("root-c"), isFalse);

    coordinator.releasePrimary(Object());
    expect(coordinator.hasPrimaryScan, isTrue);
    coordinator.releasePrimary(primaryOwner);
    expect(coordinator.canStartPrimary, isTrue);
  });

  test("updates different roots with bounded independent scan runs", () async {
    final scanner = _ConcurrentLibraryScanner();
    var catalogRefreshCount = 0;
    final roots = [
      _root("root-a", "C:\\A"),
      _root("root-b", "C:\\B"),
      _root("root-c", "C:\\C"),
    ];
    final container = ProviderContainer(
      overrides: [
        libraryScannerProvider.overrideWithValue(scanner),
        libraryUpdateConfiguredRootsProvider.overrideWithValue(roots),
        libraryUpdatePrimaryBusyProvider.overrideWithValue(false),
        libraryUpdateCatalogRefreshProvider.overrideWithValue(() async {
          catalogRefreshCount += 1;
        }),
      ],
    );
    addTearDown(container.dispose);
    addTearDown(scanner.dispose);
    final controller = container.read(libraryUpdateControllerProvider.notifier);

    controller.startUpdates(roots.map((root) => root.id));

    expect(scanner.startedRootPaths, ["C:\\A", "C:\\B"]);
    expect(scanner.activeScanIds, hasLength(libraryUpdateMaxConcurrentRoots));
    expect(
      container
          .read(libraryUpdateControllerProvider)
          .tasksByRootId["root-c"]
          ?.phase,
      LibraryRootUpdatePhase.queued,
    );

    final scanA = scanner.scanIdForRootPath("C:\\A");
    final scanB = scanner.scanIdForRootPath("C:\\B");
    scanner.add(
      scanA,
      const LibraryScanProgress(
        visitedEntries: 40,
        acceptedItems: 12,
        issueCount: 1,
      ),
    );
    scanner.add(
      scanB,
      const LibraryScanFailed(code: "root_b_failed", message: "B failed"),
    );
    await scanner.close(scanB);
    await Future<void>.delayed(Duration.zero);

    expect(scanner.startedRootPaths, ["C:\\A", "C:\\B", "C:\\C"]);
    final stateAfterFailure = container.read(libraryUpdateControllerProvider);
    expect(stateAfterFailure.tasksByRootId["root-a"]?.visitedEntries, 40);
    expect(
      stateAfterFailure.tasksByRootId["root-b"]?.phase,
      LibraryRootUpdatePhase.failed,
    );
    expect(
      stateAfterFailure.tasksByRootId["root-c"]?.phase,
      LibraryRootUpdatePhase.discovering,
    );

    controller.cancel("root-a");
    expect(scanner.cancelledScanIds, [scanA]);
    expect(
      container
          .read(libraryUpdateControllerProvider)
          .tasksByRootId["root-a"]
          ?.phase,
      LibraryRootUpdatePhase.cancelling,
    );
    scanner.add(
      scanA,
      const LibraryScanCancelled(acceptedItems: 12, issueCount: 1),
    );
    await scanner.close(scanA);

    final scanC = scanner.scanIdForRootPath("C:\\C");
    scanner.add(
      scanC,
      const LibraryScanCompleted(
        assetCount: 8,
        issueCount: 0,
        catalogPath: "C:\\Ame\\catalog.sqlite3",
        wasLimited: false,
      ),
    );
    await scanner.close(scanC);
    await Future<void>.delayed(Duration.zero);
    await Future<void>.delayed(Duration.zero);

    final finalState = container.read(libraryUpdateControllerProvider);
    expect(
      finalState.tasksByRootId["root-a"]?.phase,
      LibraryRootUpdatePhase.cancelled,
    );
    expect(
      finalState.tasksByRootId["root-b"]?.errorMessage,
      "root_b_failed: B failed",
    );
    expect(
      finalState.tasksByRootId["root-c"]?.phase,
      LibraryRootUpdatePhase.completed,
    );
    expect(catalogRefreshCount, 1);
  });

  test("cancels a queued root without touching an active scan", () {
    final scanner = _ConcurrentLibraryScanner();
    final roots = [
      _root("root-a", "C:\\A"),
      _root("root-b", "C:\\B"),
      _root("root-c", "C:\\C"),
    ];
    final container = ProviderContainer(
      overrides: [
        libraryScannerProvider.overrideWithValue(scanner),
        libraryUpdateConfiguredRootsProvider.overrideWithValue(roots),
        libraryUpdatePrimaryBusyProvider.overrideWithValue(false),
        libraryUpdateCatalogRefreshProvider.overrideWithValue(() async {}),
      ],
    );
    addTearDown(container.dispose);
    addTearDown(scanner.dispose);
    final controller = container.read(libraryUpdateControllerProvider.notifier);

    controller.startUpdates(roots.map((root) => root.id));
    controller.cancel("root-c");

    expect(scanner.cancelledScanIds, isEmpty);
    expect(
      container
          .read(libraryUpdateControllerProvider)
          .tasksByRootId["root-c"]
          ?.phase,
      LibraryRootUpdatePhase.cancelled,
    );
  });

  test("does not schedule unavailable roots", () {
    final scanner = _ConcurrentLibraryScanner();
    final root = _root(
      "root-offline",
      "E:\\Offline",
      availability: LibraryRootAvailability.offline,
    );
    final container = ProviderContainer(
      overrides: [
        libraryScannerProvider.overrideWithValue(scanner),
        libraryUpdateConfiguredRootsProvider.overrideWithValue([root]),
        libraryUpdatePrimaryBusyProvider.overrideWithValue(false),
        libraryUpdateCatalogRefreshProvider.overrideWithValue(() async {}),
      ],
    );
    addTearDown(container.dispose);
    addTearDown(scanner.dispose);

    container.read(libraryUpdateControllerProvider.notifier).startUpdates([
      root.id,
    ]);

    expect(scanner.startedRootPaths, isEmpty);
    expect(
      container.read(libraryUpdateControllerProvider).tasksByRootId,
      isEmpty,
    );
  });

  test("resolves requested ids through the current configured roots", () {
    final scanner = _ConcurrentLibraryScanner();
    final root = _root("root-a", "C:\\Current");
    final container = ProviderContainer(
      overrides: [
        libraryScannerProvider.overrideWithValue(scanner),
        libraryUpdateConfiguredRootsProvider.overrideWithValue([root]),
        libraryUpdatePrimaryBusyProvider.overrideWithValue(false),
        libraryUpdateCatalogRefreshProvider.overrideWithValue(() async {}),
      ],
    );
    addTearDown(container.dispose);
    addTearDown(scanner.dispose);

    container.read(libraryUpdateControllerProvider.notifier).startUpdates([
      "root-a",
      "removed-root",
      "root-a",
    ]);

    expect(scanner.startedRootPaths, ["C:\\Current"]);
    expect(container.read(libraryUpdateControllerProvider).tasksByRootId.keys, [
      "root-a",
    ]);
  });

  test("revalidates a queued root before starting its scan", () async {
    final scanner = _ConcurrentLibraryScanner();
    final roots = [
      _root("root-a", "C:\\A"),
      _root("root-b", "C:\\B"),
      _root("root-c", "C:\\C"),
    ];
    var currentRoots = roots;
    final container = ProviderContainer(
      overrides: [
        libraryScannerProvider.overrideWithValue(scanner),
        libraryUpdateConfiguredRootsProvider.overrideWith(
          (ref) => currentRoots,
        ),
        libraryUpdatePrimaryBusyProvider.overrideWithValue(false),
        libraryUpdateCatalogRefreshProvider.overrideWithValue(() async {}),
      ],
    );
    addTearDown(container.dispose);
    addTearDown(scanner.dispose);
    final controller = container.read(libraryUpdateControllerProvider.notifier);

    controller.startUpdates(roots.map((root) => root.id));
    currentRoots = roots.take(2).toList(growable: false);
    container.invalidate(libraryUpdateConfiguredRootsProvider);
    final scanB = scanner.scanIdForRootPath("C:\\B");
    scanner.add(
      scanB,
      const LibraryScanFailed(code: "root_b_failed", message: "B failed"),
    );
    await scanner.close(scanB);
    await Future<void>.delayed(Duration.zero);

    expect(scanner.startedRootPaths, ["C:\\A", "C:\\B"]);
    expect(
      container
          .read(libraryUpdateControllerProvider)
          .tasksByRootId["root-c"]
          ?.errorMessage,
      startsWith("library_update_root_changed:"),
    );
  });

  test("does not admit updates while the primary scan owns execution", () {
    final scanner = _ConcurrentLibraryScanner();
    final root = _root("root-a", "C:\\A");
    final execution = LibraryScanExecutionCoordinator();
    final primaryOwner = Object();
    expect(execution.tryAcquirePrimary(primaryOwner), isTrue);
    final container = ProviderContainer(
      overrides: [
        libraryScannerProvider.overrideWithValue(scanner),
        libraryScanExecutionCoordinatorProvider.overrideWithValue(execution),
        libraryUpdateConfiguredRootsProvider.overrideWithValue([root]),
        libraryUpdatePrimaryBusyProvider.overrideWithValue(false),
        libraryUpdateCatalogRefreshProvider.overrideWithValue(() async {}),
      ],
    );
    addTearDown(container.dispose);
    addTearDown(scanner.dispose);

    container.read(libraryUpdateControllerProvider.notifier).startUpdates([
      root.id,
    ]);

    expect(scanner.startedRootPaths, isEmpty);
    expect(
      container.read(libraryUpdateControllerProvider).tasksByRootId,
      isEmpty,
    );
    execution.releasePrimary(primaryOwner);
  });

  test("reissues an early cancellation after the native start event", () async {
    final scanner = _ConcurrentLibraryScanner(delayedRegistration: true);
    final root = _root("root-a", "C:\\A");
    final container = ProviderContainer(
      overrides: [
        libraryScannerProvider.overrideWithValue(scanner),
        libraryUpdateConfiguredRootsProvider.overrideWithValue([root]),
        libraryUpdatePrimaryBusyProvider.overrideWithValue(false),
        libraryUpdateCatalogRefreshProvider.overrideWithValue(() async {}),
      ],
    );
    addTearDown(container.dispose);
    addTearDown(scanner.dispose);
    final controller = container.read(libraryUpdateControllerProvider.notifier);

    controller.startUpdates([root.id]);
    final scanId = scanner.scanIdForRootPath(root.path);
    controller.cancel(root.id);
    expect(scanner.cancelledScanIds, isEmpty);
    expect(
      container
          .read(libraryUpdateControllerProvider)
          .tasksByRootId[root.id]
          ?.phase,
      LibraryRootUpdatePhase.cancelling,
    );

    scanner.add(
      scanId,
      LibraryScanStarted(scanId: scanId, rootPath: root.path),
    );
    scanner.add(
      scanId,
      const LibraryScanProgress(
        visitedEntries: 2,
        acceptedItems: 1,
        issueCount: 0,
      ),
    );
    await Future<void>.delayed(Duration.zero);
    expect(scanner.cancelledScanIds, [scanId]);
    expect(
      container
          .read(libraryUpdateControllerProvider)
          .tasksByRootId[root.id]
          ?.phase,
      LibraryRootUpdatePhase.cancelling,
    );

    scanner.add(
      scanId,
      const LibraryScanCancelled(acceptedItems: 1, issueCount: 0),
    );
    await scanner.close(scanId);
    expect(
      container
          .read(libraryUpdateControllerProvider)
          .tasksByRootId[root.id]
          ?.phase,
      LibraryRootUpdatePhase.cancelled,
    );
  });

  test("retains ownership after a stream error until native release", () async {
    final scanner = _ConcurrentLibraryScanner();
    final root = _root("root-a", "C:\\A");
    final container = ProviderContainer(
      overrides: [
        libraryScannerProvider.overrideWithValue(scanner),
        libraryUpdateConfiguredRootsProvider.overrideWithValue([root]),
        libraryUpdatePrimaryBusyProvider.overrideWithValue(false),
        libraryUpdateCatalogRefreshProvider.overrideWithValue(() async {}),
      ],
    );
    addTearDown(container.dispose);
    addTearDown(scanner.dispose);
    final controller = container.read(libraryUpdateControllerProvider.notifier);

    controller.startUpdates([root.id]);
    final scanId = scanner.scanIdForRootPath(root.path);
    scanner.addError(scanId, StateError("bridge stream failed"));
    await Future<void>.delayed(Duration.zero);
    controller.retry(root.id);

    expect(scanner.startedRootPaths, [root.path]);
    expect(scanner.cancelledScanIds, [scanId]);
    expect(
      container
          .read(libraryUpdateControllerProvider)
          .tasksByRootId[root.id]
          ?.phase,
      LibraryRootUpdatePhase.cancelling,
    );

    await scanner.close(scanId);
    await Future<void>.delayed(Duration.zero);
    expect(
      container
          .read(libraryUpdateControllerProvider)
          .tasksByRootId[root.id]
          ?.phase,
      LibraryRootUpdatePhase.failed,
    );
    controller.retry(root.id);
    expect(scanner.startedRootPaths, [root.path, root.path]);
  });

  test(
    "reports catalog refresh failure and retries without rescanning",
    () async {
      final scanner = _ConcurrentLibraryScanner();
      var refreshAttempts = 0;
      final root = _root("root-a", "C:\\A");
      final container = ProviderContainer(
        overrides: [
          libraryScannerProvider.overrideWithValue(scanner),
          libraryUpdateConfiguredRootsProvider.overrideWithValue([root]),
          libraryUpdatePrimaryBusyProvider.overrideWithValue(false),
          libraryUpdateCatalogRefreshProvider.overrideWithValue(() async {
            refreshAttempts += 1;
            if (refreshAttempts == 1) {
              throw StateError("query refresh failed");
            }
          }),
        ],
      );
      addTearDown(container.dispose);
      addTearDown(scanner.dispose);
      final controller = container.read(
        libraryUpdateControllerProvider.notifier,
      );

      controller.startUpdates([root.id]);
      final scanId = scanner.scanIdForRootPath("C:\\A");
      scanner.add(
        scanId,
        const LibraryScanCompleted(
          assetCount: 8,
          issueCount: 0,
          catalogPath: "C:\\Ame\\catalog.sqlite3",
          wasLimited: false,
        ),
      );
      await scanner.close(scanId);
      await Future<void>.delayed(Duration.zero);
      await Future<void>.delayed(Duration.zero);

      final failedTask = container
          .read(libraryUpdateControllerProvider)
          .tasksByRootId["root-a"];
      expect(failedTask?.phase, LibraryRootUpdatePhase.refreshFailed);
      expect(failedTask?.errorMessage, contains("刷新显示失败"));
      expect(scanner.startedRootPaths, ["C:\\A"]);

      controller.retry("root-a");
      await Future<void>.delayed(Duration.zero);
      await Future<void>.delayed(Duration.zero);

      expect(refreshAttempts, 2);
      expect(scanner.startedRootPaths, ["C:\\A"]);
      expect(
        container
            .read(libraryUpdateControllerProvider)
            .tasksByRootId["root-a"]
            ?.phase,
        LibraryRootUpdatePhase.completed,
      );
    },
  );

  test(
    "exposes failure retry only after the terminal stream releases",
    () async {
      final scanner = _ConcurrentLibraryScanner();
      final root = _root("root-a", "C:\\A");
      final container = ProviderContainer(
        overrides: [
          libraryScannerProvider.overrideWithValue(scanner),
          libraryUpdateConfiguredRootsProvider.overrideWithValue([root]),
          libraryUpdatePrimaryBusyProvider.overrideWithValue(false),
          libraryUpdateCatalogRefreshProvider.overrideWithValue(() async {}),
        ],
      );
      addTearDown(container.dispose);
      addTearDown(scanner.dispose);
      final controller = container.read(
        libraryUpdateControllerProvider.notifier,
      );

      controller.startUpdates([root.id]);
      final firstScanId = scanner.scanIdForRootPath(root.path);
      scanner.add(
        firstScanId,
        const LibraryScanFailed(code: "scan_failed", message: "failed"),
      );
      await Future<void>.delayed(Duration.zero);

      expect(
        container
            .read(libraryUpdateControllerProvider)
            .tasksByRootId[root.id]
            ?.phase,
        LibraryRootUpdatePhase.discovering,
      );
      controller.retry(root.id);
      expect(scanner.startedRootPaths, [root.path]);

      await scanner.close(firstScanId);
      await Future<void>.delayed(Duration.zero);
      expect(
        container
            .read(libraryUpdateControllerProvider)
            .tasksByRootId[root.id]
            ?.phase,
        LibraryRootUpdatePhase.failed,
      );

      controller.retry(root.id);
      expect(scanner.startedRootPaths, [root.path, root.path]);
      expect(
        container
            .read(libraryUpdateControllerProvider)
            .tasksByRootId[root.id]
            ?.phase,
        LibraryRootUpdatePhase.discovering,
      );
    },
  );

  test("quick refresh exposes completion only after scan release", () async {
    final scanner = _ConcurrentLibraryScanner();
    var refreshCount = 0;
    final root = _root("root-a", "C:\\A");
    final container = ProviderContainer(
      overrides: [
        libraryScannerProvider.overrideWithValue(scanner),
        libraryUpdateConfiguredRootsProvider.overrideWithValue([root]),
        libraryUpdatePrimaryBusyProvider.overrideWithValue(false),
        libraryUpdateCatalogRefreshProvider.overrideWithValue(() async {
          refreshCount += 1;
        }),
      ],
    );
    addTearDown(container.dispose);
    addTearDown(scanner.dispose);
    final controller = container.read(libraryUpdateControllerProvider.notifier);

    controller.startUpdates([root.id]);
    final firstScanId = scanner.scanIdForRootPath(root.path);
    scanner.add(
      firstScanId,
      const LibraryScanCompleted(
        assetCount: 8,
        issueCount: 0,
        catalogPath: "C:\\Ame\\catalog.sqlite3",
        wasLimited: false,
      ),
    );
    await Future<void>.delayed(Duration.zero);

    expect(refreshCount, 0);
    expect(
      container
          .read(libraryUpdateControllerProvider)
          .tasksByRootId[root.id]
          ?.phase,
      LibraryRootUpdatePhase.discovering,
    );

    await scanner.close(firstScanId);
    await Future<void>.delayed(Duration.zero);
    await Future<void>.delayed(Duration.zero);
    expect(refreshCount, 1);
    expect(
      container
          .read(libraryUpdateControllerProvider)
          .tasksByRootId[root.id]
          ?.phase,
      LibraryRootUpdatePhase.completed,
    );

    controller.startUpdates([root.id]);
    expect(scanner.startedRootPaths, [root.path, root.path]);
  });

  test("shutdown cancels and drains every active root update", () async {
    final scanner = _ConcurrentLibraryScanner(completeCancellation: true);
    final coordinator = LibraryScanShutdownCoordinator();
    final roots = [
      _root("root-a", "C:\\A"),
      _root("root-b", "C:\\B"),
      _root("root-c", "C:\\C"),
    ];
    final container = ProviderContainer(
      overrides: [
        libraryScannerProvider.overrideWithValue(scanner),
        libraryScanShutdownCoordinatorProvider.overrideWithValue(coordinator),
        libraryUpdateConfiguredRootsProvider.overrideWithValue(roots),
        libraryUpdatePrimaryBusyProvider.overrideWithValue(false),
        libraryUpdateCatalogRefreshProvider.overrideWithValue(() async {}),
      ],
    );
    addTearDown(container.dispose);
    addTearDown(scanner.dispose);
    final controller = container.read(libraryUpdateControllerProvider.notifier);
    controller.startUpdates(roots.map((root) => root.id));
    final activeScanIds = scanner.activeScanIds.toSet();

    await coordinator.suspend();

    expect(scanner.cancelledScanIds.toSet(), activeScanIds);
    expect(scanner.startedRootPaths, ["C:\\A", "C:\\B"]);
    expect(scanner.activeScanIds, isEmpty);
    expect(
      container
          .read(libraryUpdateControllerProvider)
          .tasks
          .map((task) => task.phase),
      everyElement(LibraryRootUpdatePhase.cancelled),
    );
  });

  test(
    "shutdown reissues cancellation after delayed native registration",
    () async {
      final scanner = _ConcurrentLibraryScanner(
        completeCancellation: true,
        delayedRegistration: true,
      );
      final coordinator = LibraryScanShutdownCoordinator();
      final root = _root("root-a", "C:\\A");
      final container = ProviderContainer(
        overrides: [
          libraryScannerProvider.overrideWithValue(scanner),
          libraryScanShutdownCoordinatorProvider.overrideWithValue(coordinator),
          libraryUpdateConfiguredRootsProvider.overrideWithValue([root]),
          libraryUpdatePrimaryBusyProvider.overrideWithValue(false),
          libraryUpdateCatalogRefreshProvider.overrideWithValue(() async {}),
        ],
      );
      addTearDown(container.dispose);
      addTearDown(scanner.dispose);
      container.read(libraryUpdateControllerProvider.notifier).startUpdates([
        root.id,
      ]);
      final scanId = scanner.scanIdForRootPath(root.path);

      final shutdown = coordinator.suspend();
      await Future<void>.delayed(Duration.zero);
      expect(scanner.cancelledScanIds, isEmpty);
      scanner.add(
        scanId,
        LibraryScanStarted(scanId: scanId, rootPath: root.path),
      );
      await shutdown;

      expect(scanner.cancelledScanIds, [scanId]);
      expect(scanner.activeScanIds, isEmpty);
    },
  );

  test("shutdown coordinator invokes every attached owner", () async {
    final coordinator = LibraryScanShutdownCoordinator();
    final completed = <String>[];
    coordinator.attach(Object(), () async => completed.add("first"));
    coordinator.attach(Object(), () async => completed.add("second"));

    await coordinator.suspend();

    expect(completed, containsAll(["first", "second"]));
  });
}

LibraryRoot _root(
  String id,
  String path, {
  LibraryRootAvailability availability = LibraryRootAvailability.available,
}) {
  return LibraryRoot(
    id: id,
    path: path,
    displayPath: path,
    activeScanId: "published-$id",
    createdUnixMs: 1,
    assetCount: 1,
    issueCount: 0,
    availability: availability,
  );
}

class _RemovingUpdateCatalog implements LibraryCatalog {
  _RemovingUpdateCatalog(this.roots);

  List<LibraryRoot> roots;
  BigInt revision = BigInt.one;
  BigInt sourceGeneration = BigInt.one;
  bool removalCommitted = false;
  int loads = 0;
  final unregisterCalls = <String>[];
  final _unregister = Completer<bool>();
  final _removalPage = Completer<LibrarySnapshot>();
  LibrarySnapshot? _capturedRemovalPage;

  LibrarySnapshot get snapshot => LibrarySnapshot(
    catalogPath: "catalog.sqlite3",
    revision: revision,
    queryId: "query-$revision",
    roots: roots,
    assets: [
      for (final root in roots)
        LibraryAsset(
          assetId: "asset-${root.id}",
          locationId: "location-${root.id}",
          rootId: root.id,
          activeScanId: "scan-${root.id}",
          sourcePath: "${root.path}\\image.png",
          displayPath: "${root.path}\\image.png",
          relativePath: "image.png",
          previewPath: "",
          fileSize: BigInt.one,
          modifiedUnixMs: 1,
          sourceRevision: null,
          sourceGeneration: root.id == "root-a" ? sourceGeneration : BigInt.one,
          width: 1,
          height: 1,
        ),
    ],
  );

  void publishRootUpdate() {
    revision += BigInt.one;
    sourceGeneration = BigInt.two;
  }

  void commitRemoval() {
    removalCommitted = true;
    roots = roots.where((root) => root.id != "root-c").toList();
    revision += BigInt.one;
    _unregister.complete(true);
  }

  void completeRemovalPage() => _removalPage.complete(_capturedRemovalPage!);

  @override
  Future<bool> unregisterRoot(String rootId) {
    unregisterCalls.add(rootId);
    return _unregister.future;
  }

  @override
  Future<LibrarySnapshot> load({
    required int maxItems,
    required LibraryGalleryQuery query,
    LibraryCatalogCursor? after,
    LibraryCatalogCursor? before,
  }) async {
    loads += 1;
    if (removalCommitted && _capturedRemovalPage == null) {
      _capturedRemovalPage = snapshot;
      return _removalPage.future;
    }
    return snapshot;
  }

  @override
  Future<LibraryTimeline> loadTimeline(LibraryGalleryQuery query) async =>
      LibraryTimeline(
        revision: revision,
        queryId: "query-$revision",
        totalItems: roots.length,
        buckets: const [],
      );

  @override
  Future<LibrarySnapshot> loadAtTime({
    required int maxItems,
    required LibraryGalleryQuery query,
    required LibraryTimeAnchor anchor,
  }) => throw UnimplementedError();
}

class _ConcurrentLibraryScanner implements LibraryScanner {
  @override
  Future<void> cancelRetainedScan(String scanId) async {
    throw StateError("This fixture has no retained cancellation command");
  }

  _ConcurrentLibraryScanner({
    this.completeCancellation = false,
    this.delayedRegistration = false,
  });

  final bool completeCancellation;
  final bool delayedRegistration;
  final Map<String, StreamController<LibraryScanUpdate>> _controllers = {};
  final Map<String, String> _rootPathsByScanId = {};
  final Set<String> _registeredScanIds = {};
  final List<String> startedRootPaths = [];
  final List<String> cancelledScanIds = [];

  Iterable<String> get activeScanIds => _controllers.keys;

  String scanIdForRootPath(String rootPath) {
    return _rootPathsByScanId.entries
        .singleWhere((entry) => entry.value == rootPath)
        .key;
  }

  void add(String scanId, LibraryScanUpdate update) {
    if (update is LibraryScanStarted) {
      _registeredScanIds.add(scanId);
    }
    _controllers[scanId]?.add(update);
  }

  void addError(String scanId, Object error) {
    _controllers[scanId]?.addError(error);
  }

  Future<void> close(String scanId) async {
    _registeredScanIds.remove(scanId);
    await _controllers.remove(scanId)?.close();
  }

  void dispose() {
    for (final controller in _controllers.values) {
      unawaited(controller.close());
    }
    _controllers.clear();
    _registeredScanIds.clear();
  }

  @override
  bool cancel(String scanId) {
    if (!_registeredScanIds.contains(scanId)) {
      return false;
    }
    cancelledScanIds.add(scanId);
    if (completeCancellation) {
      scheduleMicrotask(() {
        add(
          scanId,
          const LibraryScanCancelled(acceptedItems: 0, issueCount: 0),
        );
        unawaited(close(scanId));
      });
    }
    return true;
  }

  @override
  Future<RecoverableLibraryScan?> loadPausedScan() async => null;

  @override
  Future<RecoverableLibraryScan?> loadRecoverableScan() async => null;

  @override
  bool pause(String scanId) => false;

  @override
  Stream<LibraryScanUpdate> resume({
    required String scanId,
    required String rootPath,
    required int? itemLimit,
    required int? entryLimit,
    required int previewEdge,
  }) {
    throw UnimplementedError();
  }

  @override
  Stream<LibraryScanUpdate> scan({
    required String scanId,
    required String rootPath,
    required int? itemLimit,
    required int? entryLimit,
    required int previewEdge,
  }) {
    // Every controller is closed by the registered scanner teardown.
    // ignore: close_sinks
    final controller = StreamController<LibraryScanUpdate>();
    _controllers[scanId] = controller;
    _rootPathsByScanId[scanId] = rootPath;
    startedRootPaths.add(rootPath);
    if (!delayedRegistration) {
      _registeredScanIds.add(scanId);
    }
    return controller.stream;
  }

  @override
  bool suspend(String scanId) => _controllers.containsKey(scanId);
}
