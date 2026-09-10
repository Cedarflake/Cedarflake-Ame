import "dart:async";
import "dart:io";

import "package:cedarflake_ame/app/presentation/ame_menu.dart";
import "package:cedarflake_ame/features/library/application/library_catalog.dart";
import "package:cedarflake_ame/features/library/application/library_controller.dart";
import "package:cedarflake_ame/features/library/application/library_previewer.dart";
import "package:cedarflake_ame/features/library/application/library_scanner.dart";
import "package:cedarflake_ame/features/library/domain/library_models.dart";
import "package:cedarflake_ame/features/library/domain/library_state.dart";
import "package:cedarflake_ame/features/library/presentation/library_strings.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_photo_tile.dart";
import "package:flutter/gestures.dart";
import "package:flutter/material.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:flutter/services.dart";
import "package:flutter_test/flutter_test.dart";

import "../support/library_query_snapshot_fixture.dart";

void main() {
  test("buckets preview decode widths across small layout changes", () {
    expect(libraryPreviewDecodeWidth(40, 1), 128);
    expect(libraryPreviewDecodeWidth(127, 1), 128);
    expect(libraryPreviewDecodeWidth(129, 1), 256);
    expect(libraryPreviewDecodeWidth(180, 1), 256);
    expect(libraryPreviewDecodeWidth(181, 1), 256);
    expect(libraryPreviewDecodeWidth(129, 2), 512);
    expect(libraryPreviewDecodeWidth(600, 2), 512);
  });

  testWidgets("creates the photo context menu only on demand", (tester) async {
    final asset = _pendingAsset();
    final snapshot = LibrarySnapshot(
      catalogPath: "C:\\AmeData\\ame.sqlite3",
      revision: BigInt.one,
      queryId: "query-1",
      roots: const [],
      assets: [asset],
    );
    LibraryAsset? copiedAsset;

    await tester.pumpWidget(
      ProviderScope(
        overrides: [
          initialLibraryStateProvider.overrideWithValue(
            LibraryState.fromSnapshot(snapshot),
          ),
          libraryCatalogProvider.overrideWithValue(_FakeCatalog(snapshot)),
          libraryScannerProvider.overrideWithValue(const _FakeScanner()),
          libraryPreviewerProvider.overrideWithValue(
            _RecordingPreviewer(asset),
          ),
        ],
        child: MaterialApp(
          home: Scaffold(
            body: LibraryPhotoTile(
              asset: asset,
              width: 160,
              height: 120,
              isSelecting: false,
              isSelected: false,
              onOpen: (_) {},
              onToggleSelection: (_) {},
              onViewInformation: (_) {},
              onCopyPath: (value) => copiedAsset = value,
              onRevealFile: (_) {},
            ),
          ),
        ),
      ),
    );
    await tester.pump();

    expect(find.byType(MenuAnchor), findsNothing);

    await tester.tap(
      find.byType(LibraryPhotoTile),
      buttons: kSecondaryMouseButton,
    );
    await tester.pumpAndSettle();

    expect(find.byType(AmeMenuItemContent), findsNWidgets(4));
    expect(find.byType(MenuAnchor), findsNothing);

    await tester.tap(find.text(LibraryStrings.copyPath));
    await tester.pumpAndSettle();

    expect(copiedAsset, same(asset));
    expect(find.byType(AmeMenuItemContent), findsNothing);

    await tester.sendKeyEvent(LogicalKeyboardKey.contextMenu);
    await tester.pumpAndSettle();

    expect(find.byType(AmeMenuItemContent), findsNWidgets(4));
    await tester.sendKeyEvent(LogicalKeyboardKey.escape);
    await tester.pumpAndSettle();
  });

  testWidgets("shows one immediate progress state for an explicit retry", (
    tester,
  ) async {
    final asset = _failedAsset();
    final snapshot = LibrarySnapshot(
      catalogPath: "C:\\AmeData\\ame.sqlite3",
      revision: BigInt.one,
      queryId: "query-1",
      roots: const [],
      assets: [asset],
    );
    final previewer = _ControlledRetryPreviewer();

    await tester.pumpWidget(
      ProviderScope(
        overrides: [
          initialLibraryStateProvider.overrideWithValue(
            LibraryState.fromSnapshot(snapshot),
          ),
          libraryCatalogProvider.overrideWithValue(_FakeCatalog(snapshot)),
          libraryScannerProvider.overrideWithValue(const _FakeScanner()),
          libraryPreviewerProvider.overrideWithValue(previewer),
        ],
        child: MaterialApp(
          home: Scaffold(
            body: LibraryPhotoTile(
              asset: asset,
              width: 160,
              height: 120,
              isSelecting: false,
              isSelected: false,
              onOpen: (_) {},
              onToggleSelection: (_) {},
              onViewInformation: (_) {},
              onCopyPath: (_) {},
              onRevealFile: (_) {},
            ),
          ),
        ),
      ),
    );
    await tester.pump();

    final retryButton = find.byKey(const Key("preview-retry-location-failed"));
    await tester.tap(retryButton);
    await tester.tap(retryButton);
    await tester.pump();

    expect(previewer.requests, ["location-failed"]);
    expect(
      find.byKey(const Key("preview-retry-progress-location-failed")),
      findsOneWidget,
    );
    expect(find.byType(CircularProgressIndicator), findsOneWidget);
    expect(find.text(LibraryStrings.retryingPreview), findsOneWidget);
    expect(retryButton, findsNothing);

    previewer.complete(
      asset.withPreview(
        previewPath: "",
        width: asset.width,
        height: asset.height,
        previewStatus: LibraryPreviewStatus.failed,
        previewIssueCode: "preview_decode_failed",
        previewIssueMessage: "Could not decode preview",
      ),
    );
    await tester.pump();
    await tester.pump();

    expect(find.byType(CircularProgressIndicator), findsNothing);
    expect(find.text(LibraryStrings.retryingPreview), findsNothing);
    expect(retryButton, findsOneWidget);
  });

  testWidgets("explains that an unproven root requires a library update", (
    tester,
  ) async {
    final asset = _failedAsset();
    final snapshot = LibrarySnapshot(
      catalogPath: "C:\\AmeData\\ame.sqlite3",
      revision: BigInt.one,
      queryId: "query-1",
      roots: const [],
      assets: [asset],
    );

    await tester.pumpWidget(
      ProviderScope(
        overrides: [
          initialLibraryStateProvider.overrideWithValue(
            LibraryState.fromSnapshot(snapshot),
          ),
          libraryCatalogProvider.overrideWithValue(_FakeCatalog(snapshot)),
          libraryScannerProvider.overrideWithValue(const _FakeScanner()),
          libraryPreviewerProvider.overrideWithValue(
            const _UnprovenRootPreviewer(),
          ),
        ],
        child: MaterialApp(
          home: Scaffold(
            body: LibraryPhotoTile(
              asset: asset,
              width: 240,
              height: 160,
              isSelecting: false,
              isSelected: false,
              onOpen: (_) {},
              onToggleSelection: (_) {},
              onViewInformation: (_) {},
              onCopyPath: (_) {},
              onRevealFile: (_) {},
            ),
          ),
        ),
      ),
    );
    await tester.pump();

    await tester.tap(find.byKey(const Key("preview-retry-location-failed")));
    await tester.pump();
    await tester.pump();

    expect(find.text(LibraryStrings.previewUpdateRequired), findsOneWidget);
    expect(
      find.byKey(const Key("preview-retry-location-failed")),
      findsOneWidget,
    );
  });

  testWidgets(
    "keeps update guidance visible when a ready preview is corrupt on an unproven root",
    (tester) async {
      final directory = Directory.systemTemp.createTempSync(
        "ame-unproven-ready-preview-",
      );
      addTearDown(() => directory.deleteSync(recursive: true));
      final previewFile = File(
        "${directory.path}${Platform.pathSeparator}broken.jpg",
      );
      previewFile.writeAsBytesSync(const [0xFF, 0xD8, 0xFF]);
      final asset = _readyAsset(previewFile.path);
      final snapshot = LibrarySnapshot(
        catalogPath: "C:\\AmeData\\ame.sqlite3",
        revision: BigInt.one,
        queryId: "query-1",
        roots: const [],
        assets: [asset],
      );

      await tester.pumpWidget(
        ProviderScope(
          overrides: [
            initialLibraryStateProvider.overrideWithValue(
              LibraryState.fromSnapshot(snapshot),
            ),
            libraryCatalogProvider.overrideWithValue(_FakeCatalog(snapshot)),
            libraryScannerProvider.overrideWithValue(const _FakeScanner()),
            libraryPreviewerProvider.overrideWithValue(
              const _UnprovenRootPreviewer(),
            ),
          ],
          child: MaterialApp(
            home: Scaffold(
              body: LibraryPhotoTile(
                asset: asset,
                width: 240,
                height: 160,
                isSelecting: false,
                isSelected: false,
                onOpen: (_) {},
                onToggleSelection: (_) {},
                onViewInformation: (_) {},
                onCopyPath: (_) {},
                onRevealFile: (_) {},
              ),
            ),
          ),
        ),
      );
      for (
        var attempt = 0;
        attempt < 20 &&
            find.text(LibraryStrings.previewUpdateRequired).evaluate().isEmpty;
        attempt++
      ) {
        await tester.runAsync(() async {
          await Future<void>.delayed(const Duration(milliseconds: 25));
        });
        await tester.pump();
      }

      expect(find.text(LibraryStrings.previewUpdateRequired), findsOneWidget);
      expect(
        find.bySemanticsLabel(LibraryStrings.previewUpdateRequired),
        findsOneWidget,
      );
      expect(
        find.byKey(const Key("preview-retry-location-ready")),
        findsOneWidget,
      );
      await tester.tap(find.byKey(const Key("preview-retry-location-ready")));
      await tester.pump();
      await tester.pump();
      expect(find.text(LibraryStrings.previewUpdateRequired), findsOneWidget);
    },
    timeout: const Timeout(Duration(seconds: 30)),
  );

  testWidgets(
    "repairs a ready preview after Flutter cannot decode it",
    (tester) async {
      final directory = Directory.systemTemp.createTempSync(
        "ame-preview-repair-",
      );
      addTearDown(() => directory.deleteSync(recursive: true));
      final previewFile = File(
        "${directory.path}${Platform.pathSeparator}broken.jpg",
      );
      previewFile.writeAsBytesSync(const [0xFF, 0xD8, 0xFF]);
      final asset = _readyAsset(previewFile.path);
      final snapshot = LibrarySnapshot(
        catalogPath: "C:\\AmeData\\ame.sqlite3",
        revision: BigInt.one,
        queryId: "query-1",
        roots: const [],
        assets: [asset],
      );
      final previewer = _RecordingPreviewer(asset);

      await tester.pumpWidget(
        ProviderScope(
          overrides: [
            initialLibraryStateProvider.overrideWithValue(
              LibraryState.fromSnapshot(snapshot),
            ),
            libraryCatalogProvider.overrideWithValue(_FakeCatalog(snapshot)),
            libraryScannerProvider.overrideWithValue(const _FakeScanner()),
            libraryPreviewerProvider.overrideWithValue(previewer),
          ],
          child: MaterialApp(
            home: Scaffold(
              body: LibraryPhotoTile(
                asset: asset,
                width: 160,
                height: 120,
                isSelecting: false,
                isSelected: false,
                onOpen: (_) {},
                onToggleSelection: (_) {},
                onViewInformation: (_) {},
                onCopyPath: (_) {},
                onRevealFile: (_) {},
              ),
            ),
          ),
        ),
      );
      for (
        var attempt = 0;
        attempt < 20 && previewer.requests.isEmpty;
        attempt++
      ) {
        await tester.runAsync(() async {
          await Future<void>.delayed(const Duration(milliseconds: 25));
        });
        await tester.pump();
      }
      await tester.pump();

      expect(previewer.requests, [(locationId: "location-ready", retry: true)]);
      expect(find.byKey(const Key("preview-retry-location-ready")), findsOne);
      await tester.pumpWidget(const SizedBox.shrink());
      await tester.pump();
    },
    timeout: const Timeout(Duration(seconds: 30)),
  );
}

LibraryAsset _readyAsset(String previewPath) {
  return LibraryAsset(
    assetId: "asset-ready",
    locationId: "location-ready",
    rootId: "root-1",
    activeScanId: "scan-1",
    sourceRevision: null,
    sourceGeneration: BigInt.one,
    sourcePath: "C:\\Pictures\\ready.jpg",
    displayPath: "C:\\Pictures\\ready.jpg",
    relativePath: "ready.jpg",
    previewPath: previewPath,
    fileSize: BigInt.one,
    modifiedUnixMs: 1,
    width: 160,
    height: 120,
    previewStatus: LibraryPreviewStatus.ready,
  );
}

LibraryAsset _pendingAsset() {
  return LibraryAsset(
    assetId: "asset-pending",
    locationId: "location-pending",
    rootId: "root-1",
    activeScanId: "scan-1",
    sourceRevision: null,
    sourceGeneration: BigInt.one,
    sourcePath: "C:\\Pictures\\pending.jpg",
    displayPath: "C:\\Pictures\\pending.jpg",
    relativePath: "pending.jpg",
    previewPath: "",
    fileSize: BigInt.one,
    modifiedUnixMs: 1,
    width: 160,
    height: 120,
    previewStatus: LibraryPreviewStatus.pending,
  );
}

LibraryAsset _failedAsset() {
  return LibraryAsset(
    assetId: "asset-failed",
    locationId: "location-failed",
    rootId: "root-1",
    activeScanId: "scan-1",
    sourceRevision: null,
    sourceGeneration: BigInt.one,
    sourcePath: "C:\\Pictures\\failed.jpg",
    displayPath: "C:\\Pictures\\failed.jpg",
    relativePath: "failed.jpg",
    previewPath: "",
    fileSize: BigInt.one,
    modifiedUnixMs: 1,
    width: 160,
    height: 120,
    previewStatus: LibraryPreviewStatus.failed,
    previewIssueCode: "preview_decode_failed",
    previewIssueMessage: "Could not decode preview",
  );
}

class _RecordingPreviewer implements LibraryPreviewer {
  _RecordingPreviewer(this.asset);

  final LibraryAsset asset;
  final requests = <({String locationId, bool retry})>[];

  @override
  Future<LibraryAsset> materialize({
    required String locationId,
    required String expectedRootId,
    required String expectedScanId,
    required LibrarySourceRevisionEvidence? expectedSourceRevision,
    required BigInt expectedSourceGeneration,
    required int previewEdge,
    bool force = false,
    Iterable<String> protectedLocationIds = const [],
  }) {
    requests.add((locationId: locationId, retry: force));
    return Future.value(
      asset.withPreview(
        previewPath: asset.previewPath,
        width: asset.width,
        height: asset.height,
        previewStatus: LibraryPreviewStatus.failed,
        previewIssueCode: "preview_repair_failed",
        previewIssueMessage: "Repair failed",
      ),
    );
  }
}

class _ControlledRetryPreviewer implements LibraryPreviewer {
  final requests = <String>[];
  final Completer<LibraryAsset> _completion = Completer<LibraryAsset>();

  @override
  Future<LibraryAsset> materialize({
    required String locationId,
    required String expectedRootId,
    required String expectedScanId,
    required LibrarySourceRevisionEvidence? expectedSourceRevision,
    required BigInt expectedSourceGeneration,
    required int previewEdge,
    bool force = false,
    Iterable<String> protectedLocationIds = const [],
  }) {
    requests.add(locationId);
    return _completion.future;
  }

  void complete(LibraryAsset asset) => _completion.complete(asset);
}

class _UnprovenRootPreviewer implements LibraryPreviewer {
  const _UnprovenRootPreviewer();

  @override
  Future<LibraryAsset> materialize({
    required String locationId,
    required String expectedRootId,
    required String expectedScanId,
    required LibrarySourceRevisionEvidence? expectedSourceRevision,
    required BigInt expectedSourceGeneration,
    required int previewEdge,
    bool force = false,
    Iterable<String> protectedLocationIds = const [],
  }) {
    return Future.error(
      const LibraryPreviewFailure(
        code: "preview_root_identity_unproven",
        message: "root proof is unavailable",
      ),
    );
  }
}

class _FakeCatalog with LibraryQuerySnapshotFixture implements LibraryCatalog {
  const _FakeCatalog(this.snapshot);

  final LibrarySnapshot snapshot;

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
          aspectRatioSum: snapshot.assets.length.toDouble(),
        ),
      ],
    );
  }

  @override
  Future<bool> unregisterRoot(String rootId) async => false;
}

class _FakeScanner implements LibraryScanner {
  @override
  Future<void> cancelRetainedScan(String scanId) async {
    throw StateError("This fixture has no retained cancellation command");
  }

  const _FakeScanner();

  @override
  bool cancel(String scanId) => false;

  @override
  Future<RecoverableLibraryScan?> loadPausedScan() async => null;

  @override
  Future<RecoverableLibraryScan?> loadRecoverableScan() async => null;

  @override
  bool pause(String scanId) => false;

  @override
  bool suspend(String scanId) => false;

  @override
  Stream<LibraryScanUpdate> scan({
    required String scanId,
    required String rootPath,
    required int? itemLimit,
    required int? entryLimit,
    required int previewEdge,
  }) {
    return const Stream.empty();
  }

  @override
  Stream<LibraryScanUpdate> resume({
    required String scanId,
    required String rootPath,
    required int? itemLimit,
    required int? entryLimit,
    required int previewEdge,
  }) {
    return const Stream.empty();
  }
}
