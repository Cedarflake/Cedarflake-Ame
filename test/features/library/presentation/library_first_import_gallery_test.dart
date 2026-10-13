import "package:cedarflake_ame/app/ame_app.dart";
import "package:cedarflake_ame/features/library/application/library_catalog.dart";
import "package:cedarflake_ame/features/library/application/library_controller.dart";
import "package:cedarflake_ame/features/library/application/library_scanner.dart";
import "package:cedarflake_ame/features/library/domain/library_models.dart";
import "package:cedarflake_ame/features/library/domain/library_state.dart";
import "package:cedarflake_ame/features/library/presentation/library_pending_source.dart";
import "package:cedarflake_ame/features/library/presentation/library_strings.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_gallery_states.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_gallery_wall.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_source_navigation_tile.dart";
import "package:flutter/material.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:flutter_test/flutter_test.dart";

import "../support/retained_scan_fixture.dart";

void main() {
  testWidgets("restored first import does not claim its folder is empty", (
    tester,
  ) async {
    final fixture = await _show(tester);
    expect(
      fixture.container.read(libraryControllerProvider).stagedAssetCount,
      40,
    );
    expect(find.byKey(const Key("library-first-import-state")), findsOneWidget);
    expect(find.text(LibraryStrings.noSourceResults), findsNothing);
    expect(find.byKey(const Key("library-empty-state")), findsNothing);
    expect(find.byKey(const Key("library-resume-button")), findsOneWidget);
    expect(fixture.scanner.startedRoots, isEmpty);

    await tester.enterText(find.byType(SearchBar), "missing.jpg");
    await tester.pump(const Duration(milliseconds: 400));
    await tester.pump();
    expect(find.byKey(const Key("library-first-import-state")), findsOneWidget);
    expect(find.text(LibraryStrings.noSearchResults), findsNothing);

    await tester.tap(find.byKey(const ValueKey("source-title-other")));
    await tester.pump();
    await tester.pump();
    expect(find.byKey(const Key("library-first-import-state")), findsNothing);
    expect(find.text(LibraryStrings.noSearchResults), findsOneWidget);

    await tester.tap(find.byKey(const ValueKey("source-title-retained")));
    await tester.pump();
    await tester.pump();
    expect(find.byKey(const Key("library-first-import-state")), findsOneWidget);
    expect(fixture.scanner.startedRoots, isEmpty);
  });

  testWidgets("cancelling a retained import does not prove the folder empty", (
    tester,
  ) async {
    final fixture = await _show(tester);
    await tester.tap(find.byKey(const Key("library-cancel-button")));
    await tester.pump();
    await tester.pump();
    expect(fixture.scanner.cancelRetainedIds, [retainedScanCheckpoint.scanId]);
    expect(find.byKey(const Key("library-first-import-state")), findsOneWidget);
    expect(find.text(LibraryStrings.noSourceResults), findsNothing);
    expect(find.byKey(const Key("library-resume-button")), findsNothing);
  });

  testWidgets("Continue keeps incomplete feedback until catalog publication", (
    tester,
  ) async {
    final fixture = await _show(tester);
    await tester.tap(find.byKey(const Key("library-resume-button")));
    await tester.pump();
    expect(fixture.scanner.resumedIds, [retainedScanCheckpoint.scanId]);
    expect(find.byKey(const Key("library-first-import-state")), findsOneWidget);
    expect(find.text(LibraryStrings.noSourceResults), findsNothing);

    fixture.catalog.didPublish = true;
    fixture.scanner.completeScan(retainedScanCheckpoint.scanId);
    await tester.pump();
    await tester.pump();
    expect(find.byKey(const Key("library-first-import-state")), findsNothing);
    expect(find.byType(LibraryGalleryWall), findsOneWidget);
    expect(
      fixture.container.read(libraryControllerProvider).assets,
      everyElement(
        isA<LibraryAsset>().having((asset) => asset.rootId, "root", "retained"),
      ),
    );
  });

  testWidgets("published peer content stays visible during paused import", (
    tester,
  ) async {
    await _show(tester, showPublishedAssets: true);
    expect(find.byType(LibraryGalleryWall), findsOneWidget);
    expect(find.byKey(const Key("library-first-import-state")), findsNothing);
    expect(find.byKey(const Key("library-resume-button")), findsOneWidget);
  });

  for (final hasRegisteredImport in [true, false]) {
    testWidgets(
      "published import retries display only with registered root $hasRegisteredImport",
      (tester) async {
        final fixture = await _show(
          tester,
          hasRegisteredImport: hasRegisteredImport,
        );
        await tester.tap(find.byKey(const Key("library-resume-button")));
        await tester.pump();
        fixture.catalog.didPublish = true;
        fixture.catalog.loadFailure = StateError("display unavailable");
        fixture.scanner.completeScan(retainedScanCheckpoint.scanId);
        await tester.pump();
        await tester.pump();

        expect(
          fixture.container.read(libraryControllerProvider).status,
          LibraryStatus.failed,
        );
        expect(
          find.byKey(const Key("library-published-reload-state")),
          findsOneWidget,
        );
        expect(
          find.byKey(const Key("library-first-import-state")),
          findsNothing,
        );
        expect(find.text(LibraryStrings.firstImportIncomplete), findsNothing);
        expect(find.byKey(const Key("pending-source-progress")), findsNothing);

        fixture.catalog.loadFailure = null;
        await tester.tap(find.byKey(const Key("library-retry-button")));
        await tester.pump();
        await tester.pump();
        expect(find.byType(LibraryGalleryWall), findsOneWidget);
        expect(
          find.byKey(const Key("library-published-reload-state")),
          findsNothing,
        );
        expect(fixture.scanner.resumedIds, [retainedScanCheckpoint.scanId]);
        expect(fixture.scanner.startedRoots, [retainedScanRoot.path]);
      },
    );
  }

  testWidgets("pending sidebar source stops its spinner while paused", (
    tester,
  ) async {
    final fixture = await _show(tester, hasRegisteredImport: false);
    final pendingTile = find.byKey(const Key("pending-source-tile"));
    expect(pendingTile, findsOneWidget);
    expect(
      find.descendant(
        of: pendingTile,
        matching: find.text(LibraryStrings.firstImportIncomplete),
      ),
      findsOneWidget,
    );
    expect(find.byKey(const Key("pending-source-progress")), findsNothing);

    await tester.tap(find.byKey(const Key("library-resume-button")));
    await tester.pump();
    expect(find.byKey(const Key("pending-source-progress")), findsOneWidget);
    expect(find.text(LibraryStrings.addingSource), findsOneWidget);

    fixture.catalog.didPublish = true;
    fixture.scanner.completeScan(retainedScanCheckpoint.scanId);
    await tester.pump();
    await tester.pump();
    expect(pendingTile, findsNothing);
    expect(find.byKey(const Key("library-first-import-state")), findsNothing);
  });

  testWidgets("first import stays incomplete before its root list refresh", (
    tester,
  ) async {
    for (final status in [LibraryStatus.scanning, LibraryStatus.paused]) {
      await tester.pumpWidget(
        MaterialApp(
          home: EmptyLibraryGallery(
            state: LibraryState(
              primaryScan: LibraryPrimaryScanSnapshot(
                status: status,
                taskKind: LibraryTaskKind.import,
                scanId: "first-import",
                rootPath: retainedScanRoot.path,
              ),
            ),
            onImport: () {},
          ),
        ),
      );
      expect(
        find.byKey(const Key("library-first-import-state")),
        findsOneWidget,
      );
      expect(find.byKey(const Key("library-empty-state")), findsNothing);
    }
  });

  testWidgets("published empty folder and new library retain their states", (
    tester,
  ) async {
    await tester.pumpWidget(
      MaterialApp(
        home: EmptyLibraryGallery(
          state: const LibraryState(roots: [otherPublishedRoot]),
          onImport: () {},
        ),
      ),
    );
    expect(find.text(LibraryStrings.noSourceResults), findsOneWidget);
    await tester.pumpWidget(
      MaterialApp(
        home: EmptyLibraryGallery(state: const LibraryState(), onImport: () {}),
      ),
    );
    expect(find.byKey(const Key("library-empty-state")), findsOneWidget);
  });

  testWidgets("query loading does not animate the compact paused source", (
    tester,
  ) async {
    const state = LibraryState(
      queryActivity: LibraryQueryLoading(
        LibraryGalleryQuery(searchText: "new"),
      ),
      primaryScan: LibraryPrimaryScanSnapshot(
        taskKind: LibraryTaskKind.import,
        status: LibraryStatus.paused,
        scanId: "paused",
        rootPath: "C:\\Retained",
        displayRootPath: "C:\\Retained",
      ),
    );
    await tester.pumpWidget(
      MaterialApp(
        home: Row(
          children: [
            PendingLibrarySourceTile(
              source: LibraryPendingSource.fromState(state)!,
              isCompact: true,
            ),
            Expanded(
              child: EmptyLibraryGallery(state: state, onImport: () {}),
            ),
          ],
        ),
      ),
    );
    expect(find.byKey(const Key("library-query-loading")), findsOneWidget);
    expect(
      find.descendant(
        of: find.byType(PendingLibrarySourceTile),
        matching: find.byType(CircularProgressIndicator),
      ),
      findsNothing,
    );
    expect(find.byKey(const Key("library-first-import-state")), findsNothing);
  });

  testWidgets(
    "a peer display failure does not override selected first import",
    (tester) async {
      const state = LibraryState(
        roots: [retainedScanRoot, otherPublishedRoot],
        query: LibraryGalleryQuery(rootId: "retained"),
        primaryScan: LibraryPrimaryScanSnapshot(
          taskKind: LibraryTaskKind.update,
          status: LibraryStatus.failed,
          publication: LibraryScanPublication.reloadPending,
          rootPath: "C:\\Other",
          displayRootPath: "C:\\Other",
        ),
      );
      await tester.pumpWidget(
        MaterialApp(
          home: EmptyLibraryGallery(state: state, onImport: () {}),
        ),
      );
      expect(
        find.byKey(const Key("library-first-import-state")),
        findsOneWidget,
      );
      expect(
        find.byKey(const Key("library-published-reload-state")),
        findsNothing,
      );
      await tester.pumpWidget(
        MaterialApp(
          home: EmptyLibraryGallery(
            state: state.copyWith(
              query: const LibraryGalleryQuery(rootId: "other"),
            ),
            onImport: () {},
          ),
        ),
      );
      expect(
        find.byKey(const Key("library-published-reload-state")),
        findsOneWidget,
      );
      expect(find.byKey(const Key("library-first-import-state")), findsNothing);
    },
  );
}

Future<
  ({
    ProviderContainer container,
    RetainedScanScanner scanner,
    _FirstImportCatalog catalog,
  })
>
_show(
  WidgetTester tester, {
  bool showPublishedAssets = false,
  bool hasRegisteredImport = true,
}) async {
  tester.view.physicalSize = const Size(1280, 800);
  tester.view.devicePixelRatio = 1;
  addTearDown(tester.view.resetPhysicalSize);
  addTearDown(tester.view.resetDevicePixelRatio);
  final scanner = RetainedScanScanner();
  final catalog = _FirstImportCatalog(scanner)
    ..showPublishedAssets = showPublishedAssets;
  if (!hasRegisteredImport) {
    catalog.roots.removeWhere((root) => root.id == retainedScanRoot.id);
  }
  final container = ProviderContainer(
    overrides: [
      libraryScannerProvider.overrideWithValue(scanner),
      libraryCatalogProvider.overrideWithValue(catalog),
      initialLibraryStateProvider.overrideWithValue(
        LibraryState.fromSnapshot(
          catalog.snapshot(const LibraryGalleryQuery()),
        ),
      ),
    ],
  );
  addTearDown(container.dispose);
  addTearDown(scanner.dispose);
  await tester.pumpWidget(
    UncontrolledProviderScope(container: container, child: const AmeApp()),
  );
  await tester.pump();
  await tester.pump();
  return (container: container, scanner: scanner, catalog: catalog);
}

class _FirstImportCatalog extends RetainedScanCatalog {
  _FirstImportCatalog(super.scanner);

  bool didPublish = false;
  bool showPublishedAssets = false;

  @override
  LibrarySnapshot snapshot(LibraryGalleryQuery query, {int offset = 0}) {
    final published = super.snapshot(query, offset: offset);
    final List<LibraryAsset> assets;
    if (didPublish) {
      assets = _completedImportAssets;
    } else if (showPublishedAssets) {
      assets = published.assets;
    } else {
      assets = const [];
    }
    return LibrarySnapshot(
      catalogPath: published.catalogPath,
      revision: BigInt.one,
      queryId: published.queryId,
      roots: didPublish
          ? const [_completedImportRoot, otherPublishedRoot]
          : roots,
      assets: assets,
    );
  }

  @override
  Future<LibraryTimeline> loadTimeline(LibraryGalleryQuery query) async {
    final current = snapshot(query);
    return LibraryTimeline(
      revision: current.revision,
      queryId: current.queryId,
      totalItems: current.assets.length,
      buckets: [
        if (current.assets.isNotEmpty)
          LibraryTimeBucket(
            monthKey: "2026-09",
            itemCount: current.assets.length,
            aspectRatioSum: current.assets.length.toDouble(),
          ),
      ],
    );
  }
}

const _completedImportRoot = LibraryRoot(
  id: "retained",
  path: "C:\\Retained",
  displayPath: "C:\\Retained",
  activeScanId: "retained-first-import",
  createdUnixMs: 1,
  assetCount: 6,
  issueCount: 0,
  availability: LibraryRootAvailability.available,
);

final _completedImportAssets = [
  for (var index = 0; index < 6; index++)
    LibraryAsset(
      assetId: "retained-$index",
      locationId: "retained-$index",
      rootId: _completedImportRoot.id,
      activeScanId: retainedScanCheckpoint.scanId,
      sourcePath: "C:\\Retained\\$index.png",
      displayPath: "C:\\Retained\\$index.png",
      relativePath: "$index.png",
      previewPath: "",
      fileSize: BigInt.from(128),
      modifiedUnixMs: 1,
      sourceRevision: null,
      sourceGeneration: BigInt.one,
      width: 32,
      height: 32,
      previewStatus: LibraryPreviewStatus.pending,
    ),
];
