import "dart:async";

import "package:cedarflake_ame/app/ame_app.dart";
import "package:cedarflake_ame/app/presentation/ame_menu.dart";
import "package:cedarflake_ame/features/library/application/library_catalog.dart";
import "package:cedarflake_ame/features/library/application/library_controller.dart";
import "package:cedarflake_ame/features/library/application/library_scanner.dart";
import "package:cedarflake_ame/features/library/domain/library_models.dart";
import "package:cedarflake_ame/features/library/domain/library_state.dart";
import "package:cedarflake_ame/features/library/presentation/library_strings.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_gallery_header.dart";
import "package:flutter/gestures.dart";
import "package:flutter/material.dart";
import "package:flutter/services.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:flutter_test/flutter_test.dart";

import "../support/library_query_snapshot_fixture.dart";
import "support/library_screen_fixture.dart";

void main() {
  testWidgets("opens one source menu from overflow or secondary click", (
    tester,
  ) async {
    tester.view.physicalSize = const Size(1280, 800);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);

    await tester.pumpWidget(
      ProviderScope(
        overrides: [
          initialLibraryStateProvider.overrideWithValue(
            populatedScreenState(totalItems: 1),
          ),
        ],
        child: const AmeApp(),
      ),
    );
    await tester.pump();

    final sourceMore = find.byKey(const ValueKey("source-more-root-1"));
    final sourceMoreRect = tester.getRect(sourceMore);
    await tester.tap(sourceMore);
    await tester.pumpAndSettle();
    expect(find.text("更新图库"), findsOneWidget);
    expect(find.text("在文件资源管理器中打开"), findsOneWidget);
    expect(find.text("从 Ame 中移除"), findsOneWidget);
    expect(find.text(LibraryStrings.relocateLibrary), findsOneWidget);
    expect(find.byType(AmeMenuItemContent), findsNWidgets(4));
    final updateMenuItem = find.ancestor(
      of: find.text("更新图库"),
      matching: find.byWidgetPredicate((widget) => widget is PopupMenuItem),
    );
    expect(
      tester.getRect(updateMenuItem).right,
      closeTo(sourceMoreRect.right, 1),
    );
    await tester.tapAt(sourceMoreRect.center);
    await tester.pumpAndSettle();
    expect(find.text("更新图库"), findsNothing);

    await tester.sendKeyEvent(LogicalKeyboardKey.escape);
    await tester.pumpAndSettle();
    await tester.tap(find.text("Pictures"), buttons: kSecondaryMouseButton);
    await tester.pumpAndSettle();
    expect(find.text("从 Ame 中移除"), findsOneWidget);
  });

  testWidgets("shows root removal feedback while deferred catalog work runs", (
    tester,
  ) async {
    tester.view.physicalSize = const Size(1280, 800);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);
    final initialState = populatedScreenState(totalItems: 1);
    var feedbackWasLaidOutBeforeCatalogEntry = false;
    final catalog = _DeferredScreenRemovalCatalog(
      initialState,
      onUnregister: () {
        final feedback = find.text("正在从 Ame 中移除“Pictures”…");
        feedbackWasLaidOutBeforeCatalogEntry =
            feedback.evaluate().length == 1 &&
            tester.getSize(feedback).height > 0;
      },
    );

    await tester.pumpWidget(
      ProviderScope(
        overrides: [
          initialLibraryStateProvider.overrideWithValue(initialState),
          libraryCatalogProvider.overrideWithValue(catalog),
        ],
        child: const AmeApp(),
      ),
    );
    await tester.pump();

    await tester.tap(find.byKey(const Key("library-more-menu")));
    await tester.pumpAndSettle();
    await tester.tap(find.text("全选"));
    await tester.pumpAndSettle();
    expect(find.text("已选择 1 个项目"), findsOneWidget);

    await tester.tap(find.byKey(const ValueKey("source-more-root-1")));
    await tester.pumpAndSettle();
    await tester.tap(find.text(LibraryStrings.removeFromAme));
    await tester.pumpAndSettle();
    expect(find.text("从 Ame 中移除目录？"), findsOneWidget);

    await tester.tap(find.text(LibraryStrings.removeFromAme));
    expect(catalog.unregisteredRootIds, isEmpty);
    await tester.pump();

    expect(find.byKey(const Key("library-task-surface")), findsOneWidget);
    expect(find.text("正在从 Ame 中移除“Pictures”…"), findsOneWidget);
    expect(find.text("原目录和图片都会保留。"), findsOneWidget);
    expect(catalog.unregisteredRootIds, ["root-1"]);
    expect(feedbackWasLaidOutBeforeCatalogEntry, isTrue);

    catalog.completeRemoval(false);
    await tester.pumpAndSettle();
    expect(find.textContaining("已选择"), findsNothing);
    expect(
      tester
          .widget<LibraryGalleryHeader>(find.byType(LibraryGalleryHeader))
          .isSelecting,
      isFalse,
    );
  });

  testWidgets(
    "disposal before removal feedback cancels prepared catalog work",
    (tester) async {
      tester.view.physicalSize = const Size(1280, 800);
      tester.view.devicePixelRatio = 1;
      addTearDown(tester.view.resetPhysicalSize);
      addTearDown(tester.view.resetDevicePixelRatio);
      final initialState = populatedScreenState(totalItems: 1);
      final catalog = _DeferredScreenRemovalCatalog(initialState);
      await tester.pumpWidget(
        ProviderScope(
          overrides: [
            initialLibraryStateProvider.overrideWithValue(initialState),
            libraryCatalogProvider.overrideWithValue(catalog),
          ],
          child: const AmeApp(),
        ),
      );
      await tester.pump();
      await tester.tap(find.byKey(const ValueKey("source-more-root-1")));
      await tester.pumpAndSettle();
      await tester.tap(find.text(LibraryStrings.removeFromAme));
      await tester.pumpAndSettle();
      await tester.tap(find.text(LibraryStrings.removeFromAme));
      expect(catalog.unregisteredRootIds, isEmpty);

      await tester.pumpWidget(const SizedBox.shrink());
      await tester.pump();

      expect(catalog.unregisteredRootIds, isEmpty);
      expect(tester.takeException(), isNull);
    },
  );

  testWidgets(
    "clears selection when root removal and first-page reload finish together",
    (tester) async {
      tester.view.physicalSize = const Size(1280, 800);
      tester.view.devicePixelRatio = 1;
      addTearDown(tester.view.resetPhysicalSize);
      addTearDown(tester.view.resetDevicePixelRatio);
      final baseState = populatedScreenState(totalItems: 2);
      const remainingRoot = LibraryRoot(
        id: "root-2",
        path: "D:\\Archive",
        displayPath: "D:\\Archive",
        activeScanId: "scan-2",
        createdUnixMs: 2,
        assetCount: 1,
        issueCount: 0,
        availability: LibraryRootAvailability.available,
      );
      final remainingAsset = _assetForRoot(root: remainingRoot, ordinal: 2);
      final initialState = baseState.copyWith(
        roots: [...baseState.roots, remainingRoot],
        assets: [...baseState.assets, remainingAsset],
        timeline: LibraryTimeline(
          revision: baseState.catalogRevision ?? BigInt.one,
          queryId: baseState.queryId,
          totalItems: 2,
          buckets: const [LibraryTimeBucket(itemCount: 2, aspectRatioSum: 2)],
        ),
      );
      final catalog = _ImmediateScreenRemovalCatalog(
        afterRemoval: LibrarySnapshot(
          catalogPath: initialState.catalogPath ?? "catalog.sqlite3",
          revision: BigInt.from(2),
          queryId: "query-after-removal",
          roots: const [remainingRoot],
          assets: [remainingAsset],
        ),
      );

      await tester.pumpWidget(
        ProviderScope(
          overrides: [
            initialLibraryStateProvider.overrideWithValue(initialState),
            libraryCatalogProvider.overrideWithValue(catalog),
          ],
          child: const AmeApp(),
        ),
      );
      await tester.pump();

      await tester.tap(find.byKey(const Key("library-more-menu")));
      await tester.pumpAndSettle();
      await tester.tap(find.text("全选"));
      await tester.pumpAndSettle();
      expect(find.text("已选择 2 个项目"), findsOneWidget);

      await tester.tap(find.byKey(const ValueKey("source-more-root-1")));
      await tester.pumpAndSettle();
      await tester.tap(find.text(LibraryStrings.removeFromAme));
      await tester.pumpAndSettle();
      await tester.tap(find.text(LibraryStrings.removeFromAme));
      await tester.pumpAndSettle();

      final header = tester.widget<LibraryGalleryHeader>(
        find.byType(LibraryGalleryHeader),
      );
      expect(catalog.unregisteredRootIds, ["root-1"]);
      expect(header.isSelecting, isFalse);
      expect(header.selectedCount, 0);
      expect(find.byKey(const Key("library-selection-toolbar")), findsNothing);
      expect(find.textContaining("已选择"), findsNothing);
      expect(find.text("Archive"), findsOneWidget);
    },
  );

  testWidgets("removal replaces unacknowledged import feedback", (
    tester,
  ) async {
    tester.view.physicalSize = const Size(1280, 800);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);
    final state = populatedScreenState(totalItems: 1).copyWith(
      scanId: "scan-completed",
      rootPath: "C:\\Pictures",
      displayRootPath: "C:\\Pictures",
      taskKind: LibraryTaskKind.import,
      visitedEntries: 128,
      stagedAssetCount: 1,
    );
    final catalog = _ImmediateScreenRemovalCatalog(
      afterRemoval: LibrarySnapshot(
        catalogPath: state.catalogPath ?? "catalog.sqlite3",
        revision: BigInt.from(2),
        queryId: "query-after-removal",
        roots: const [],
        assets: const [],
      ),
    );

    await tester.pumpWidget(
      ProviderScope(
        overrides: [
          initialLibraryStateProvider.overrideWithValue(state),
          libraryCatalogProvider.overrideWithValue(catalog),
          libraryScannerProvider.overrideWithValue(const NoopLibraryScanner()),
        ],
        child: const AmeApp(),
      ),
    );
    await tester.pump();
    expect(find.text("添加完成"), findsOneWidget);

    await tester.tap(find.byKey(const ValueKey("source-more-root-1")));
    await tester.pumpAndSettle();
    await tester.tap(find.text(LibraryStrings.removeFromAme));
    await tester.pumpAndSettle();
    await tester.tap(find.text(LibraryStrings.removeFromAme));
    await tester.pumpAndSettle();

    expect(catalog.unregisteredRootIds, ["root-1"]);
    expect(find.text("添加完成"), findsNothing);
    expect(find.byKey(const Key("library-task-surface")), findsNothing);
    expect(find.text("已从 Ame 中移除“Pictures”"), findsOneWidget);
  });

  testWidgets("routes source update to the configured-root selection dialog", (
    tester,
  ) async {
    tester.view.physicalSize = const Size(1280, 800);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);

    await tester.pumpWidget(
      ProviderScope(
        overrides: [
          initialLibraryStateProvider.overrideWithValue(
            populatedScreenState(totalItems: 1),
          ),
        ],
        child: const AmeApp(),
      ),
    );
    await tester.pump();

    await tester.tap(find.byKey(const ValueKey("source-more-root-1")));
    await tester.pumpAndSettle();
    await tester.tap(find.text("更新图库"));
    await tester.pumpAndSettle();

    expect(find.byKey(const Key("library-update-dialog")), findsOneWidget);
    expect(
      tester
          .widget<CheckboxListTile>(
            find.byKey(const ValueKey("library-update-root-root-1")),
          )
          .value,
      isTrue,
    );
  });

  testWidgets("submits two configured roots from one source update action", (
    tester,
  ) async {
    tester.view.physicalSize = const Size(1280, 800);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);
    final baseState = populatedScreenState(totalItems: 2);
    final secondRoot = LibraryRoot(
      id: "root-2",
      path: "D:\\Archive",
      displayPath: "D:\\Archive",
      activeScanId: "scan-2",
      createdUnixMs: 2,
      assetCount: 1,
      issueCount: 0,
      availability: LibraryRootAvailability.available,
    );
    final scanner = _RecordingUpdateScanner();
    addTearDown(scanner.dispose);

    await tester.pumpWidget(
      ProviderScope(
        overrides: [
          initialLibraryStateProvider.overrideWithValue(
            baseState.copyWith(roots: [...baseState.roots, secondRoot]),
          ),
          libraryScannerProvider.overrideWithValue(scanner),
        ],
        child: const AmeApp(),
      ),
    );
    await tester.pump();

    await tester.tap(find.byKey(const ValueKey("source-more-root-1")));
    await tester.pumpAndSettle();
    await tester.tap(find.text(LibraryStrings.updateLibrary));
    await tester.pumpAndSettle();
    await tester.tap(find.byKey(const ValueKey("library-update-root-root-2")));
    await tester.pump();
    await tester.tap(find.byKey(const Key("library-update-confirm")));
    await tester.pump();
    await tester.pump(const Duration(milliseconds: 250));

    expect(scanner.startedRootPaths, ["C:\\Pictures", "D:\\Archive"]);
    expect(
      find.byKey(const Key("library-update-task-surface")),
      findsOneWidget,
    );
    expect(find.text("图库更新"), findsOneWidget);
    expect(
      find.byKey(const ValueKey("library-update-task-root-1")),
      findsOneWidget,
    );
    expect(
      find.byKey(const ValueKey("library-update-task-root-2")),
      findsOneWidget,
    );
    expect(find.textContaining("正在添加“"), findsNothing);
  });
}

LibraryAsset _assetForRoot({required LibraryRoot root, required int ordinal}) {
  return LibraryAsset(
    assetId: "asset-$ordinal",
    locationId: "location-$ordinal",
    rootId: root.id,
    activeScanId: root.activeScanId ?? "scan-$ordinal",
    sourceRevision: null,
    sourceGeneration: BigInt.one,
    sourcePath: "${root.path}\\image-$ordinal.png",
    displayPath: "${root.displayPath}\\image-$ordinal.png",
    relativePath: "image-$ordinal.png",
    previewPath: "C:\\Missing\\image-$ordinal.jpg",
    fileSize: BigInt.one,
    modifiedUnixMs: ordinal,
    width: 4,
    height: 3,
  );
}

class _DeferredScreenRemovalCatalog
    with LibraryQuerySnapshotFixture
    implements LibraryCatalog {
  _DeferredScreenRemovalCatalog(this.state, {this.onUnregister});

  final LibraryState state;
  final VoidCallback? onUnregister;
  final List<String> unregisteredRootIds = [];
  final Completer<bool> _removal = Completer<bool>();

  @override
  Future<LibrarySnapshot> load({
    required int maxItems,
    required LibraryGalleryQuery query,
    LibraryCatalogCursor? after,
    LibraryCatalogCursor? before,
  }) async => _snapshot;

  @override
  Future<LibrarySnapshot> loadAtTime({
    required int maxItems,
    required LibraryGalleryQuery query,
    required LibraryTimeAnchor anchor,
  }) async => _snapshot;

  @override
  Future<LibraryTimeline> loadTimeline(LibraryGalleryQuery query) async {
    return LibraryTimeline(
      revision: state.catalogRevision ?? BigInt.one,
      queryId: state.queryId,
      totalItems: state.assets.length,
      buckets: const [],
    );
  }

  @override
  Future<bool> unregisterRoot(String rootId) {
    onUnregister?.call();
    unregisteredRootIds.add(rootId);
    return _removal.future;
  }

  LibrarySnapshot get _snapshot => LibrarySnapshot(
    catalogPath: state.catalogPath ?? "catalog.sqlite3",
    revision: state.catalogRevision ?? BigInt.one,
    queryId: state.queryId,
    roots: state.roots,
    assets: state.assets,
    previousCursor: state.previousCursor,
    nextCursor: state.nextCursor,
  );

  void completeRemoval(bool removed) {
    _removal.complete(removed);
  }
}

class _ImmediateScreenRemovalCatalog
    with LibraryQuerySnapshotFixture
    implements LibraryCatalog {
  _ImmediateScreenRemovalCatalog({required this.afterRemoval});

  final LibrarySnapshot afterRemoval;
  final List<String> unregisteredRootIds = [];

  @override
  Future<LibrarySnapshot> load({
    required int maxItems,
    required LibraryGalleryQuery query,
    LibraryCatalogCursor? after,
    LibraryCatalogCursor? before,
  }) async => afterRemoval;

  @override
  Future<LibrarySnapshot> loadAtTime({
    required int maxItems,
    required LibraryGalleryQuery query,
    required LibraryTimeAnchor anchor,
  }) async => afterRemoval;

  @override
  Future<LibraryTimeline> loadTimeline(LibraryGalleryQuery query) async {
    return LibraryTimeline(
      revision: afterRemoval.revision,
      queryId: afterRemoval.queryId,
      totalItems: afterRemoval.assets.length,
      buckets: [
        if (afterRemoval.assets.isNotEmpty)
          LibraryTimeBucket(
            itemCount: afterRemoval.assets.length,
            aspectRatioSum: afterRemoval.assets.length.toDouble(),
          ),
      ],
    );
  }

  @override
  Future<bool> unregisterRoot(String rootId) async {
    unregisteredRootIds.add(rootId);
    return true;
  }
}

class _RecordingUpdateScanner implements LibraryScanner {
  @override
  Future<void> cancelRetainedScan(String scanId) async {
    throw StateError("This fixture has no retained cancellation command");
  }

  final List<String> startedRootPaths = [];
  final Map<String, StreamController<LibraryScanUpdate>> _controllers = {};

  @override
  bool cancel(String scanId) => _controllers.containsKey(scanId);

  @override
  Future<RecoverableLibraryScan?> loadPausedScan() async => null;

  @override
  Future<RecoverableLibraryScan?> loadRecoverableScan() async => null;

  @override
  bool pause(String scanId) => false;

  @override
  bool suspend(String scanId) => _controllers.containsKey(scanId);

  @override
  Stream<LibraryScanUpdate> scan({
    required String scanId,
    required String rootPath,
    required int? itemLimit,
    required int? entryLimit,
    required int previewEdge,
  }) {
    startedRootPaths.add(rootPath);
    // The controller is closed by the scanner teardown registered by this test.
    // ignore: close_sinks
    final controller = StreamController<LibraryScanUpdate>();
    _controllers[scanId] = controller;
    controller.add(LibraryScanStarted(scanId: scanId, rootPath: rootPath));
    return controller.stream;
  }

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

  Future<void> dispose() async {
    final controllers = _controllers.values.toList(growable: false);
    _controllers.clear();
    await Future.wait(controllers.map((controller) => controller.close()));
  }
}
