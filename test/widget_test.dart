import "package:cedarflake_ame/app/ame_app.dart";
import "package:cedarflake_ame/adapters/windows_library_platform_actions.dart";
import "package:cedarflake_ame/features/library/application/library_catalog.dart";
import "package:cedarflake_ame/features/library/application/library_controller.dart";
import "package:cedarflake_ame/features/library/application/library_platform_actions.dart";
import "package:cedarflake_ame/features/library/domain/library_folder_models.dart";
import "package:cedarflake_ame/features/library/domain/library_models.dart";
import "package:cedarflake_ame/features/library/domain/library_state.dart";
import "package:cedarflake_ame/features/library/presentation/library_strings.dart";
import "package:flutter/gestures.dart";
import "package:flutter/material.dart";
import "package:flutter/services.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:flutter_test/flutter_test.dart";

void main() {
  testWidgets("shows the unified empty library shell", (tester) async {
    tester.view.physicalSize = const Size(1280, 800);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);

    await tester.pumpWidget(const ProviderScope(child: AmeApp()));

    expect(find.text("图库"), findsNWidgets(2));
    expect(find.byKey(const Key("library-empty-state")), findsOneWidget);
    expect(find.text("Read-only validation"), findsNothing);
    expect(find.byKey(const Key("library-task-activity-button")), findsNothing);
    expect(find.byKey(const Key("library-import-button")), findsOneWidget);
    expect(
      find.ancestor(
        of: find.byKey(const Key("library-import-button")),
        matching: find.byType(AppBar),
      ),
      findsNothing,
    );
  });

  testWidgets("shows scan controls in temporary import feedback", (
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
            const LibraryState(
              status: LibraryStatus.scanning,
              scanId: "scan-1",
              rootPath: "C:\\Pictures",
              visitedEntries: 128,
              stagedAssetCount: 64,
            ),
          ),
        ],
        child: const AmeApp(),
      ),
    );

    expect(find.text("正在添加文件夹“Pictures”…"), findsOneWidget);
    expect(find.text("已检查 128 个文件 · 已找到 64 张图片"), findsOneWidget);
    expect(find.byKey(const Key("library-pause-button")), findsOneWidget);
    expect(find.byKey(const Key("library-cancel-button")), findsOneWidget);
    expect(find.byKey(const Key("library-task-activity-button")), findsNothing);
  });

  testWidgets("connects source search and Material sort to one gallery query", (
    tester,
  ) async {
    tester.view.physicalSize = const Size(1280, 800);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);
    final initialState = _populatedState(totalItems: 1);
    final catalog = _RecordingQueryCatalog(
      LibrarySnapshot(
        catalogPath: initialState.catalogPath ?? "",
        revision: initialState.catalogRevision ?? BigInt.zero,
        queryId: "query-result",
        roots: initialState.roots,
        assets: initialState.assets,
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

    await tester.tap(find.byKey(const ValueKey("source-title-root-1")));
    await tester.pumpAndSettle();
    expect(catalog.loadQueries.last.rootId, "root-1");

    await tester.enterText(find.byKey(const Key("library-search")), "one");
    await tester.pump(const Duration(milliseconds: 300));
    await tester.pumpAndSettle();
    expect(catalog.loadQueries.last.searchText, "one");
    expect(catalog.loadQueries.last.rootId, "root-1");

    await tester.tap(find.byKey(const Key("library-sort-menu")));
    await tester.pumpAndSettle();
    await tester.tap(find.text(LibraryStrings.fileName).last);
    await tester.pumpAndSettle();
    expect(catalog.loadQueries.last.sortKey, LibraryGallerySortKey.fileName);
    expect(catalog.loadQueries.last.searchText, "one");
    expect(catalog.loadQueries.last.rootId, "root-1");
    expect(find.text(LibraryStrings.unknownCaptureDate), findsNothing);
  });

  testWidgets(
    "uses folder rows as gallery scopes and keeps Explorer in the context menu",
    (tester) async {
      tester.view.physicalSize = const Size(1280, 800);
      tester.view.devicePixelRatio = 1;
      addTearDown(tester.view.resetPhysicalSize);
      addTearDown(tester.view.resetDevicePixelRatio);
      final initialState = _populatedState(totalItems: 3);
      final queryCatalog = _RecordingQueryCatalog(
        LibrarySnapshot(
          catalogPath: initialState.catalogPath ?? "",
          revision: initialState.catalogRevision ?? BigInt.zero,
          queryId: "folder-query-result",
          roots: initialState.roots,
          assets: initialState.assets,
        ),
      );
      final folderCatalog = _RecordingFolderCatalog();
      final platformActions = _RecordingPlatformActions();

      await tester.pumpWidget(
        ProviderScope(
          overrides: [
            initialLibraryStateProvider.overrideWithValue(initialState),
            libraryCatalogProvider.overrideWithValue(queryCatalog),
            libraryFolderCatalogProvider.overrideWithValue(folderCatalog),
            libraryPlatformActionsProvider.overrideWithValue(platformActions),
          ],
          child: const AmeApp(),
        ),
      );
      await tester.pump();

      await tester.tap(find.byKey(const ValueKey("source-expand-root-1")));
      await tester.pumpAndSettle();
      expect(
        find.byKey(const ValueKey("folder-title-root-1-Album")),
        findsOneWidget,
      );
      expect(folderCatalog.requests, [
        (rootId: "root-1", parentRelativePath: ""),
      ]);

      await tester.tap(find.byKey(const ValueKey("folder-title-root-1-Album")));
      await tester.pumpAndSettle();
      expect(queryCatalog.loadQueries.last.rootId, "root-1");
      expect(queryCatalog.loadQueries.last.folderRelativePath, "Album");
      expect(queryCatalog.loadQueries.last.includeDescendants, isTrue);
      expect(platformActions.openedLibraryFolders, isEmpty);
      expect(find.text("Album"), findsWidgets);

      await tester.tap(
        find.byKey(const ValueKey("folder-expand-root-1-Album")),
      );
      await tester.pumpAndSettle();
      expect(
        find.byKey(const ValueKey("folder-title-root-1-Album/Sub")),
        findsOneWidget,
      );

      await tester.tap(
        find.byKey(const ValueKey("folder-title-root-1-Album/Sub")),
      );
      await tester.pumpAndSettle();
      expect(queryCatalog.loadQueries.last.folderRelativePath, "Album/Sub");
      expect(platformActions.openedLibraryFolders, isEmpty);

      await tester.tap(
        find.byKey(const ValueKey("folder-title-root-1-Album/Sub")),
        buttons: kSecondaryMouseButton,
      );
      await tester.pumpAndSettle();
      await tester.tap(find.text(LibraryStrings.openInExplorer).last);
      await tester.pumpAndSettle();
      expect(platformActions.openedLibraryFolders, [
        (rootPath: "C:\\Pictures", relativePath: "Album/Sub"),
      ]);
    },
  );

  testWidgets("aligns a pending source with existing source rows", (
    tester,
  ) async {
    tester.view.physicalSize = const Size(1280, 800);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);
    final state = _populatedState(totalItems: 1).copyWith(
      status: LibraryStatus.scanning,
      scanId: "scan-pending",
      rootPath: "C:\\Documents",
      visitedEntries: 12,
      stagedAssetCount: 3,
    );

    await tester.pumpWidget(
      ProviderScope(
        overrides: [initialLibraryStateProvider.overrideWithValue(state)],
        child: const AmeApp(),
      ),
    );
    await tester.pump();

    final sourceIcon = find.byKey(const ValueKey("source-icon-root-1"));
    final pendingIcon = find.byKey(const Key("pending-source-icon"));
    final sourceTitle = find.byKey(const ValueKey("source-title-root-1"));
    final pendingTitle = find.byKey(const Key("pending-source-title"));
    expect(sourceIcon, findsOneWidget);
    expect(pendingIcon, findsOneWidget);
    expect(find.byKey(const Key("pending-source-progress")), findsOneWidget);
    expect(
      tester.getTopLeft(sourceIcon).dx,
      closeTo(tester.getTopLeft(pendingIcon).dx, 0.1),
    );
    expect(
      tester.getTopLeft(sourceTitle).dx,
      closeTo(tester.getTopLeft(pendingTitle).dx, 0.1),
    );
  });

  testWidgets(
    "keeps keyset continuation automatic without visible pagination",
    (tester) async {
      tester.view.physicalSize = const Size(1280, 800);
      tester.view.devicePixelRatio = 1;
      addTearDown(tester.view.resetPhysicalSize);
      addTearDown(tester.view.resetDevicePixelRatio);
      final snapshot = LibrarySnapshot(
        catalogPath: "C:\\AmeData\\ame.sqlite3",
        revision: BigInt.one,
        queryId: "query-1",
        roots: const [
          LibraryRoot(
            id: "root-1",
            path: "C:\\Pictures",
            activeScanId: "scan-1",
            createdUnixMs: 1,
            assetCount: 2,
            issueCount: 0,
          ),
        ],
        assets: [
          LibraryAsset(
            assetId: "asset-1",
            locationId: "location-1",
            rootId: "root-1",
            sourcePath: "C:\\Pictures\\one.png",
            relativePath: "one.png",
            previewPath: "C:\\Missing\\one.jpg",
            fileSize: BigInt.one,
            modifiedUnixMs: 1,
            width: 1,
            height: 1,
          ),
        ],
        nextCursor: LibraryCatalogCursor(
          revision: BigInt.one,
          queryId: "query-1",
          primaryMissing: true,
          primaryText: "",
          primaryNumber: 1,
          rootId: "root-1",
          locationId: "location-1",
        ),
      );

      await tester.pumpWidget(
        ProviderScope(
          overrides: [
            initialLibraryStateProvider.overrideWithValue(
              LibraryState.fromSnapshot(snapshot),
            ),
          ],
          child: const AmeApp(),
        ),
      );
      await tester.pump();

      expect(find.byKey(const Key("library-load-more-button")), findsNothing);
      expect(find.byKey(const Key("gallery-date-unknown")), findsOneWidget);
      final summary = tester.widget<Text>(
        find.byKey(const Key("library-summary")),
      );
      expect(summary.data, "2 张图片");
    },
  );

  testWidgets("groups server-ordered assets under capture dates and unknown", (
    tester,
  ) async {
    tester.view.physicalSize = const Size(1280, 800);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);
    final capture = LibraryCaptureTimeEvidence(
      localTime: "2025-08-07T12:34:56.000000000",
      source: LibraryCaptureTimeSource.exifDateTimeOriginal,
      rawValue: "2025:08:07 12:34:56",
    );
    final snapshot = LibrarySnapshot(
      catalogPath: "C:\\AmeData\\ame.sqlite3",
      revision: BigInt.one,
      queryId: "query-1",
      roots: const [
        LibraryRoot(
          id: "root-1",
          path: "C:\\Pictures",
          activeScanId: "scan-1",
          createdUnixMs: 1,
          assetCount: 3,
          issueCount: 0,
        ),
      ],
      assets: [
        LibraryAsset(
          assetId: "asset-1",
          locationId: "location-1",
          rootId: "root-1",
          sourcePath: "C:\\Pictures\\one.png",
          relativePath: "one.png",
          previewPath: "C:\\Missing\\one.jpg",
          fileSize: BigInt.one,
          modifiedUnixMs: 3,
          width: 1,
          height: 1,
          captureTime: capture,
        ),
        LibraryAsset(
          assetId: "asset-2",
          locationId: "location-2",
          rootId: "root-1",
          sourcePath: "C:\\Pictures\\two.png",
          relativePath: "two.png",
          previewPath: "C:\\Missing\\two.jpg",
          fileSize: BigInt.one,
          modifiedUnixMs: 2,
          width: 1,
          height: 1,
          captureTime: capture,
        ),
        LibraryAsset(
          assetId: "asset-3",
          locationId: "location-3",
          rootId: "root-1",
          sourcePath: "C:\\Pictures\\unknown.png",
          relativePath: "unknown.png",
          previewPath: "C:\\Missing\\unknown.jpg",
          fileSize: BigInt.one,
          modifiedUnixMs: 1,
          width: 1,
          height: 1,
        ),
      ],
    );

    await tester.pumpWidget(
      ProviderScope(
        overrides: [
          initialLibraryStateProvider.overrideWithValue(
            LibraryState.fromSnapshot(snapshot),
          ),
        ],
        child: const AmeApp(),
      ),
    );
    await tester.pump();

    final datedHeader = find.byKey(const Key("gallery-date-2025-08-07"));
    final unknownHeader = find.byKey(const Key("gallery-date-unknown"));
    expect(datedHeader, findsOneWidget);
    expect(unknownHeader, findsOneWidget);
    expect(
      tester.getTopLeft(datedHeader).dy,
      lessThan(tester.getTopLeft(unknownHeader).dy),
    );
    expect(find.byKey(const ValueKey("location-1")), findsOneWidget);
    expect(find.byKey(const ValueKey("location-2")), findsOneWidget);
    expect(find.byKey(const ValueKey("location-3")), findsOneWidget);
  });

  testWidgets("keeps a missing source visible with its cached catalog state", (
    tester,
  ) async {
    tester.view.physicalSize = const Size(1280, 800);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);
    final snapshot = LibrarySnapshot(
      catalogPath: "C:\\AmeData\\ame.sqlite3",
      revision: BigInt.one,
      queryId: "query-1",
      roots: const [
        LibraryRoot(
          id: "missing-root",
          path: "E:\\DisconnectedPictures",
          activeScanId: "scan-1",
          createdUnixMs: 1,
          assetCount: 42,
          issueCount: 0,
          availability: LibraryRootAvailability.missing,
          availabilityMessage: "The source volume is disconnected",
        ),
      ],
      assets: const [],
    );

    await tester.pumpWidget(
      ProviderScope(
        overrides: [
          initialLibraryStateProvider.overrideWithValue(
            LibraryState.fromSnapshot(snapshot),
          ),
        ],
        child: const AmeApp(),
      ),
    );
    await tester.pump();

    expect(find.text("DisconnectedPictures"), findsOneWidget);
    expect(find.text("文件夹不存在"), findsOneWidget);
    expect(find.byIcon(Icons.folder_off_outlined), findsOneWidget);
  });

  testWidgets("reveals a checkbox on hover and keeps selection visible", (
    tester,
  ) async {
    tester.view.physicalSize = const Size(1280, 800);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);
    final state = _populatedState(totalItems: 1);

    await tester.pumpWidget(
      ProviderScope(
        overrides: [initialLibraryStateProvider.overrideWithValue(state)],
        child: const AmeApp(),
      ),
    );
    await tester.pump();

    final tile = find.byKey(const ValueKey("location-1"));
    final checkbox = find.byKey(const ValueKey("select-location-1"));
    expect(checkbox, findsNothing);

    final mouse = await tester.createGesture(kind: PointerDeviceKind.mouse);
    await mouse.addPointer(location: Offset.zero);
    await mouse.moveTo(tester.getCenter(tile));
    await tester.pump();
    expect(checkbox, findsOneWidget);

    await tester.tap(checkbox);
    await tester.pump();
    expect(find.byKey(const Key("library-selection-toolbar")), findsOneWidget);
    expect(find.text("已选择 1 个项目"), findsOneWidget);

    await mouse.moveTo(Offset.zero);
    await tester.pump();
    expect(checkbox, findsOneWidget);
  });

  testWidgets("opens the read-only photo context menu on secondary click", (
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
            _populatedState(totalItems: 1),
          ),
        ],
        child: const AmeApp(),
      ),
    );
    await tester.pump();

    await tester.tap(
      find.byKey(const ValueKey("location-1")),
      buttons: kSecondaryMouseButton,
    );
    await tester.pumpAndSettle();

    expect(find.text("打开"), findsOneWidget);
    expect(find.text("查看信息"), findsOneWidget);
    expect(find.text("复制路径"), findsOneWidget);
    expect(find.text("在文件资源管理器中打开"), findsOneWidget);
    expect(find.text("删除"), findsNothing);
  });

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
            _populatedState(totalItems: 1),
          ),
        ],
        child: const AmeApp(),
      ),
    );
    await tester.pump();

    await tester.tap(find.byKey(const ValueKey("source-more-root-1")));
    await tester.pumpAndSettle();
    expect(find.text("更新图库"), findsOneWidget);
    expect(find.text("在文件资源管理器中打开"), findsOneWidget);
    expect(find.text("从 Ame 中移除"), findsOneWidget);

    await tester.sendKeyEvent(LogicalKeyboardKey.escape);
    await tester.pumpAndSettle();
    await tester.tap(find.text("Pictures"), buttons: kSecondaryMouseButton);
    await tester.pumpAndSettle();
    expect(find.text("从 Ame 中移除"), findsOneWidget);
  });

  testWidgets("select all covers the complete query instead of loaded tiles", (
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
            _populatedState(totalItems: 79013),
          ),
        ],
        child: const AmeApp(),
      ),
    );
    await tester.pump();

    await tester.tap(find.byKey(const Key("library-more-menu")));
    await tester.pumpAndSettle();
    await tester.tap(find.text("全选"));
    await tester.pumpAndSettle();

    expect(find.text("已选择 79013 个项目"), findsOneWidget);
    expect(find.byKey(const Key("library-cancel-selection")), findsOneWidget);
  });

  testWidgets("renders the complete-result Material timeline", (tester) async {
    tester.view.physicalSize = const Size(1280, 800);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);

    await tester.pumpWidget(
      ProviderScope(
        overrides: [
          initialLibraryStateProvider.overrideWithValue(
            _populatedState(totalItems: 3).copyWith(
              timeline: LibraryTimeline(
                revision: BigInt.one,
                queryId: "query-1",
                totalItems: 3,
                buckets: const [
                  LibraryTimeBucket(
                    monthKey: "2026-08",
                    itemCount: 1,
                    aspectRatioSum: 1,
                  ),
                  LibraryTimeBucket(
                    monthKey: "2025-01",
                    itemCount: 2,
                    aspectRatioSum: 7,
                  ),
                ],
              ),
            ),
          ),
        ],
        child: const AmeApp(),
      ),
    );
    await tester.pump();

    expect(find.byKey(const Key("library-time-rail")), findsOneWidget);
    expect(find.byKey(const Key("timeline-slider")), findsOneWidget);
    expect(
      find.byKey(const Key("current-month-native-scrollbar")),
      findsNothing,
    );
    expect(find.byKey(const ValueKey("time-marker-2026-08")), findsOneWidget);
    expect(find.byKey(const ValueKey("time-marker-2025-01")), findsOneWidget);
  });

  testWidgets("keeps the single time rail synchronized with gallery scroll", (
    tester,
  ) async {
    tester.view.physicalSize = const Size(1280, 800);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);
    final assets = [
      for (var index = 0; index < 120; index++)
        LibraryAsset(
          assetId: "asset-$index",
          locationId: "location-$index",
          rootId: "root-1",
          sourcePath: "C:\\Pictures\\$index.png",
          relativePath: "$index.png",
          previewPath: "C:\\Missing\\$index.jpg",
          fileSize: BigInt.one,
          modifiedUnixMs: index,
          width: 4,
          height: 3,
          captureTime: const LibraryCaptureTimeEvidence(
            localTime: "2026-08-05T12:00:00.000000000",
            source: LibraryCaptureTimeSource.exifDateTimeOriginal,
            rawValue: "2026:08:05 12:00:00",
          ),
        ),
    ];
    final snapshot = LibrarySnapshot(
      catalogPath: "C:\\AmeData\\ame.sqlite3",
      revision: BigInt.one,
      queryId: "query-1",
      roots: const [
        LibraryRoot(
          id: "root-1",
          path: "C:\\Pictures",
          activeScanId: "scan-1",
          createdUnixMs: 1,
          assetCount: 120,
          issueCount: 0,
        ),
      ],
      assets: assets,
    );
    final state = LibraryState.fromSnapshot(snapshot).copyWith(
      timeline: LibraryTimeline(
        revision: BigInt.one,
        queryId: "query-1",
        totalItems: 120,
        buckets: const [
          LibraryTimeBucket(
            monthKey: "2026-08",
            itemCount: 120,
            aspectRatioSum: 160,
          ),
        ],
      ),
    );
    final catalog = _RecordingQueryCatalog(snapshot);

    await tester.pumpWidget(
      ProviderScope(
        overrides: [
          initialLibraryStateProvider.overrideWithValue(state),
          libraryCatalogProvider.overrideWithValue(catalog),
        ],
        child: const AmeApp(),
      ),
    );
    await tester.pumpAndSettle();

    final timelineSlider = find.byKey(const Key("timeline-slider"));
    expect(tester.widget<Slider>(timelineSlider).value, 1);
    final galleryScrollable = find.descendant(
      of: find.byKey(const Key("library-photo-wall")),
      matching: find.byType(Scrollable),
    );
    final galleryPosition = tester
        .state<ScrollableState>(galleryScrollable)
        .position;
    expect(galleryPosition.pixels, 0);

    expect(
      find.byKey(const Key("current-month-native-scrollbar")),
      findsNothing,
    );
    await tester.drag(
      find.byKey(const Key("library-photo-wall")),
      const Offset(0, -480),
    );
    await tester.pumpAndSettle();

    expect(galleryPosition.pixels, greaterThan(0));
    expect(tester.widget<Slider>(timelineSlider).value, 1);
    expect(catalog.timeAnchors, isEmpty);
  });

  testWidgets(
    "loads the previous window from a timeline anchor without visual jumping",
    (tester) async {
      tester.view.physicalSize = const Size(1280, 800);
      tester.view.devicePixelRatio = 1;
      addTearDown(tester.view.resetPhysicalSize);
      addTearDown(tester.view.resetDevicePixelRatio);
      const roots = [
        LibraryRoot(
          id: "root-1",
          path: "C:\\Pictures",
          activeScanId: "scan-1",
          createdUnixMs: 1,
          assetCount: 90,
          issueCount: 0,
        ),
      ];
      final previousCursor = LibraryCatalogCursor(
        revision: BigInt.one,
        queryId: "query-1",
        primaryMissing: false,
        primaryText: "2025-01-01T00:00:00.000000000",
        primaryNumber: 0,
        rootId: "root-1",
        locationId: "anchor-0",
      );
      final anchoredAssets = [
        for (var index = 0; index < 60; index++)
          _galleryAsset(
            id: "anchor-$index",
            captureLocalTime: "2025-01-01T00:00:00.000000000",
          ),
      ];
      final previousAssets = [
        for (var index = 0; index < 30; index++)
          _galleryAsset(
            id: "previous-$index",
            captureLocalTime: "2026-08-01T00:00:00.000000000",
          ),
      ];
      final anchoredSnapshot = LibrarySnapshot(
        catalogPath: "C:\\AmeData\\ame.sqlite3",
        revision: BigInt.one,
        queryId: "query-1",
        roots: roots,
        assets: anchoredAssets,
        previousCursor: previousCursor,
      );
      final previousSnapshot = LibrarySnapshot(
        catalogPath: "C:\\AmeData\\ame.sqlite3",
        revision: BigInt.one,
        queryId: "query-1",
        roots: roots,
        assets: previousAssets,
      );
      final initialState = LibraryState.fromSnapshot(anchoredSnapshot).copyWith(
        timeline: LibraryTimeline(
          revision: BigInt.one,
          queryId: "query-1",
          totalItems: 90,
          buckets: const [
            LibraryTimeBucket(
              monthKey: "2026-08",
              itemCount: 30,
              aspectRatioSum: 40,
            ),
            LibraryTimeBucket(
              monthKey: "2025-01",
              itemCount: 60,
              aspectRatioSum: 80,
            ),
          ],
        ),
      );
      final catalog = _RecordingQueryCatalog(
        anchoredSnapshot,
        previousSnapshot: previousSnapshot,
      );
      final container = ProviderContainer(
        overrides: [
          initialLibraryStateProvider.overrideWithValue(initialState),
          libraryCatalogProvider.overrideWithValue(catalog),
        ],
      );
      addTearDown(container.dispose);

      await tester.pumpWidget(
        UncontrolledProviderScope(container: container, child: const AmeApp()),
      );
      await tester.pumpAndSettle();

      final photoWall = find.byKey(const Key("library-photo-wall"));
      final scrollable = find.descendant(
        of: photoWall,
        matching: find.byType(Scrollable),
      );
      final position = tester.state<ScrollableState>(scrollable).position;
      final maximumBefore = position.maxScrollExtent;
      final anchorTopBefore = tester.getTopLeft(
        find.byKey(const ValueKey("anchor-0")),
      );
      expect(position.pixels, 0);

      await tester.sendEventToBinding(
        PointerScrollEvent(
          position: tester.getCenter(photoWall),
          scrollDelta: const Offset(0, -240),
        ),
      );
      await tester.pumpAndSettle();

      final state = container.read(libraryControllerProvider);
      final anchorAfter = find.byKey(const ValueKey("anchor-0"));
      expect(
        anchorAfter,
        findsOneWidget,
        reason:
            "pixels=${position.pixels}, max=${position.maxScrollExtent}, "
            "oldMax=$maximumBefore, assets=${state.assets.length}, "
            "before=${catalog.beforeCursors.length}",
      );
      final anchorTopAfter = tester.getTopLeft(anchorAfter);
      expect(catalog.beforeCursors, [same(previousCursor)]);
      expect(state.assets, hasLength(90));
      expect(state.assets.first.locationId, "previous-0");
      expect(state.previousCursor, isNull);
      expect(position.pixels, greaterThan(0));
      expect(anchorTopAfter.dy, closeTo(anchorTopBefore.dy, 1));
    },
  );
}

LibraryAsset _galleryAsset({
  required String id,
  required String captureLocalTime,
}) {
  return LibraryAsset(
    assetId: "asset-$id",
    locationId: id,
    rootId: "root-1",
    sourcePath: "C:\\Pictures\\$id.png",
    relativePath: "$id.png",
    previewPath: "C:\\Missing\\$id.jpg",
    fileSize: BigInt.one,
    modifiedUnixMs: 1,
    width: 4,
    height: 3,
    captureTime: LibraryCaptureTimeEvidence(
      localTime: captureLocalTime,
      source: LibraryCaptureTimeSource.exifDateTimeOriginal,
      rawValue: captureLocalTime,
    ),
  );
}

LibraryState _populatedState({required int totalItems}) {
  final snapshot = LibrarySnapshot(
    catalogPath: "C:\\AmeData\\ame.sqlite3",
    revision: BigInt.one,
    queryId: "query-1",
    roots: [
      LibraryRoot(
        id: "root-1",
        path: "C:\\Pictures",
        activeScanId: "scan-1",
        createdUnixMs: 1,
        assetCount: totalItems,
        issueCount: 0,
        availability: LibraryRootAvailability.available,
      ),
    ],
    assets: [
      LibraryAsset(
        assetId: "asset-1",
        locationId: "location-1",
        rootId: "root-1",
        sourcePath: "C:\\Pictures\\one.png",
        relativePath: "one.png",
        previewPath: "C:\\Missing\\one.jpg",
        fileSize: BigInt.one,
        modifiedUnixMs: 1,
        width: 4,
        height: 3,
      ),
    ],
  );
  return LibraryState.fromSnapshot(snapshot).copyWith(
    timeline: LibraryTimeline(
      revision: BigInt.one,
      queryId: "query-1",
      totalItems: totalItems,
      buckets: [
        LibraryTimeBucket(
          itemCount: totalItems,
          aspectRatioSum: totalItems.toDouble(),
        ),
      ],
    ),
  );
}

class _RecordingQueryCatalog implements LibraryCatalog {
  _RecordingQueryCatalog(this.snapshot, {this.previousSnapshot});

  final LibrarySnapshot snapshot;
  final LibrarySnapshot? previousSnapshot;
  final List<LibraryGalleryQuery> loadQueries = [];
  final List<LibraryTimeAnchor> timeAnchors = [];
  final List<LibraryCatalogCursor?> beforeCursors = [];

  @override
  Future<LibrarySnapshot> load({
    required int maxItems,
    required LibraryGalleryQuery query,
    LibraryCatalogCursor? after,
    LibraryCatalogCursor? before,
  }) async {
    loadQueries.add(query);
    beforeCursors.add(before);
    return before == null ? snapshot : previousSnapshot ?? snapshot;
  }

  @override
  Future<LibrarySnapshot> loadAtTime({
    required int maxItems,
    required LibraryGalleryQuery query,
    required LibraryTimeAnchor anchor,
  }) async {
    loadQueries.add(query);
    timeAnchors.add(anchor);
    return snapshot;
  }

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
  Future<bool> unregisterRoot(String rootId) async => true;
}

class _RecordingFolderCatalog implements LibraryFolderCatalog {
  final List<({String rootId, String parentRelativePath})> requests = [];

  @override
  Future<LibraryFolderPage> loadFolderPage({
    required String rootId,
    required String parentRelativePath,
    required int maxItems,
    LibraryFolderCursor? after,
  }) async {
    requests.add((rootId: rootId, parentRelativePath: parentRelativePath));
    final folders = switch (parentRelativePath) {
      "" => const [
        LibraryFolder(
          rootId: "root-1",
          relativePath: "Album",
          name: "Album",
          directAssetCount: 1,
          descendantAssetCount: 2,
        ),
        LibraryFolder(
          rootId: "root-1",
          relativePath: "Other",
          name: "Other",
          directAssetCount: 1,
          descendantAssetCount: 1,
        ),
      ],
      "Album" => const [
        LibraryFolder(
          rootId: "root-1",
          relativePath: "Album/Sub",
          name: "Sub",
          directAssetCount: 1,
          descendantAssetCount: 1,
        ),
      ],
      _ => const <LibraryFolder>[],
    };
    return LibraryFolderPage(
      revision: BigInt.one,
      rootId: rootId,
      parentRelativePath: parentRelativePath,
      folders: folders,
    );
  }
}

class _RecordingPlatformActions implements LibraryPlatformActions {
  final List<({String rootPath, String relativePath})> openedLibraryFolders =
      [];

  @override
  Future<void> copyText(String value) async {}

  @override
  Future<void> openDirectory(String path) async {}

  @override
  Future<void> openLibraryFolder(String rootPath, String relativePath) async {
    openedLibraryFolders.add((rootPath: rootPath, relativePath: relativePath));
  }

  @override
  Future<void> revealFile(String path) async {}
}
