import "dart:async";

import "package:cedarflake_ame/features/library/adapters/directory_picker.dart";
import "package:cedarflake_ame/features/library/application/library_catalog.dart";
import "package:cedarflake_ame/features/library/application/library_controller.dart";
import "package:cedarflake_ame/features/library/application/library_previewer.dart";
import "package:cedarflake_ame/features/library/application/library_scanner.dart";
import "package:cedarflake_ame/features/library/domain/gallery_layout_manifest.dart";
import "package:cedarflake_ame/features/library/domain/library_models.dart";
import "package:cedarflake_ame/features/library/domain/library_state.dart";
import "package:cedarflake_ame/features/settings/application/ame_preferences.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:flutter_test/flutter_test.dart";

void main() {
  test("publishes streamed assets only after a completed scan event", () async {
    final scanner = _FakeLibraryScanner();
    final catalog = _FakeLibraryCatalog.dynamic(
      () => _snapshot(
        roots: [
          LibraryRoot(
            id: "root-1",
            path: r"\\?\C:\Pictures",
            displayPath: "C:\\Pictures",
            activeScanId: scanner.startedScanId,
            createdUnixMs: 1,
            assetCount: 1,
            issueCount: 0,
          ),
        ],
        assets: [_asset()],
      ),
    );
    final container = ProviderContainer(
      overrides: [
        directoryPickerProvider.overrideWithValue(
          const _FakeDirectoryPicker(r"\\?\C:\Pictures"),
        ),
        libraryScannerProvider.overrideWithValue(scanner),
        libraryCatalogProvider.overrideWithValue(catalog),
      ],
    );
    addTearDown(container.dispose);
    addTearDown(scanner.dispose);

    final controller = container.read(libraryControllerProvider.notifier);
    await controller.chooseDirectoryAndScan();

    expect(
      container.read(libraryControllerProvider).status,
      LibraryStatus.scanning,
    );
    expect(scanner.startedItemLimit, isNull);
    expect(scanner.startedEntryLimit, isNull);
    final scanId = scanner.startedScanId ?? fail("scan did not start");

    scanner.add(LibraryScanStarted(scanId: scanId, rootPath: "C:\\Pictures"));
    scanner.add(
      LibraryAssetDiscovered(
        LibraryAsset(
          assetId: "asset-1",
          locationId: "location-1",
          rootId: "root-1",
          sourcePath: "C:\\Pictures\\one.png",
          displayPath: "C:\\Pictures\\one.png",
          relativePath: "one.png",
          previewPath: "C:\\AmeCache\\one.jpg",
          fileSize: BigInt.from(128),
          modifiedUnixMs: 42,
          width: 320,
          height: 240,
        ),
      ),
    );
    await _waitForLibraryState(
      container,
      (state) =>
          state.displayRootPath == r"C:\Pictures" &&
          state.stagedAssetCount == 1,
    );
    expect(
      container.read(libraryControllerProvider).rootPath,
      r"\\?\C:\Pictures",
    );
    expect(
      container.read(libraryControllerProvider).displayRootPath,
      r"C:\Pictures",
    );
    expect(container.read(libraryControllerProvider).assets, isEmpty);
    scanner.add(
      const LibraryScanCompleted(
        assetCount: 1,
        issueCount: 0,
        catalogPath: "C:\\AmeData\\ame.sqlite3",
        wasLimited: false,
      ),
    );
    await _waitForLibraryState(
      container,
      (state) =>
          state.status == LibraryStatus.completed && state.assets.length == 1,
    );

    final state = container.read(libraryControllerProvider);
    expect(state.status, LibraryStatus.completed);
    expect(state.assets.single.relativePath, "1.png");
    expect(state.catalogPath, "C:\\AmeData\\ame.sqlite3");
    expect(state.scanId, scanId);
    expect(state.visitedEntries, 1);
    expect(state.stagedAssetCount, 1);

    controller.dismissCompletedImport();
    final dismissedState = container.read(libraryControllerProvider);
    expect(dismissedState.status, LibraryStatus.completed);
    expect(dismissedState.scanId, isNull);
    expect(dismissedState.assets.single.relativePath, "1.png");
  });

  test("forwards cancellation to the active Rust scan", () async {
    final scanner = _FakeLibraryScanner();
    final container = ProviderContainer(
      overrides: [
        directoryPickerProvider.overrideWithValue(
          const _FakeDirectoryPicker("C:\\Pictures"),
        ),
        libraryScannerProvider.overrideWithValue(scanner),
        libraryCatalogProvider.overrideWithValue(
          _FakeLibraryCatalog(_snapshot()),
        ),
      ],
    );
    addTearDown(container.dispose);
    addTearDown(scanner.dispose);

    final controller = container.read(libraryControllerProvider.notifier);
    await controller.chooseDirectoryAndScan();
    controller.cancelScan();

    expect(scanner.cancelledScanId, isNotNull);
    expect(
      container.read(libraryControllerProvider).status,
      LibraryStatus.cancelling,
    );
  });

  test("keeps source changes distinct from generic failures", () async {
    final scanner = _FakeLibraryScanner();
    final container = ProviderContainer(
      overrides: [
        directoryPickerProvider.overrideWithValue(
          const _FakeDirectoryPicker("C:\\Pictures"),
        ),
        libraryScannerProvider.overrideWithValue(scanner),
        libraryCatalogProvider.overrideWithValue(
          _FakeLibraryCatalog(_snapshot()),
        ),
      ],
    );
    addTearDown(container.dispose);
    addTearDown(scanner.dispose);

    final controller = container.read(libraryControllerProvider.notifier);
    await controller.chooseDirectoryAndScan();
    scanner.add(const LibraryScanStale(acceptedItems: 1, issueCount: 1));
    await Future<void>.delayed(Duration.zero);

    expect(
      container.read(libraryControllerProvider).status,
      LibraryStatus.stale,
    );
  });

  test(
    "preserves a terminal Rust failure instead of reporting stream end",
    () async {
      final scanner = _FakeLibraryScanner();
      final container = ProviderContainer(
        overrides: [
          libraryScannerProvider.overrideWithValue(scanner),
          libraryCatalogProvider.overrideWithValue(
            _FakeLibraryCatalog(_snapshot()),
          ),
        ],
      );
      addTearDown(container.dispose);
      addTearDown(scanner.dispose);

      final controller = container.read(libraryControllerProvider.notifier);
      await controller.scanDirectory("C:\\Pictures");
      final scanId = scanner.startedScanId ?? fail("scan did not start");
      scanner.add(LibraryScanStarted(scanId: scanId, rootPath: "C:\\Pictures"));
      scanner.add(
        const LibraryScanFailed(
          code: "catalog_database_busy",
          message: "The catalog database remained busy after waiting",
        ),
      );
      await Future<void>.delayed(Duration.zero);

      final state = container.read(libraryControllerProvider);
      expect(state.status, LibraryStatus.failed);
      expect(
        state.errorMessage,
        "catalog_database_busy: The catalog database remained busy after waiting",
      );
    },
  );

  test(
    "reconciles an atomically published scan after its terminal event is lost",
    () async {
      const bucket = LibraryTimeBucket(
        monthKey: "2024-05",
        itemCount: 100,
        aspectRatioSum: 100,
      );
      final scanner = _FakeLibraryScanner();
      final initialSnapshot = _snapshot(
        roots: const [
          LibraryRoot(
            id: "root-old",
            path: "C:\\Old",
            displayPath: "C:\\Old",
            activeScanId: "scan-old",
            createdUnixMs: 1,
            assetCount: 1,
            issueCount: 0,
          ),
        ],
        assets: [_asset(suffix: "old")],
      );
      final catalog = _FakeLibraryCatalog.dynamic(() {
        final scanId = scanner.startedScanId;
        return _snapshot(
          revision: BigInt.two,
          roots: [
            LibraryRoot(
              id: "root-new",
              path: "C:\\Pictures",
              displayPath: "C:\\Pictures",
              activeScanId: scanId,
              createdUnixMs: 2,
              assetCount: 80,
              issueCount: 2,
            ),
          ],
          assets: [_asset(suffix: "published")],
        );
      });
      final container = ProviderContainer(
        overrides: [
          initialLibraryStateProvider.overrideWithValue(
            LibraryState.fromSnapshot(initialSnapshot).copyWith(
              timeline: LibraryTimeline(
                revision: initialSnapshot.revision,
                queryId: initialSnapshot.queryId,
                totalItems: 100,
                buckets: const [bucket],
              ),
            ),
          ),
          libraryScannerProvider.overrideWithValue(scanner),
          libraryCatalogProvider.overrideWithValue(catalog),
        ],
      );
      addTearDown(container.dispose);
      addTearDown(scanner.dispose);

      final controller = container.read(libraryControllerProvider.notifier);
      await controller.scanDirectory("C:\\Pictures");
      final scanId = scanner.startedScanId ?? fail("scan did not start");
      scanner.add(LibraryScanStarted(scanId: scanId, rootPath: "C:\\Pictures"));
      scanner.add(
        const LibraryScanProgress(
          visitedEntries: 100,
          acceptedItems: 80,
          issueCount: 2,
        ),
      );
      final pendingJumps = [
        for (var index = 0; index < 20; index += 1)
          controller.jumpToTime(bucket, itemOffset: index),
      ];
      await Future<void>.delayed(Duration.zero);
      await scanner.close();
      await Future<void>.delayed(Duration.zero);
      await Future<void>.delayed(Duration.zero);
      await Future<void>.delayed(Duration.zero);

      final state = container.read(libraryControllerProvider);
      expect(state.status, LibraryStatus.completed);
      expect(state.errorMessage, isNull);
      expect(state.scanId, scanner.startedScanId);
      expect(state.stagedAssetCount, 80);
      expect(state.assets.single.locationId, "location-published");
      expect(catalog.anchors, isEmpty);
      expect(await Future.wait(pendingJumps), everyElement(isFalse));
    },
  );

  test("serializes overlapping scan starts", () async {
    final scanner = _FakeLibraryScanner();
    final container = ProviderContainer(
      overrides: [
        libraryScannerProvider.overrideWithValue(scanner),
        libraryCatalogProvider.overrideWithValue(
          _FakeLibraryCatalog(_snapshot()),
        ),
      ],
    );
    addTearDown(container.dispose);
    addTearDown(scanner.dispose);

    final controller = container.read(libraryControllerProvider.notifier);
    await Future.wait([
      controller.scanDirectory("C:\\First"),
      controller.scanDirectory("C:\\Second"),
    ]);

    expect(scanner.scanCallCount, 1);
    expect(container.read(libraryControllerProvider).rootPath, "C:\\First");
  });

  test("releases scan ownership when stream creation throws", () async {
    final scanner = _FakeLibraryScanner(throwFirstScan: true);
    final container = ProviderContainer(
      overrides: [
        directoryPickerProvider.overrideWithValue(
          const _FakeDirectoryPicker("C:\\Pictures"),
        ),
        libraryScannerProvider.overrideWithValue(scanner),
        libraryCatalogProvider.overrideWithValue(
          _FakeLibraryCatalog(_snapshot()),
        ),
      ],
    );
    addTearDown(container.dispose);
    addTearDown(scanner.dispose);

    final controller = container.read(libraryControllerProvider.notifier);
    await controller.chooseDirectoryAndScan();
    expect(
      container.read(libraryControllerProvider).status,
      LibraryStatus.failed,
    );

    await controller.retry();

    expect(scanner.scanCallCount, 2);
    expect(
      container.read(libraryControllerProvider).status,
      LibraryStatus.scanning,
    );
  });

  test(
    "waits for a paused stream to close before resuming the same scan",
    () async {
      final scanner = _DelayedDoneLibraryScanner();
      final container = ProviderContainer(
        overrides: [
          libraryScannerProvider.overrideWithValue(scanner),
          libraryCatalogProvider.overrideWithValue(
            _FakeLibraryCatalog(_snapshot()),
          ),
        ],
      );
      addTearDown(container.dispose);
      addTearDown(scanner.dispose);

      final controller = container.read(libraryControllerProvider.notifier);
      await controller.scanDirectory("C:\\Pictures");
      final scanId = scanner.startedScanIds.single;
      scanner.add(
        0,
        LibraryScanStarted(scanId: scanId, rootPath: "C:\\Pictures"),
      );
      scanner.add(
        0,
        const LibraryScanPaused(
          visitedEntries: 128,
          acceptedItems: 80,
          issueCount: 1,
        ),
      );
      await Future<void>.delayed(Duration.zero);

      final resume = controller.resumePausedScan();
      await Future<void>.delayed(Duration.zero);
      expect(scanner.startedScanIds, [scanId]);

      await scanner.close(0);
      await resume;

      expect(scanner.startedScanIds, [scanId, scanId]);
      expect(
        container.read(libraryControllerProvider).status,
        LibraryStatus.scanning,
      );
    },
  );

  test("automatically resumes a persisted interrupted scan", () async {
    final scanner = _FakeLibraryScanner(
      recoverableScan: const RecoverableLibraryScan(
        scanId: "scan-recover",
        rootPath: r"\\?\C:\Pictures",
        displayRootPath: "C:\\Pictures",
        itemLimit: 500,
        entryLimit: 2000,
        previewEdge: 512,
        visitedEntries: 128,
        acceptedItems: 40,
        issueCount: 3,
      ),
    );
    final container = ProviderContainer(
      overrides: [
        libraryScannerProvider.overrideWithValue(scanner),
        libraryCatalogProvider.overrideWithValue(
          _FakeLibraryCatalog(_snapshot()),
        ),
      ],
    );
    addTearDown(container.dispose);
    addTearDown(scanner.dispose);

    container.read(libraryControllerProvider);
    await Future<void>.delayed(Duration.zero);
    await Future<void>.delayed(Duration.zero);

    final state = container.read(libraryControllerProvider);
    expect(state.status, LibraryStatus.scanning);
    expect(state.isResumingScan, isTrue);
    expect(state.scanId, "scan-recover");
    expect(state.visitedEntries, 128);
    expect(state.stagedAssetCount, 40);
    expect(state.issueCount, 3);
    expect(scanner.startedScanId, "scan-recover");
  });

  test("restores a paused scan without starting it until resume", () async {
    final scanner = _FakeLibraryScanner(
      pausedScan: const RecoverableLibraryScan(
        scanId: "scan-paused",
        rootPath: r"\\?\C:\Pictures",
        displayRootPath: "C:\\Pictures",
        itemLimit: 500,
        entryLimit: 2000,
        previewEdge: 512,
        visitedEntries: 64,
        acceptedItems: 20,
        issueCount: 2,
      ),
    );
    final container = ProviderContainer(
      overrides: [
        libraryScannerProvider.overrideWithValue(scanner),
        libraryCatalogProvider.overrideWithValue(
          _FakeLibraryCatalog(_snapshot()),
        ),
      ],
    );
    addTearDown(container.dispose);
    addTearDown(scanner.dispose);

    final controller = container.read(libraryControllerProvider.notifier);
    await Future<void>.delayed(Duration.zero);
    await Future<void>.delayed(Duration.zero);

    var state = container.read(libraryControllerProvider);
    expect(state.status, LibraryStatus.paused);
    expect(state.visitedEntries, 64);
    expect(scanner.startedScanId, isNull);

    await controller.resumePausedScan();
    state = container.read(libraryControllerProvider);
    expect(state.status, LibraryStatus.scanning);
    expect(state.isResumingScan, isTrue);
    expect(scanner.startedScanId, "scan-paused");
  });

  test("forwards pause and keeps the staged scan private", () async {
    final scanner = _FakeLibraryScanner();
    final container = ProviderContainer(
      overrides: [
        directoryPickerProvider.overrideWithValue(
          const _FakeDirectoryPicker("C:\\Pictures"),
        ),
        libraryScannerProvider.overrideWithValue(scanner),
        libraryCatalogProvider.overrideWithValue(
          _FakeLibraryCatalog(_snapshot()),
        ),
      ],
    );
    addTearDown(container.dispose);
    addTearDown(scanner.dispose);

    final controller = container.read(libraryControllerProvider.notifier);
    await controller.chooseDirectoryAndScan();
    final scanId = container.read(libraryControllerProvider).scanId;
    controller.pauseScan();

    expect(scanner.pausedScanId, scanId);
    expect(
      container.read(libraryControllerProvider).status,
      LibraryStatus.pausing,
    );

    scanner.add(
      const LibraryScanPaused(
        visitedEntries: 10,
        acceptedItems: 4,
        issueCount: 1,
      ),
    );
    await Future<void>.delayed(Duration.zero);

    final state = container.read(libraryControllerProvider);
    expect(state.status, LibraryStatus.paused);
    expect(state.assets, isEmpty);
    expect(state.stagedAssetCount, 4);
  });

  test("merges keyset pages without duplicating a location", () async {
    final cursor = _cursor();
    const roots = [
      LibraryRoot(
        id: "root-1",
        path: "C:\\Pictures",
        displayPath: "C:\\Pictures",
        activeScanId: "scan-test",
        createdUnixMs: 1,
        assetCount: 2,
        issueCount: 0,
      ),
    ];
    final initialSnapshot = _snapshot(
      roots: roots,
      assets: [_asset()],
      nextCursor: cursor,
    );
    final catalog = _FakeLibraryCatalog.sequence([
      _snapshot(
        roots: roots,
        assets: [
          _asset(previewPath: "C:\\AmeCache\\one-updated.jpg"),
          _asset(suffix: "2"),
        ],
      ),
    ], initialRevision: initialSnapshot.revision);
    final container = ProviderContainer(
      overrides: [
        initialLibraryStateProvider.overrideWithValue(
          LibraryState.fromSnapshot(initialSnapshot),
        ),
        libraryCatalogProvider.overrideWithValue(catalog),
      ],
    );
    addTearDown(container.dispose);

    await container.read(libraryControllerProvider.notifier).loadNextPage();

    final state = container.read(libraryControllerProvider);
    expect(state.assets, hasLength(2));
    expect(state.assets.first.previewPath, endsWith("one-updated.jpg"));
    expect(state.assets.last.locationId, "location-2");
    expect(state.nextCursor, isNull);
    expect(catalog.afters.single, same(cursor));
  });

  test(
    "refreshes the previous cursor when a page only updates assets",
    () async {
      final cursor = _cursor(suffix: "current");
      final refreshedCursor = _cursor(suffix: "refreshed");
      final initialSnapshot = _snapshot(
        assets: [_asset()],
        previousCursor: cursor,
      );
      final catalog = _FakeLibraryCatalog.sequence([
        _snapshot(
          assets: [_asset(previewPath: "C:\\AmeCache\\one-updated.jpg")],
          previousCursor: refreshedCursor,
        ),
      ], initialRevision: initialSnapshot.revision);
      final container = ProviderContainer(
        overrides: [
          initialLibraryStateProvider.overrideWithValue(
            LibraryState.fromSnapshot(initialSnapshot),
          ),
          libraryCatalogProvider.overrideWithValue(catalog),
        ],
      );
      addTearDown(container.dispose);

      final didLoad = await container
          .read(libraryControllerProvider.notifier)
          .loadPreviousPage();

      final state = container.read(libraryControllerProvider);
      expect(didLoad, isTrue);
      expect(state.assets, hasLength(1));
      expect(state.assets.single.previewPath, endsWith("one-updated.jpg"));
      expect(state.previousCursor, same(refreshedCursor));
      expect(catalog.befores.single, same(cursor));
    },
  );

  test(
    "soft detail budget trims distant pages and preserves nearby reversal",
    () async {
      List<LibraryAsset> pageAssets(int page) {
        return [
          for (var index = 0; index < libraryCatalogWindow; index++)
            _asset(suffix: "${page * libraryCatalogWindow + index}"),
        ];
      }

      LibraryCatalogCursor boundary(int page) => _cursor(suffix: "page-$page");
      final initialSnapshot = _snapshot(
        assets: pageAssets(0),
        nextCursor: boundary(1),
      );
      final responses = <LibrarySnapshot>[
        for (var page = 1; page <= 10; page++)
          _snapshot(
            assets: pageAssets(page),
            previousCursor: boundary(page - 1),
            nextCursor: boundary(page + 1),
          ),
        _snapshot(
          assets: pageAssets(3),
          previousCursor: boundary(2),
          nextCursor: boundary(4),
        ),
      ];
      final catalog = _FakeLibraryCatalog.sequence(
        responses,
        initialRevision: initialSnapshot.revision,
      );
      final initialState = LibraryState.fromSnapshot(initialSnapshot).copyWith(
        timeline: LibraryTimeline(
          revision: initialSnapshot.revision,
          queryId: initialSnapshot.queryId,
          totalItems: 79_030,
          buckets: const [],
        ),
      );
      final container = ProviderContainer(
        overrides: [
          initialLibraryStateProvider.overrideWithValue(initialState),
          libraryCatalogProvider.overrideWithValue(catalog),
        ],
      );
      addTearDown(container.dispose);
      final controller = container.read(libraryControllerProvider.notifier);

      for (var page = 1; page <= 10; page++) {
        await controller.loadNextPage();
      }

      final trimmed = container.read(libraryControllerProvider);
      expect(trimmed.assets, hasLength(3500));
      expect(trimmed.windowStartItemOffset, 2000);
      expect(trimmed.assets.first.locationId, "location-2000");
      expect(trimmed.assets.last.locationId, "location-5499");
      expect(trimmed.previousCursor?.locationId, "location-page-3");

      final loadedPrevious = await controller.loadPreviousPage();
      final reversed = container.read(libraryControllerProvider);

      expect(loadedPrevious, isTrue);
      expect(reversed.assets, hasLength(4000));
      expect(reversed.windowStartItemOffset, 1500);
      expect(reversed.assets.first.locationId, "location-1500");
      expect(reversed.assets.last.locationId, "location-5499");
      expect(catalog.afters.whereType<LibraryCatalogCursor>(), hasLength(10));
      expect(catalog.befores.whereType<LibraryCatalogCursor>(), hasLength(1));
    },
  );

  test("loads both sides of a bounded window after a timeline jump", () async {
    const bucket = LibraryTimeBucket(
      monthKey: "2024-05",
      itemCount: 18,
      aspectRatioSum: 23.5,
    );
    final initialSnapshot = _snapshot(
      roots: const [
        LibraryRoot(
          id: "root-1",
          path: "C:\\Pictures",
          displayPath: "C:\\Pictures",
          activeScanId: "scan-1",
          createdUnixMs: 1,
          assetCount: 19,
          issueCount: 0,
        ),
      ],
      assets: [_asset(suffix: "newest")],
    );
    final previousCursor = _cursor(suffix: "anchor-boundary");
    final nextCursor = _cursor(suffix: "next-boundary");
    final anchoredSnapshot = _snapshot(
      roots: initialSnapshot.roots,
      assets: [_asset(suffix: "anchor")],
      previousCursor: previousCursor,
      nextCursor: nextCursor,
    );
    final catalog = _FakeLibraryCatalog.sequence([
      anchoredSnapshot,
      _snapshot(
        roots: initialSnapshot.roots,
        assets: [_asset(suffix: "newer")],
      ),
    ], initialRevision: initialSnapshot.revision);
    final initialState = LibraryState.fromSnapshot(initialSnapshot).copyWith(
      timeline: LibraryTimeline(
        revision: initialSnapshot.revision,
        queryId: initialSnapshot.queryId,
        totalItems: 19,
        buckets: const [bucket],
      ),
    );
    final container = ProviderContainer(
      overrides: [
        initialLibraryStateProvider.overrideWithValue(initialState),
        libraryCatalogProvider.overrideWithValue(catalog),
      ],
    );
    addTearDown(container.dispose);

    final didJump = await container
        .read(libraryControllerProvider.notifier)
        .jumpToTime(bucket, itemOffset: 9);

    final state = container.read(libraryControllerProvider);
    expect(didJump, isTrue);
    expect(state.assets.single.locationId, "location-anchor");
    expect(state.previousCursor, same(previousCursor));
    expect(state.nextCursor, isNotNull);
    expect(state.activeTimeAnchor?.monthKey, "2024-05");
    expect(state.activeTimeAnchor?.itemOffset, 9);
    expect(state.windowStartItemOffset, 9);
    expect(catalog.anchors, hasLength(1));
    expect(catalog.anchors.single.revision, initialSnapshot.revision);
    expect(catalog.anchors.single.itemOffset, 9);
    expect(catalog.timeWindowSizes.single, libraryTimelineWindow);

    final loadedPrevious = await container
        .read(libraryControllerProvider.notifier)
        .loadPreviousPage();
    final expandedState = container.read(libraryControllerProvider);

    expect(loadedPrevious, isTrue);
    expect(expandedState.assets.map((asset) => asset.locationId), [
      "location-newer",
      "location-anchor",
    ]);
    expect(expandedState.previousCursor, isNull);
    expect(expandedState.nextCursor, same(nextCursor));
    expect(expandedState.windowStartItemOffset, 8);
    expect(catalog.befores.single, same(previousCursor));
  });

  test(
    "does not schedule catalog work while the visible rows are loaded",
    () async {
      final initialSnapshot = _snapshot(
        assets: [
          _asset(suffix: "visible-10"),
          _asset(suffix: "visible-11"),
          _asset(suffix: "visible-12"),
        ],
      );
      final catalog = _FakeLibraryCatalog.sequence(
        const [],
        initialRevision: initialSnapshot.revision,
      );
      final initialState = LibraryState.fromSnapshot(initialSnapshot).copyWith(
        windowStartItemOffset: 10,
        timeline: LibraryTimeline(
          revision: initialSnapshot.revision,
          queryId: initialSnapshot.queryId,
          totalItems: 100,
          buckets: const [
            LibraryTimeBucket(
              monthKey: "2024-05",
              itemCount: 100,
              aspectRatioSum: 100,
            ),
          ],
        ),
      );
      final container = ProviderContainer(
        overrides: [
          initialLibraryStateProvider.overrideWithValue(initialState),
          libraryCatalogProvider.overrideWithValue(catalog),
        ],
      );
      addTearDown(container.dispose);

      final controller = container.read(libraryControllerProvider.notifier);
      for (var offset = 10; offset < 13; offset++) {
        controller.ensureVisibleRange(
          startItemOffset: offset,
          endItemOffsetExclusive: offset + 1,
        );
      }
      await Future<void>.delayed(Duration.zero);

      expect(catalog.anchors, isEmpty);
      expect(catalog.afters, isEmpty);
      expect(catalog.befores, isEmpty);
    },
  );

  test(
    "fills an overlapping visible range on both window boundaries",
    () async {
      final previousCursor = _cursor(suffix: "visible-previous");
      final nextCursor = _cursor(suffix: "visible-next");
      final initialSnapshot = _snapshot(
        assets: [
          _asset(suffix: "visible-10"),
          _asset(suffix: "visible-11"),
        ],
        previousCursor: previousCursor,
        nextCursor: nextCursor,
      );
      final catalog = _FakeLibraryCatalog.sequence([
        _snapshot(assets: [_asset(suffix: "visible-9")]),
        _snapshot(assets: [_asset(suffix: "visible-12")]),
      ], initialRevision: initialSnapshot.revision);
      final initialState = LibraryState.fromSnapshot(initialSnapshot).copyWith(
        windowStartItemOffset: 10,
        timeline: LibraryTimeline(
          revision: initialSnapshot.revision,
          queryId: initialSnapshot.queryId,
          totalItems: 100,
          buckets: const [
            LibraryTimeBucket(
              monthKey: "2024-05",
              itemCount: 100,
              aspectRatioSum: 100,
            ),
          ],
        ),
      );
      final container = ProviderContainer(
        overrides: [
          initialLibraryStateProvider.overrideWithValue(initialState),
          libraryCatalogProvider.overrideWithValue(catalog),
        ],
      );
      addTearDown(container.dispose);

      container
          .read(libraryControllerProvider.notifier)
          .ensureVisibleRange(startItemOffset: 9, endItemOffsetExclusive: 13);
      await Future<void>.delayed(Duration.zero);
      await Future<void>.delayed(Duration.zero);

      final state = container.read(libraryControllerProvider);
      expect(state.windowStartItemOffset, 9);
      expect(state.assets.map((asset) => asset.locationId), [
        "location-visible-9",
        "location-visible-10",
        "location-visible-11",
        "location-visible-12",
      ]);
      expect(catalog.befores, [same(previousCursor), isNull]);
      expect(catalog.afters, [isNull, same(nextCursor)]);
    },
  );

  test("loads a detail window for a disjoint visible range", () async {
    const olderBucket = LibraryTimeBucket(
      monthKey: "2024-04",
      itemCount: 30,
      aspectRatioSum: 30,
    );
    const newerBucket = LibraryTimeBucket(
      monthKey: "2024-05",
      itemCount: 70,
      aspectRatioSum: 70,
    );
    final initialSnapshot = _snapshot(
      assets: [_asset(suffix: "visible-initial")],
    );
    final catalog = _FakeLibraryCatalog.sequence([
      _snapshot(assets: [_asset(suffix: "visible-target")]),
    ], initialRevision: initialSnapshot.revision);
    final initialState = LibraryState.fromSnapshot(initialSnapshot).copyWith(
      timeline: LibraryTimeline(
        revision: initialSnapshot.revision,
        queryId: initialSnapshot.queryId,
        totalItems: 100,
        buckets: const [olderBucket, newerBucket],
      ),
    );
    final container = ProviderContainer(
      overrides: [
        initialLibraryStateProvider.overrideWithValue(initialState),
        libraryCatalogProvider.overrideWithValue(catalog),
      ],
    );
    addTearDown(container.dispose);

    container
        .read(libraryControllerProvider.notifier)
        .ensureVisibleRange(startItemOffset: 70, endItemOffsetExclusive: 75);
    await Future<void>.delayed(Duration.zero);
    await Future<void>.delayed(Duration.zero);
    await Future<void>.delayed(Duration.zero);

    final state = container.read(libraryControllerProvider);
    expect(catalog.anchors, hasLength(1));
    expect(catalog.anchors.single.monthKey, "2024-05");
    expect(catalog.anchors.single.itemOffset, 40);
    expect(state.windowStartItemOffset, 70);
    expect(state.assets.single.locationId, "location-visible-target");
  });

  test("runs only the latest time target after an active request", () async {
    const bucket = LibraryTimeBucket(
      monthKey: "2024-05",
      itemCount: 100,
      aspectRatioSum: 100,
    );
    final initialSnapshot = _snapshot(assets: [_asset(suffix: "initial")]);
    final firstResponse = Completer<LibrarySnapshot>();
    final latestResponse = Completer<LibrarySnapshot>();
    final catalog = _FakeLibraryCatalog.sequence([
      firstResponse.future,
      latestResponse.future,
    ], initialRevision: initialSnapshot.revision);
    final container = ProviderContainer(
      overrides: [
        initialLibraryStateProvider.overrideWithValue(
          LibraryState.fromSnapshot(initialSnapshot).copyWith(
            timeline: LibraryTimeline(
              revision: initialSnapshot.revision,
              queryId: initialSnapshot.queryId,
              totalItems: 100,
              buckets: const [bucket],
            ),
          ),
        ),
        libraryCatalogProvider.overrideWithValue(catalog),
      ],
    );
    addTearDown(container.dispose);

    final controller = container.read(libraryControllerProvider.notifier);
    final firstJump = controller.jumpToTime(bucket, itemOffset: 10);
    await Future<void>.delayed(Duration.zero);
    expect(catalog.anchors.map((anchor) => anchor.itemOffset), [10]);

    final latestJump = controller.jumpToTime(bucket, itemOffset: 80);
    firstResponse.complete(_snapshot(assets: [_asset(suffix: "obsolete")]));

    expect(await firstJump, isFalse);
    await Future<void>.delayed(Duration.zero);
    final stateAfterObsoleteResponse = container.read(
      libraryControllerProvider,
    );
    expect(catalog.anchors.map((anchor) => anchor.itemOffset), [10, 80]);
    expect(stateAfterObsoleteResponse.windowStartItemOffset, 0);
    expect(
      stateAfterObsoleteResponse.assets.single.locationId,
      "location-initial",
    );
    expect(stateAfterObsoleteResponse.isLoadingTimeAnchor, isTrue);

    latestResponse.complete(_snapshot(assets: [_asset(suffix: "latest")]));
    expect(await latestJump, isTrue);
    final state = container.read(libraryControllerProvider);
    expect(catalog.anchors.map((anchor) => anchor.itemOffset), [10, 80]);
    expect(state.activeTimeAnchor?.itemOffset, 80);
    expect(state.windowStartItemOffset, 80);
    expect(state.assets.single.locationId, "location-latest");
  });

  test("shares an active time request for the same global target", () async {
    const bucket = LibraryTimeBucket(
      monthKey: "2024-05",
      itemCount: 100,
      aspectRatioSum: 100,
    );
    final initialSnapshot = _snapshot(assets: [_asset(suffix: "initial")]);
    final response = Completer<LibrarySnapshot>();
    final catalog = _FakeLibraryCatalog.sequence([
      response.future,
    ], initialRevision: initialSnapshot.revision);
    final container = ProviderContainer(
      overrides: [
        initialLibraryStateProvider.overrideWithValue(
          LibraryState.fromSnapshot(initialSnapshot).copyWith(
            timeline: LibraryTimeline(
              revision: initialSnapshot.revision,
              queryId: initialSnapshot.queryId,
              totalItems: 100,
              buckets: const [bucket],
            ),
          ),
        ),
        libraryCatalogProvider.overrideWithValue(catalog),
      ],
    );
    addTearDown(container.dispose);

    final controller = container.read(libraryControllerProvider.notifier);
    final firstJump = controller.jumpToTime(bucket, itemOffset: 20);
    await Future<void>.delayed(Duration.zero);
    final duplicateJump = controller.jumpToTime(bucket, itemOffset: 20);

    expect(catalog.anchors.map((anchor) => anchor.itemOffset), [20]);
    response.complete(_snapshot(assets: [_asset(suffix: "target")]));
    expect(await firstJump, isTrue);
    expect(await duplicateJump, isTrue);
    expect(catalog.anchors.map((anchor) => anchor.itemOffset), [20]);
  });

  test(
    "an active disjoint range cannot publish after returning to loaded rows",
    () async {
      const bucket = LibraryTimeBucket(
        monthKey: "2024-05",
        itemCount: 100,
        aspectRatioSum: 100,
      );
      final initialSnapshot = _snapshot(assets: [_asset(suffix: "initial")]);
      final disjointResponse = Completer<LibrarySnapshot>();
      final catalog = _FakeLibraryCatalog.sequence([
        disjointResponse.future,
      ], initialRevision: initialSnapshot.revision);
      final container = ProviderContainer(
        overrides: [
          initialLibraryStateProvider.overrideWithValue(
            LibraryState.fromSnapshot(initialSnapshot).copyWith(
              timeline: LibraryTimeline(
                revision: initialSnapshot.revision,
                queryId: initialSnapshot.queryId,
                totalItems: 100,
                buckets: const [bucket],
              ),
            ),
          ),
          libraryCatalogProvider.overrideWithValue(catalog),
        ],
      );
      addTearDown(container.dispose);

      final controller = container.read(libraryControllerProvider.notifier);
      controller.ensureVisibleRange(
        startItemOffset: 70,
        endItemOffsetExclusive: 75,
      );
      await Future<void>.delayed(Duration.zero);
      await Future<void>.delayed(Duration.zero);
      expect(catalog.anchors.map((anchor) => anchor.itemOffset), [70]);

      controller.ensureVisibleRange(
        startItemOffset: 0,
        endItemOffsetExclusive: 1,
      );
      disjointResponse.complete(
        _snapshot(assets: [_asset(suffix: "obsolete")]),
      );
      await Future<void>.delayed(Duration.zero);
      await Future<void>.delayed(Duration.zero);

      final state = container.read(libraryControllerProvider);
      expect(state.windowStartItemOffset, 0);
      expect(state.assets.single.locationId, "location-initial");
      expect(state.isLoadingTimeAnchor, isFalse);
    },
  );

  test("sequence invalidation releases time loading and its future", () async {
    const bucket = LibraryTimeBucket(
      monthKey: "2024-05",
      itemCount: 100,
      aspectRatioSum: 100,
    );
    final initialSnapshot = _snapshot(assets: [_asset(suffix: "initial")]);
    final timeResponse = Completer<LibrarySnapshot>();
    final scanner = _FakeLibraryScanner();
    final catalog = _FakeLibraryCatalog.sequence([
      timeResponse.future,
    ], initialRevision: initialSnapshot.revision);
    final container = ProviderContainer(
      overrides: [
        initialLibraryStateProvider.overrideWithValue(
          LibraryState.fromSnapshot(initialSnapshot).copyWith(
            timeline: LibraryTimeline(
              revision: initialSnapshot.revision,
              queryId: initialSnapshot.queryId,
              totalItems: 100,
              buckets: const [bucket],
            ),
          ),
        ),
        libraryCatalogProvider.overrideWithValue(catalog),
        libraryScannerProvider.overrideWithValue(scanner),
      ],
    );
    addTearDown(container.dispose);
    addTearDown(scanner.dispose);

    final controller = container.read(libraryControllerProvider.notifier);
    final timeJump = controller.jumpToTime(bucket, itemOffset: 60);
    await Future<void>.delayed(Duration.zero);
    expect(
      container.read(libraryControllerProvider).isLoadingTimeAnchor,
      isTrue,
    );

    await controller.scanDirectory("C:\\Replacement");
    timeResponse.complete(_snapshot(assets: [_asset(suffix: "obsolete")]));

    expect(await timeJump.timeout(const Duration(seconds: 1)), isFalse);
    final state = container.read(libraryControllerProvider);
    expect(state.status, LibraryStatus.scanning);
    expect(state.isLoadingTimeAnchor, isFalse);
    expect(state.assets.single.locationId, "location-initial");
    expect(state.windowStartItemOffset, 0);
  });

  test("disposal settles active and pending time navigation futures", () async {
    const bucket = LibraryTimeBucket(
      monthKey: "2024-05",
      itemCount: 100,
      aspectRatioSum: 100,
    );
    final initialSnapshot = _snapshot(assets: [_asset(suffix: "initial")]);
    final activeResponse = Completer<LibrarySnapshot>();
    final catalog = _FakeLibraryCatalog.sequence([
      activeResponse.future,
    ], initialRevision: initialSnapshot.revision);
    final container = ProviderContainer(
      overrides: [
        initialLibraryStateProvider.overrideWithValue(
          LibraryState.fromSnapshot(initialSnapshot).copyWith(
            timeline: LibraryTimeline(
              revision: initialSnapshot.revision,
              queryId: initialSnapshot.queryId,
              totalItems: 100,
              buckets: const [bucket],
            ),
          ),
        ),
        libraryCatalogProvider.overrideWithValue(catalog),
      ],
    );

    final controller = container.read(libraryControllerProvider.notifier);
    final activeJump = controller.jumpToTime(bucket, itemOffset: 10);
    await Future<void>.delayed(Duration.zero);
    final pendingJump = controller.jumpToTime(bucket, itemOffset: 80);

    container.dispose();

    expect(await activeJump.timeout(const Duration(seconds: 1)), isFalse);
    expect(await pendingJump.timeout(const Duration(seconds: 1)), isFalse);
  });

  test("retains a time target while adjacent pagination is busy", () async {
    const bucket = LibraryTimeBucket(
      monthKey: "2024-05",
      itemCount: 100,
      aspectRatioSum: 100,
    );
    final nextCursor = _cursor(suffix: "next");
    final initialSnapshot = _snapshot(
      assets: [_asset(suffix: "initial")],
      nextCursor: nextCursor,
    );
    final pageResponse = Completer<LibrarySnapshot>();
    final catalog = _FakeLibraryCatalog.sequence([
      pageResponse.future,
      _snapshot(assets: [_asset(suffix: "target")]),
    ], initialRevision: initialSnapshot.revision);
    final container = ProviderContainer(
      overrides: [
        initialLibraryStateProvider.overrideWithValue(
          LibraryState.fromSnapshot(initialSnapshot).copyWith(
            timeline: LibraryTimeline(
              revision: initialSnapshot.revision,
              queryId: initialSnapshot.queryId,
              totalItems: 100,
              buckets: const [bucket],
            ),
          ),
        ),
        libraryCatalogProvider.overrideWithValue(catalog),
      ],
    );
    addTearDown(container.dispose);

    final controller = container.read(libraryControllerProvider.notifier);
    final pageLoad = controller.loadNextPage();
    final targetLoad = controller.jumpToTime(bucket, itemOffset: 70);
    await Future<void>.delayed(Duration.zero);
    expect(container.read(libraryControllerProvider).isLoadingPage, isTrue);
    expect(catalog.anchors, isEmpty);

    pageResponse.complete(_snapshot(assets: [_asset(suffix: "page")]));
    await pageLoad;
    await Future<void>.delayed(const Duration(milliseconds: 150));

    expect(await targetLoad, isTrue);
    final state = container.read(libraryControllerProvider);
    expect(catalog.anchors.single.itemOffset, 70);
    expect(state.windowStartItemOffset, 70);
    expect(state.assets.single.locationId, "location-target");
  });

  test("unregisters one root and reloads the remaining catalog", () async {
    const removedRoot = LibraryRoot(
      id: "root-remove",
      path: "C:\\Remove",
      displayPath: "C:\\Remove",
      activeScanId: "scan-remove",
      createdUnixMs: 1,
      assetCount: 1,
      issueCount: 0,
    );
    const keptRoot = LibraryRoot(
      id: "root-keep",
      path: "C:\\Keep",
      displayPath: "C:\\Keep",
      activeScanId: "scan-keep",
      createdUnixMs: 2,
      assetCount: 1,
      issueCount: 0,
    );
    final initialSnapshot = _snapshot(
      roots: const [removedRoot, keptRoot],
      assets: [_asset()],
    );
    final refreshedSnapshot = _snapshot(
      revision: BigInt.two,
      roots: const [keptRoot],
      assets: [_asset(suffix: "kept")],
    );
    final catalog = _FakeLibraryCatalog.sequence([
      refreshedSnapshot,
    ], initialRevision: initialSnapshot.revision);
    final container = ProviderContainer(
      overrides: [
        initialLibraryStateProvider.overrideWithValue(
          LibraryState.fromSnapshot(initialSnapshot),
        ),
        libraryCatalogProvider.overrideWithValue(catalog),
      ],
    );
    addTearDown(container.dispose);

    final didRemove = await container
        .read(libraryControllerProvider.notifier)
        .unregisterRoot(removedRoot);

    final state = container.read(libraryControllerProvider);
    expect(didRemove, isTrue);
    expect(catalog.unregisteredRootIds, ["root-remove"]);
    expect(state.roots, const [keptRoot]);
    expect(state.assets.single.locationId, "location-kept");
    expect(state.catalogRevision, BigInt.two);
  });

  test("refreshes the first page when a keyset cursor becomes stale", () async {
    final cursor = _cursor();
    final initialSnapshot = _snapshot(
      roots: const [
        LibraryRoot(
          id: "root-1",
          path: "C:\\Pictures",
          displayPath: "C:\\Pictures",
          activeScanId: "scan-1",
          createdUnixMs: 1,
          assetCount: 2,
          issueCount: 0,
        ),
      ],
      assets: [_asset()],
      nextCursor: cursor,
    );
    final refreshedSnapshot = _snapshot(
      revision: BigInt.two,
      roots: const [
        LibraryRoot(
          id: "root-1",
          path: "C:\\Pictures",
          displayPath: "C:\\Pictures",
          activeScanId: "scan-2",
          createdUnixMs: 1,
          assetCount: 1,
          issueCount: 0,
        ),
      ],
      assets: [_asset(suffix: "fresh")],
    );
    final catalog = _FakeLibraryCatalog.sequence([
      const LibraryCatalogFailure(
        code: "catalog_cursor_stale",
        message: "catalog changed",
      ),
      refreshedSnapshot,
    ], initialRevision: initialSnapshot.revision);
    final container = ProviderContainer(
      overrides: [
        initialLibraryStateProvider.overrideWithValue(
          LibraryState.fromSnapshot(initialSnapshot),
        ),
        libraryCatalogProvider.overrideWithValue(catalog),
      ],
    );
    addTearDown(container.dispose);

    await container.read(libraryControllerProvider.notifier).loadNextPage();

    final state = container.read(libraryControllerProvider);
    expect(state.catalogRevision, BigInt.two);
    expect(state.assets.single.locationId, "location-fresh");
    expect(state.pageErrorMessage, isNull);
    expect(catalog.afters, [same(cursor), isNull]);
  });

  test("bounds preview work and starts the next visible request", () async {
    final scanner = _FakeLibraryScanner();
    final previewer = _FakeLibraryPreviewer();
    final initialSnapshot = _snapshot(
      assets: [_pendingAsset("1"), _pendingAsset("2"), _pendingAsset("3")],
    );
    final container = ProviderContainer(
      overrides: [
        initialLibraryStateProvider.overrideWithValue(
          LibraryState.fromSnapshot(initialSnapshot),
        ),
        libraryScannerProvider.overrideWithValue(scanner),
        libraryCatalogProvider.overrideWithValue(
          _FakeLibraryCatalog(initialSnapshot),
        ),
        libraryPreviewerProvider.overrideWithValue(previewer),
      ],
    );
    addTearDown(container.dispose);
    addTearDown(scanner.dispose);

    final controller = container.read(libraryControllerProvider.notifier);
    final assetList = container.read(libraryControllerProvider).assets;
    for (final asset in initialSnapshot.assets) {
      controller.requestPreview(asset);
    }

    expect(previewer.requests, ["location-1", "location-2"]);

    previewer.succeed("location-1", _asset(suffix: "1"));
    await Future<void>.delayed(Duration.zero);
    await Future<void>.delayed(Duration.zero);

    expect(previewer.requests, ["location-1", "location-2", "location-3"]);
    expect(
      identical(container.read(libraryControllerProvider).assets, assetList),
      isTrue,
    );
    expect(
      controller.resolvePreview(initialSnapshot.assets.first).previewStatus,
      LibraryPreviewStatus.ready,
    );
  });

  test(
    "applies preview loading speed changes without cancelling work",
    () async {
      final scanner = _FakeLibraryScanner();
      final previewer = _FakeLibraryPreviewer();
      final initialSnapshot = _snapshot(
        assets: [
          for (var index = 1; index <= 6; index++) _pendingAsset("$index"),
        ],
      );
      final container = ProviderContainer(
        overrides: [
          initialLibraryStateProvider.overrideWithValue(
            LibraryState.fromSnapshot(initialSnapshot),
          ),
          libraryScannerProvider.overrideWithValue(scanner),
          libraryCatalogProvider.overrideWithValue(
            _FakeLibraryCatalog(initialSnapshot),
          ),
          libraryPreviewerProvider.overrideWithValue(previewer),
        ],
      );
      addTearDown(container.dispose);
      addTearDown(scanner.dispose);

      final controller = container.read(libraryControllerProvider.notifier);
      for (final asset in initialSnapshot.assets) {
        controller.requestPreview(asset);
      }
      expect(previewer.requests, ["location-1", "location-2"]);

      await container
          .read(amePreferencesControllerProvider.notifier)
          .update(
            const AmePreferences(
              previewLoadingSpeed: PreviewLoadingSpeed.large,
            ),
          );
      expect(previewer.requests, [
        "location-1",
        "location-2",
        "location-3",
        "location-4",
      ]);

      await container
          .read(amePreferencesControllerProvider.notifier)
          .update(
            const AmePreferences(
              previewLoadingSpeed: PreviewLoadingSpeed.small,
            ),
          );
      for (var index = 1; index <= 3; index++) {
        previewer.succeed("location-$index", _asset(suffix: "$index"));
        await Future<void>.delayed(Duration.zero);
        await Future<void>.delayed(Duration.zero);
      }
      expect(previewer.requests, hasLength(4));

      previewer.succeed("location-4", _asset(suffix: "4"));
      await Future<void>.delayed(Duration.zero);
      await Future<void>.delayed(Duration.zero);
      expect(previewer.requests, [
        "location-1",
        "location-2",
        "location-3",
        "location-4",
        "location-5",
      ]);
    },
  );

  test("records a preview failure and requires an explicit retry", () async {
    final scanner = _FakeLibraryScanner();
    final previewer = _FakeLibraryPreviewer();
    final pending = _pendingAsset("1");
    final initialSnapshot = _snapshot(assets: [pending]);
    final container = ProviderContainer(
      overrides: [
        initialLibraryStateProvider.overrideWithValue(
          LibraryState.fromSnapshot(initialSnapshot),
        ),
        libraryScannerProvider.overrideWithValue(scanner),
        libraryCatalogProvider.overrideWithValue(
          _FakeLibraryCatalog(initialSnapshot),
        ),
        libraryPreviewerProvider.overrideWithValue(previewer),
      ],
    );
    addTearDown(container.dispose);
    addTearDown(scanner.dispose);

    final controller = container.read(libraryControllerProvider.notifier);
    final assetList = container.read(libraryControllerProvider).assets;
    controller.requestPreview(pending);
    previewer.fail("location-1", StateError("decoder stopped"));
    await Future<void>.delayed(Duration.zero);
    await Future<void>.delayed(Duration.zero);

    final failed = controller.resolvePreview(pending);
    expect(failed.previewStatus, LibraryPreviewStatus.failed);
    expect(failed.previewIssueCode, "preview_request_failed");
    expect(
      identical(container.read(libraryControllerProvider).assets, assetList),
      isTrue,
    );

    controller.requestPreview(failed);
    expect(previewer.requests, ["location-1"]);
    controller.requestPreview(failed, retry: true);
    expect(previewer.requests, ["location-1", "location-1"]);
    expect(previewer.retryRequests, [false, true]);

    previewer.succeed("location-1", _asset(suffix: "1"), attempt: 1);
    await Future<void>.delayed(Duration.zero);
    await Future<void>.delayed(Duration.zero);

    expect(
      controller.resolvePreview(pending).previewStatus,
      LibraryPreviewStatus.ready,
    );
  });

  test(
    "maps the latest gallery viewport demand onto preview priorities",
    () async {
      final scanner = _FakeLibraryScanner();
      final previewer = _FakeLibraryPreviewer();
      final assets = [
        for (var index = 1; index <= 4; index++) _pendingAsset("$index"),
      ];
      final initialSnapshot = _snapshot(assets: assets);
      final container = ProviderContainer(
        overrides: [
          initialLibraryStateProvider.overrideWithValue(
            LibraryState.fromSnapshot(initialSnapshot),
          ),
          libraryScannerProvider.overrideWithValue(scanner),
          libraryCatalogProvider.overrideWithValue(
            _FakeLibraryCatalog(initialSnapshot),
          ),
          libraryPreviewerProvider.overrideWithValue(previewer),
        ],
      );
      addTearDown(container.dispose);
      addTearDown(scanner.dispose);

      final controller = container.read(libraryControllerProvider.notifier);
      final assetList = container.read(libraryControllerProvider).assets;
      controller.updateGalleryPreviewDemand(
        visible: assets.sublist(2),
        nearDirection: assets.sublist(0, 2),
      );

      expect(previewer.requests, ["location-3", "location-4"]);

      controller.updateGalleryPreviewDemand(
        visible: [assets.first],
        guard: assets.sublist(2),
      );
      expect(previewer.requests, ["location-3", "location-4", "location-1"]);
      previewer.succeed("location-3", _asset(suffix: "3"));
      await Future<void>.delayed(Duration.zero);
      await Future<void>.delayed(Duration.zero);

      expect(previewer.requests, ["location-3", "location-4", "location-1"]);
      expect(
        identical(container.read(libraryControllerProvider).assets, assetList),
        isTrue,
      );
    },
  );

  test(
    "publishes preview readiness only to the matching location identity",
    () async {
      final scanner = _FakeLibraryScanner();
      final previewer = _FakeLibraryPreviewer();
      final first = _pendingAsset("1");
      final second = _pendingAsset("2");
      final initialSnapshot = _snapshot(assets: [first, second]);
      final container = ProviderContainer(
        overrides: [
          initialLibraryStateProvider.overrideWithValue(
            LibraryState.fromSnapshot(initialSnapshot),
          ),
          libraryScannerProvider.overrideWithValue(scanner),
          libraryCatalogProvider.overrideWithValue(
            _FakeLibraryCatalog(initialSnapshot),
          ),
          libraryPreviewerProvider.overrideWithValue(previewer),
        ],
      );
      addTearDown(container.dispose);
      addTearDown(scanner.dispose);

      final controller = container.read(libraryControllerProvider.notifier);
      final assetList = container.read(libraryControllerProvider).assets;
      var firstPublications = 0;
      var secondPublications = 0;
      final firstSubscription = controller
          .watchPreview(first.locationId)
          .listen((_) => firstPublications++);
      final secondSubscription = controller
          .watchPreview(second.locationId)
          .listen((_) => secondPublications++);

      controller.requestPreview(first);
      previewer.succeed(first.locationId, _asset(suffix: "1"));
      await Future<void>.delayed(Duration.zero);
      await Future<void>.delayed(Duration.zero);

      expect(
        identical(container.read(libraryControllerProvider).assets, assetList),
        isTrue,
      );
      expect(firstPublications, 1);
      expect(secondPublications, 0);
      expect(
        controller.resolvePreview(first).previewStatus,
        LibraryPreviewStatus.ready,
      );
      expect(
        controller.resolvePreview(second).previewStatus,
        LibraryPreviewStatus.pending,
      );

      await firstSubscription.cancel();
      await secondSubscription.cancel();
    },
  );

  test(
    "publishes recovered dimensions with the active global gallery identity",
    () async {
      final scanner = _FakeLibraryScanner();
      final previewer = _FakeLibraryPreviewer();
      final unknown = _unknownDimensionAsset("1");
      final known = _pendingAsset("2");
      final initialSnapshot = _snapshot(assets: [unknown, known]);
      final container = ProviderContainer(
        overrides: [
          initialLibraryStateProvider.overrideWithValue(
            LibraryState.fromSnapshot(
              initialSnapshot,
            ).copyWith(windowStartItemOffset: 40),
          ),
          libraryScannerProvider.overrideWithValue(scanner),
          libraryCatalogProvider.overrideWithValue(
            _FakeLibraryCatalog(initialSnapshot),
          ),
          libraryPreviewerProvider.overrideWithValue(previewer),
        ],
      );
      addTearDown(container.dispose);
      addTearDown(scanner.dispose);

      final controller = container.read(libraryControllerProvider.notifier);
      final updates = <LibraryGalleryLayoutDimensionUpdate>[];
      final subscription = controller.watchLayoutDimensionUpdates().listen(
        updates.add,
      );
      addTearDown(subscription.cancel);

      controller.requestPreview(unknown);
      previewer.succeed(unknown.locationId, _asset(suffix: "1"));
      await Future<void>.delayed(Duration.zero);
      await Future<void>.delayed(Duration.zero);

      expect(updates, hasLength(1));
      expect(updates.single.revision, initialSnapshot.revision);
      expect(updates.single.queryId, initialSnapshot.queryId);
      expect(updates.single.globalItemIndex, 40);
      expect(updates.single.locationId, unknown.locationId);
      expect(updates.single.width, 320);
      expect(updates.single.height, 240);

      controller.requestPreview(known);
      previewer.succeed(known.locationId, _asset(suffix: "2"));
      await Future<void>.delayed(Duration.zero);
      await Future<void>.delayed(Duration.zero);

      expect(updates, hasLength(1));
    },
  );
}

LibraryAsset _asset({String suffix = "1", String? previewPath}) {
  return LibraryAsset(
    assetId: "asset-$suffix",
    locationId: "location-$suffix",
    rootId: "root-1",
    sourcePath: "C:\\Pictures\\$suffix.png",
    displayPath: "C:\\Pictures\\$suffix.png",
    relativePath: "$suffix.png",
    previewPath: previewPath ?? "C:\\AmeCache\\$suffix.jpg",
    fileSize: BigInt.from(128),
    modifiedUnixMs: 42,
    width: 320,
    height: 240,
  );
}

LibraryAsset _pendingAsset(String suffix) {
  return LibraryAsset(
    assetId: "asset-$suffix",
    locationId: "location-$suffix",
    rootId: "root-1",
    sourcePath: "C:\\Pictures\\$suffix.png",
    displayPath: "C:\\Pictures\\$suffix.png",
    relativePath: "$suffix.png",
    previewPath: "",
    fileSize: BigInt.from(128),
    modifiedUnixMs: 42,
    width: 320,
    height: 240,
    previewStatus: LibraryPreviewStatus.pending,
  );
}

LibraryAsset _unknownDimensionAsset(String suffix) {
  return LibraryAsset(
    assetId: "asset-$suffix",
    locationId: "location-$suffix",
    rootId: "root-1",
    sourcePath: "C:\\Pictures\\$suffix.png",
    displayPath: "C:\\Pictures\\$suffix.png",
    relativePath: "$suffix.png",
    previewPath: "",
    fileSize: BigInt.from(128),
    modifiedUnixMs: 42,
    width: 0,
    height: 0,
    previewStatus: LibraryPreviewStatus.pending,
  );
}

LibrarySnapshot _snapshot({
  BigInt? revision,
  List<LibraryRoot> roots = const [],
  List<LibraryAsset> assets = const [],
  LibraryCatalogCursor? previousCursor,
  LibraryCatalogCursor? nextCursor,
}) {
  return LibrarySnapshot(
    catalogPath: "C:\\AmeData\\ame.sqlite3",
    revision: revision ?? BigInt.one,
    queryId: "query-1",
    roots: roots,
    assets: assets,
    previousCursor: previousCursor,
    nextCursor: nextCursor,
  );
}

LibraryCatalogCursor _cursor({String suffix = "1"}) {
  return LibraryCatalogCursor(
    revision: BigInt.one,
    queryId: "query-1",
    primaryMissing: true,
    primaryText: "",
    primaryNumber: 1,
    rootId: "root-1",
    locationId: "location-$suffix",
  );
}

Future<void> _waitForLibraryState(
  ProviderContainer container,
  bool Function(LibraryState state) predicate,
) async {
  if (predicate(container.read(libraryControllerProvider))) {
    return;
  }
  final completer = Completer<void>();
  final subscription = container.listen<LibraryState>(
    libraryControllerProvider,
    (_, next) {
      if (!completer.isCompleted && predicate(next)) {
        completer.complete();
      }
    },
    fireImmediately: true,
  );
  try {
    await completer.future.timeout(const Duration(seconds: 2));
  } finally {
    subscription.close();
  }
}

class _FakeDirectoryPicker implements DirectoryPicker {
  const _FakeDirectoryPicker(this.path);

  final String? path;

  @override
  Future<String?> pickDirectory() async => path;
}

class _FakeLibraryScanner implements LibraryScanner {
  _FakeLibraryScanner({
    this.recoverableScan,
    this.pausedScan,
    this.throwFirstScan = false,
  });

  final _controller = StreamController<LibraryScanUpdate>.broadcast();
  final RecoverableLibraryScan? recoverableScan;
  final RecoverableLibraryScan? pausedScan;
  final bool throwFirstScan;
  String? cancelledScanId;
  String? pausedScanId;
  String? startedScanId;
  int? startedItemLimit;
  int? startedEntryLimit;
  int scanCallCount = 0;

  void add(LibraryScanUpdate update) {
    _controller.add(update);
  }

  void dispose() {
    if (!_controller.isClosed) {
      unawaited(_controller.close());
    }
  }

  Future<void> close() async {
    if (!_controller.isClosed) {
      await _controller.close();
    }
  }

  @override
  bool cancel(String scanId) {
    cancelledScanId = scanId;
    return true;
  }

  @override
  Future<RecoverableLibraryScan?> loadRecoverableScan() async {
    return recoverableScan;
  }

  @override
  Future<RecoverableLibraryScan?> loadPausedScan() async {
    return pausedScan;
  }

  @override
  bool pause(String scanId) {
    pausedScanId = scanId;
    return true;
  }

  @override
  Stream<LibraryScanUpdate> scan({
    required String scanId,
    required String rootPath,
    required int? itemLimit,
    required int? entryLimit,
    required int previewEdge,
  }) {
    scanCallCount += 1;
    if (throwFirstScan && scanCallCount == 1) {
      throw StateError("synthetic stream creation failure");
    }
    startedScanId = scanId;
    startedItemLimit = itemLimit;
    startedEntryLimit = entryLimit;
    return _controller.stream;
  }
}

class _FakeLibraryCatalog implements LibraryCatalog {
  _FakeLibraryCatalog(LibrarySnapshot snapshot)
    : _responses = [snapshot],
      _lastRevision = snapshot.revision,
      _snapshotFactory = null;

  _FakeLibraryCatalog.sequence(
    List<Object> responses, {
    required BigInt initialRevision,
  }) : _responses = List.of(responses),
       _lastRevision = initialRevision,
       _snapshotFactory = null;

  _FakeLibraryCatalog.dynamic(LibrarySnapshot Function() snapshotFactory)
    : _responses = const [],
      _lastRevision = BigInt.one,
      _snapshotFactory = snapshotFactory;

  final List<Object> _responses;
  final LibrarySnapshot Function()? _snapshotFactory;
  final List<LibraryCatalogCursor?> afters = [];
  final List<LibraryCatalogCursor?> befores = [];
  final List<LibraryTimeAnchor> anchors = [];
  final List<int> timeWindowSizes = [];
  final List<String> unregisteredRootIds = [];
  BigInt _lastRevision;

  @override
  Future<LibrarySnapshot> load({
    required int maxItems,
    required LibraryGalleryQuery query,
    LibraryCatalogCursor? after,
    LibraryCatalogCursor? before,
  }) async {
    afters.add(after);
    befores.add(before);
    return _nextSnapshot();
  }

  @override
  Future<LibrarySnapshot> loadAtTime({
    required int maxItems,
    required LibraryGalleryQuery query,
    required LibraryTimeAnchor anchor,
  }) async {
    anchors.add(anchor);
    timeWindowSizes.add(maxItems);
    return _nextSnapshot();
  }

  @override
  Future<LibraryTimeline> loadTimeline(LibraryGalleryQuery query) async {
    return LibraryTimeline(
      revision: _lastRevision,
      queryId: "query-1",
      totalItems: 0,
      buckets: const [],
    );
  }

  @override
  Future<bool> unregisterRoot(String rootId) async {
    unregisteredRootIds.add(rootId);
    return true;
  }

  Future<LibrarySnapshot> _nextSnapshot() async {
    final snapshotFactory = _snapshotFactory;
    if (snapshotFactory != null) {
      final snapshot = snapshotFactory();
      _lastRevision = snapshot.revision;
      return snapshot;
    }
    if (_responses.isEmpty) {
      throw StateError("No fake catalog response remains");
    }
    final response = _responses.removeAt(0);
    if (response is LibrarySnapshot) {
      _lastRevision = response.revision;
      return response;
    }
    if (response is Future<LibrarySnapshot>) {
      final snapshot = await response;
      _lastRevision = snapshot.revision;
      return snapshot;
    }
    throw response;
  }
}

class _DelayedDoneLibraryScanner implements LibraryScanner {
  final List<StreamController<LibraryScanUpdate>> _controllers = [];
  final List<String> startedScanIds = [];

  void add(int index, LibraryScanUpdate update) {
    _controllers[index].add(update);
  }

  Future<void> close(int index) => _controllers[index].close();

  void dispose() {
    for (final controller in _controllers) {
      if (!controller.isClosed) {
        unawaited(controller.close());
      }
    }
  }

  @override
  bool cancel(String scanId) => true;

  @override
  Future<RecoverableLibraryScan?> loadPausedScan() async => null;

  @override
  Future<RecoverableLibraryScan?> loadRecoverableScan() async => null;

  @override
  bool pause(String scanId) => true;

  @override
  Stream<LibraryScanUpdate> scan({
    required String scanId,
    required String rootPath,
    required int? itemLimit,
    required int? entryLimit,
    required int previewEdge,
  }) {
    startedScanIds.add(scanId);
    final controller = StreamController<LibraryScanUpdate>();
    _controllers.add(controller);
    return controller.stream;
  }
}

class _FakeLibraryPreviewer implements LibraryPreviewer {
  final List<String> requests = [];
  final List<bool> retryRequests = [];
  final Map<String, List<Completer<LibraryAsset>>> _attempts = {};

  @override
  Future<LibraryAsset> materialize({
    required String locationId,
    required int previewEdge,
    bool retry = false,
    Iterable<String> protectedLocationIds = const [],
  }) {
    requests.add(locationId);
    retryRequests.add(retry);
    final completer = Completer<LibraryAsset>();
    _attempts.putIfAbsent(locationId, () => []).add(completer);
    return completer.future;
  }

  void succeed(String locationId, LibraryAsset asset, {int attempt = 0}) {
    _attempts[locationId]![attempt].complete(asset);
  }

  void fail(String locationId, Object error, {int attempt = 0}) {
    _attempts[locationId]![attempt].completeError(error);
  }
}
