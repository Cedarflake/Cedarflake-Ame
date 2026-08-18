import "dart:async";

import "package:cedarflake_ame/app/ame_app.dart";
import "package:cedarflake_ame/features/library/adapters/windows_library_platform_actions.dart";
import "package:cedarflake_ame/features/library/application/library_catalog.dart";
import "package:cedarflake_ame/features/library/application/library_controller.dart";
import "package:cedarflake_ame/features/library/application/library_platform_actions.dart";
import "package:cedarflake_ame/features/library/application/library_synchronization.dart";
import "package:cedarflake_ame/features/library/domain/library_models.dart";
import "package:cedarflake_ame/features/library/domain/library_state.dart";
import "package:cedarflake_ame/features/library/domain/library_synchronization_models.dart";
import "package:cedarflake_ame/features/library/presentation/library_strings.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_photo_tile.dart";
import "package:flutter/material.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:flutter_test/flutter_test.dart";

void main() {
  testWidgets("preserves gallery position after closing the image viewer", (
    tester,
  ) async {
    tester.view.physicalSize = const Size(1280, 800);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);

    await tester.pumpWidget(
      ProviderScope(
        overrides: [
          initialLibraryStateProvider.overrideWithValue(_libraryState()),
        ],
        child: const AmeApp(),
      ),
    );
    await tester.pumpAndSettle();

    final photoWall = find.byKey(const Key("library-photo-wall"));
    await tester.drag(photoWall, const Offset(0, -1200));
    await tester.pumpAndSettle();

    final scrollable = find.descendant(
      of: photoWall,
      matching: find.byType(Scrollable),
    );
    final positionBefore = tester.state<ScrollableState>(scrollable).position;
    final offsetBefore = positionBefore.pixels;
    expect(offsetBefore, greaterThan(0));

    final tile = find.byType(LibraryPhotoTile).hitTestable().first;
    final tileRect = tester.getRect(tile);
    await tester.tapAt(tileRect.topLeft + const Offset(16, 16));
    await tester.pump();
    final backButton = find.byKey(const Key("viewer-back-button"));
    expect(backButton, findsOneWidget);
    await tester.tap(backButton);
    await tester.pump();

    final restoredScrollable = find.descendant(
      of: find.byKey(const Key("library-photo-wall")),
      matching: find.byType(Scrollable),
    );
    final positionAfter = tester
        .state<ScrollableState>(restoredScrollable)
        .position;
    expect(identical(positionAfter, positionBefore), isTrue);
    expect(positionAfter.pixels, closeTo(offsetBefore, 0.01));
  });

  testWidgets("reveals the active viewer file after adjacent navigation", (
    tester,
  ) async {
    tester.view.physicalSize = const Size(1280, 800);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);
    final platformActions = _RecordingPlatformActions();

    await tester.pumpWidget(
      ProviderScope(
        overrides: [
          initialLibraryStateProvider.overrideWithValue(
            _libraryState(
              assetCount: 2,
              sourceRoot: r"\\?\G:\CloudLibrary\图片",
              displayRoot: r"G:\CloudLibrary\图片",
            ),
          ),
          libraryPlatformActionsProvider.overrideWithValue(platformActions),
        ],
        child: const AmeApp(),
      ),
    );
    await tester.pumpAndSettle();

    final openingTile = find.byType(LibraryPhotoTile).hitTestable().first;
    final openingTileRect = tester.getRect(openingTile);
    await tester.tapAt(openingTileRect.topLeft + const Offset(16, 16));
    await tester.pump();
    await _revealCurrentViewerFile(tester);
    expect(platformActions.revealedFiles, [r"\\?\G:\CloudLibrary\图片\0.jpg"]);

    await tester.tap(find.byKey(const Key("viewer-next")));
    await tester.pump();
    await _revealCurrentViewerFile(tester);
    expect(platformActions.revealedFiles, [
      r"\\?\G:\CloudLibrary\图片\0.jpg",
      r"\\?\G:\CloudLibrary\图片\1.jpg",
    ]);
  });

  testWidgets(
    "refreshes a catalog revision already published before screen subscription",
    (tester) async {
      tester.view.physicalSize = const Size(1280, 800);
      tester.view.devicePixelRatio = 1;
      addTearDown(tester.view.resetPhysicalSize);
      addTearDown(tester.view.resetDevicePixelRatio);
      final initialState = _libraryState(assetCount: 1);
      final initialAsset = initialState.assets.single;
      final renamedAsset = _assetAtPath(
        initialAsset,
        locationId: "location-renamed-before-subscription",
        relativePath: "renamed-before-subscription.jpg",
      );
      final catalog = _SynchronizationViewerCatalog(
        LibrarySnapshot(
          catalogPath: initialState.catalogPath ?? "",
          revision: BigInt.two,
          queryId: initialState.queryId,
          roots: initialState.roots,
          assets: [renamedAsset],
        ),
      );
      final synchronization = _TestLibrarySynchronization(
        _synchronizationSnapshot(BigInt.two),
      );
      addTearDown(synchronization.dispose);

      await tester.pumpWidget(
        ProviderScope(
          overrides: [
            initialLibraryStateProvider.overrideWithValue(initialState),
            libraryCatalogProvider.overrideWithValue(catalog),
            librarySynchronizationProvider.overrideWithValue(synchronization),
          ],
          child: const AmeApp(),
        ),
      );
      await _pumpSynchronizationRefresh(tester);

      final container = ProviderScope.containerOf(
        tester.element(find.byType(AmeApp)),
      );
      final refreshedState = container.read(libraryControllerProvider);
      expect(refreshedState.catalogRevision, BigInt.two);
      expect(
        refreshedState.assets.single.locationId,
        "location-renamed-before-subscription",
      );
    },
  );

  testWidgets(
    "keeps the preferred viewer location until authoritative lookup resolves it",
    (tester) async {
      tester.view.physicalSize = const Size(1280, 800);
      tester.view.devicePixelRatio = 1;
      addTearDown(tester.view.resetPhysicalSize);
      addTearDown(tester.view.resetDevicePixelRatio);
      final baseState = _libraryState(assetCount: 1);
      final firstLocation = baseState.assets.single;
      final preferredLocation = _assetAtPath(
        firstLocation,
        locationId: "location-preferred",
        relativePath: "preferred.jpg",
      );
      final initialState = _stateWithAssets(baseState, [
        firstLocation,
        preferredLocation,
      ]);
      final initialSnapshot = LibrarySnapshot(
        catalogPath: initialState.catalogPath ?? "",
        revision: BigInt.one,
        queryId: initialState.queryId,
        roots: initialState.roots,
        assets: initialState.assets,
      );
      final catalog = _SynchronizationViewerCatalog(initialSnapshot);
      final synchronization = _TestLibrarySynchronization(
        _synchronizationSnapshot(BigInt.one),
      );
      addTearDown(synchronization.dispose);

      await tester.pumpWidget(
        ProviderScope(
          overrides: [
            initialLibraryStateProvider.overrideWithValue(initialState),
            libraryCatalogProvider.overrideWithValue(catalog),
            librarySynchronizationProvider.overrideWithValue(synchronization),
          ],
          child: const AmeApp(),
        ),
      );
      await tester.pumpAndSettle();

      final preferredTile = find.byWidgetPredicate(
        (widget) =>
            widget is LibraryPhotoTile &&
            widget.asset.locationId == preferredLocation.locationId,
      );
      final preferredTileRect = tester.getRect(preferredTile);
      await tester.tapAt(preferredTileRect.topLeft + const Offset(16, 16));
      await tester.pump();
      expect(find.text("preferred.jpg"), findsOneWidget);

      catalog.snapshot = LibrarySnapshot(
        catalogPath: initialSnapshot.catalogPath,
        revision: BigInt.two,
        queryId: initialSnapshot.queryId,
        roots: initialSnapshot.roots,
        assets: [firstLocation],
        queryAnchorResolution: const LibraryQueryAnchorResolution(
          requestedLocationId: "location-preferred",
          locationId: "location-0",
          ordinal: 0,
          windowStartItemOffset: 0,
        ),
      );
      synchronization.publish(_synchronizationSnapshot(BigInt.two));
      await _pumpSynchronizationRefresh(tester);

      expect(catalog.requestedLocationIds, ["location-preferred"]);
      expect(catalog.preferredLocationIds, ["location-preferred"]);
      expect(find.byKey(const Key("viewer-back-button")), findsOneWidget);
      expect(find.text("0.jpg"), findsOneWidget);
    },
  );

  testWidgets(
    "stale asset lookup cannot replace newer same-asset viewer navigation",
    (tester) async {
      tester.view.physicalSize = const Size(1280, 800);
      tester.view.devicePixelRatio = 1;
      addTearDown(tester.view.resetPhysicalSize);
      addTearDown(tester.view.resetDevicePixelRatio);
      final baseState = _libraryState(assetCount: 1);
      final firstLocation = baseState.assets.single;
      final secondLocation = _assetAtPath(
        firstLocation,
        locationId: "location-second",
        relativePath: "second.jpg",
      );
      final initialState = _stateWithAssets(baseState, [
        firstLocation,
        secondLocation,
      ]);
      final initialSnapshot = LibrarySnapshot(
        catalogPath: initialState.catalogPath ?? "",
        revision: BigInt.one,
        queryId: initialState.queryId,
        roots: initialState.roots,
        assets: initialState.assets,
      );
      final catalog = _SynchronizationViewerCatalog(initialSnapshot);
      final synchronization = _TestLibrarySynchronization(
        _synchronizationSnapshot(BigInt.one),
      );
      addTearDown(synchronization.dispose);

      await tester.pumpWidget(
        ProviderScope(
          overrides: [
            initialLibraryStateProvider.overrideWithValue(initialState),
            libraryCatalogProvider.overrideWithValue(catalog),
            librarySynchronizationProvider.overrideWithValue(synchronization),
          ],
          child: const AmeApp(),
        ),
      );
      await tester.pumpAndSettle();

      final firstTile = find.byWidgetPredicate(
        (widget) =>
            widget is LibraryPhotoTile &&
            widget.asset.locationId == firstLocation.locationId,
      );
      final firstTileRect = tester.getRect(firstTile);
      await tester.tapAt(firstTileRect.topLeft + const Offset(16, 16));
      await tester.pump();
      expect(find.text("0.jpg"), findsOneWidget);

      catalog.snapshot = LibrarySnapshot(
        catalogPath: initialSnapshot.catalogPath,
        revision: BigInt.two,
        queryId: initialSnapshot.queryId,
        roots: initialSnapshot.roots,
        assets: [firstLocation, secondLocation],
        queryAnchorResolution: const LibraryQueryAnchorResolution(
          requestedLocationId: "location-0",
          locationId: "location-0",
          ordinal: 0,
          windowStartItemOffset: 0,
        ),
      );
      catalog.holdNextAssetLookup();
      synchronization.publish(_synchronizationSnapshot(BigInt.two));
      await tester.pump(const Duration(milliseconds: 100));
      expect(catalog.preferredLocationIds, ["location-0"]);

      await tester.tap(find.byKey(const Key("viewer-next")));
      await tester.pump();
      expect(find.text("second.jpg"), findsOneWidget);

      catalog.completeHeldAssetLookup(firstLocation);
      await _pumpSynchronizationRefresh(tester);

      expect(find.byKey(const Key("viewer-back-button")), findsOneWidget);
      expect(find.text("second.jpg"), findsOneWidget);
      expect(find.text("0.jpg"), findsNothing);
    },
  );

  testWidgets(
    "synchronization keeps a renamed viewer asset and closes an authoritative removal",
    (tester) async {
      tester.view.physicalSize = const Size(1280, 800);
      tester.view.devicePixelRatio = 1;
      addTearDown(tester.view.resetPhysicalSize);
      addTearDown(tester.view.resetDevicePixelRatio);
      final initialState = _libraryState(assetCount: 1);
      final initialAsset = initialState.assets.single;
      final initialSnapshot = LibrarySnapshot(
        catalogPath: initialState.catalogPath ?? "",
        revision: initialState.catalogRevision ?? BigInt.one,
        queryId: initialState.queryId,
        roots: initialState.roots,
        assets: initialState.assets,
      );
      final catalog = _SynchronizationViewerCatalog(initialSnapshot);
      final synchronization = _TestLibrarySynchronization(
        _synchronizationSnapshot(BigInt.one),
      );
      addTearDown(synchronization.dispose);

      await tester.pumpWidget(
        ProviderScope(
          overrides: [
            initialLibraryStateProvider.overrideWithValue(initialState),
            libraryCatalogProvider.overrideWithValue(catalog),
            librarySynchronizationProvider.overrideWithValue(synchronization),
          ],
          child: const AmeApp(),
        ),
      );
      await tester.pumpAndSettle();

      final openingTile = find.byType(LibraryPhotoTile).hitTestable().first;
      final openingTileRect = tester.getRect(openingTile);
      await tester.tapAt(openingTileRect.topLeft + const Offset(16, 16));
      await tester.pump();
      expect(find.byKey(const Key("viewer-back-button")), findsOneWidget);
      expect(find.text("0.jpg"), findsOneWidget);

      final renamedAsset = _assetAtPath(
        initialAsset,
        locationId: "location-renamed",
        relativePath: "renamed.jpg",
      );
      catalog.snapshot = LibrarySnapshot(
        catalogPath: initialSnapshot.catalogPath,
        revision: BigInt.two,
        queryId: initialSnapshot.queryId,
        roots: initialSnapshot.roots,
        assets: [renamedAsset],
        queryAnchorResolution: const LibraryQueryAnchorResolution(
          requestedLocationId: "location-0",
          locationId: "location-renamed",
          ordinal: 0,
          windowStartItemOffset: 0,
        ),
      );
      synchronization.publish(_synchronizationSnapshot(BigInt.two));
      await _pumpSynchronizationRefresh(tester);

      expect(find.byKey(const Key("viewer-back-button")), findsOneWidget);
      expect(find.text("renamed.jpg"), findsOneWidget);
      expect(catalog.requestedLocationIds, ["location-0"]);
      expect(catalog.preferredLocationIds, ["location-0"]);

      final fallbackAsset = _assetAtPath(
        initialAsset,
        assetId: "asset-fallback",
        locationId: "location-fallback",
        relativePath: "fallback.jpg",
      );
      catalog.snapshot = LibrarySnapshot(
        catalogPath: initialSnapshot.catalogPath,
        revision: BigInt.from(3),
        queryId: initialSnapshot.queryId,
        roots: initialSnapshot.roots,
        assets: [fallbackAsset],
        queryAnchorResolution: const LibraryQueryAnchorResolution(
          requestedLocationId: "location-renamed",
          locationId: "location-fallback",
          ordinal: 0,
          windowStartItemOffset: 0,
        ),
      );
      synchronization.publish(_synchronizationSnapshot(BigInt.from(3)));
      await _pumpSynchronizationRefresh(tester);

      expect(find.byKey(const Key("viewer-back-button")), findsNothing);
      expect(find.byKey(const Key("library-photo-wall")), findsOneWidget);
      expect(catalog.requestedLocationIds, ["location-0", "location-renamed"]);
      expect(catalog.preferredLocationIds, ["location-0", "location-renamed"]);
    },
  );
}

Future<void> _revealCurrentViewerFile(WidgetTester tester) async {
  await tester.tap(find.byKey(const Key("viewer-more-menu")));
  await tester.pump();
  await tester.pump(const Duration(milliseconds: 500));
  await tester.tap(find.text(LibraryStrings.openInExplorer));
  await tester.pump();
  await tester.pump(const Duration(milliseconds: 500));
}

Future<void> _pumpSynchronizationRefresh(WidgetTester tester) async {
  for (var index = 0; index < 8; index++) {
    await tester.pump(const Duration(milliseconds: 100));
  }
}

LibraryState _libraryState({
  int assetCount = 120,
  String sourceRoot = r"C:\Pictures",
  String? displayRoot,
}) {
  final readableRoot = displayRoot ?? sourceRoot;
  final assets = [
    for (var index = 0; index < assetCount; index++)
      LibraryAsset(
        assetId: "asset-$index",
        locationId: "location-$index",
        rootId: "root-1",
        sourcePath: "$sourceRoot\\$index.jpg",
        displayPath: "$readableRoot\\$index.jpg",
        relativePath: "$index.jpg",
        previewPath: "C:\\Ame\\previews\\$index.jpg",
        fileSize: BigInt.one,
        modifiedUnixMs: index,
        width: 4,
        height: 3,
        previewStatus: LibraryPreviewStatus.ready,
      ),
  ];
  return LibraryState.fromSnapshot(
    LibrarySnapshot(
      catalogPath: "C:\\Ame\\catalog.db",
      revision: BigInt.one,
      queryId: "viewer-position",
      roots: [
        LibraryRoot(
          id: "root-1",
          path: sourceRoot,
          displayPath: readableRoot,
          createdUnixMs: 1,
          assetCount: assets.length,
          issueCount: 0,
        ),
      ],
      assets: assets,
    ),
  );
}

LibraryState _stateWithAssets(LibraryState source, List<LibraryAsset> assets) {
  return LibraryState.fromSnapshot(
    LibrarySnapshot(
      catalogPath: source.catalogPath ?? "",
      revision: source.catalogRevision ?? BigInt.one,
      queryId: source.queryId,
      roots: source.roots,
      assets: assets,
    ),
  );
}

LibraryAsset _assetAtPath(
  LibraryAsset source, {
  String? assetId,
  required String locationId,
  required String relativePath,
}) {
  return LibraryAsset(
    assetId: assetId ?? source.assetId,
    locationId: locationId,
    rootId: source.rootId,
    sourcePath: "C:\\Pictures\\$relativePath",
    displayPath: "C:\\Pictures\\$relativePath",
    relativePath: relativePath,
    previewPath: source.previewPath,
    fileSize: source.fileSize,
    createdUnixMs: source.createdUnixMs,
    modifiedUnixMs: source.modifiedUnixMs,
    width: source.width,
    height: source.height,
    previewStatus: source.previewStatus,
    previewIssueCode: source.previewIssueCode,
    previewIssueMessage: source.previewIssueMessage,
    metadataEngineId: source.metadataEngineId,
    metadataEngineVersion: source.metadataEngineVersion,
    captureTime: source.captureTime,
    fileIdentity: source.fileIdentity,
  );
}

LibrarySynchronizationSnapshot _synchronizationSnapshot(BigInt revision) {
  return LibrarySynchronizationSnapshot(
    isRunning: true,
    catalogRevision: revision,
    appliedMutationCount: 1,
    roots: const {},
  );
}

class _TestLibrarySynchronization implements LibrarySynchronization {
  _TestLibrarySynchronization(this._current);

  final StreamController<LibrarySynchronizationSnapshot> _updates =
      StreamController.broadcast(sync: true);
  LibrarySynchronizationSnapshot _current;

  @override
  LibrarySynchronizationSnapshot get current => _current;

  @override
  Future<void> start() async {}

  @override
  Future<void> stop() async {}

  @override
  Stream<LibrarySynchronizationSnapshot> watch() => _updates.stream;

  void publish(LibrarySynchronizationSnapshot snapshot) {
    _current = snapshot;
    _updates.add(snapshot);
  }

  Future<void> dispose() => _updates.close();
}

class _SynchronizationViewerCatalog
    implements
        LibraryCatalog,
        LibraryStableQueryAnchorCatalog,
        LibraryStableAssetCatalog {
  _SynchronizationViewerCatalog(this.snapshot);

  LibrarySnapshot snapshot;
  final List<String> requestedLocationIds = [];
  final List<String?> preferredLocationIds = [];
  Completer<LibraryAsset?>? _heldAssetLookup;

  @override
  Future<LibrarySnapshot> loadAroundAsset({
    required int maxItems,
    required LibraryGalleryQuery query,
    required String requestedLocationId,
    required String anchorAssetId,
    required int fallbackGlobalItemIndex,
  }) async {
    requestedLocationIds.add(requestedLocationId);
    return snapshot;
  }

  @override
  Future<LibraryAsset?> loadAssetById({
    required String assetId,
    String? preferredLocationId,
  }) async {
    preferredLocationIds.add(preferredLocationId);
    final heldAssetLookup = _heldAssetLookup;
    if (heldAssetLookup != null) {
      return heldAssetLookup.future;
    }
    for (final asset in snapshot.assets) {
      if (asset.assetId == assetId) {
        return asset;
      }
    }
    return null;
  }

  @override
  Future<LibrarySnapshot> load({
    required int maxItems,
    required LibraryGalleryQuery query,
    LibraryCatalogCursor? after,
    LibraryCatalogCursor? before,
  }) async => snapshot;

  @override
  Future<LibrarySnapshot> loadAtTime({
    required int maxItems,
    required LibraryGalleryQuery query,
    required LibraryTimeAnchor anchor,
  }) async => snapshot;

  @override
  Future<LibraryTimeline> loadTimeline(LibraryGalleryQuery query) async {
    return LibraryTimeline(
      revision: snapshot.revision,
      queryId: snapshot.queryId,
      totalItems: snapshot.assets.length,
      buckets: [
        LibraryTimeBucket(
          itemCount: snapshot.assets.length,
          aspectRatioSum: snapshot.assets.length * 4 / 3,
        ),
      ],
    );
  }

  @override
  Future<bool> unregisterRoot(String rootId) async => true;

  void holdNextAssetLookup() {
    _heldAssetLookup = Completer<LibraryAsset?>();
  }

  void completeHeldAssetLookup(LibraryAsset? asset) {
    final heldAssetLookup = _heldAssetLookup;
    _heldAssetLookup = null;
    heldAssetLookup?.complete(asset);
  }
}

class _RecordingPlatformActions implements LibraryPlatformActions {
  final List<String> revealedFiles = [];

  @override
  Future<void> copyText(String value) async {}

  @override
  Future<void> revealDirectory(String path) async {}

  @override
  Future<void> revealFile(String path) async {
    revealedFiles.add(path);
  }

  @override
  Future<void> revealLibraryFolder(
    String rootPath,
    String relativePath,
  ) async {}
}
