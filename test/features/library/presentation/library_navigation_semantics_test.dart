import "package:cedarflake_ame/app/ame_app.dart";
import "package:cedarflake_ame/app/presentation/ame_overlay_semantics.dart";
import "package:cedarflake_ame/app/presentation/ame_theme.dart";
import "package:cedarflake_ame/features/library/application/library_folder_controller.dart";
import "package:cedarflake_ame/features/library/application/library_controller.dart";
import "package:cedarflake_ame/features/library/domain/library_folder_models.dart";
import "package:cedarflake_ame/features/library/domain/library_models.dart";
import "package:cedarflake_ame/features/library/domain/library_state.dart";
import "package:cedarflake_ame/features/library/domain/library_synchronization_models.dart";
import "package:cedarflake_ame/features/library/presentation/library_strings.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_folder_navigation_tile.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_navigation.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_photo_tile.dart";
import "package:flutter/gestures.dart";
import "package:flutter/material.dart";
import "package:flutter/services.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:flutter_test/flutter_test.dart";

import "../../../support/semantics/retained_semantics_update_harness.dart";

void main() {
  RetainedSemanticsUpdateBinding();

  setUp(RetainedSemanticsUpdateValidator.instance.reset);

  testWidgets(
    "keeps sidebar overlay updates reachable through rebuilds and interactions",
    (tester) async {
      tester.view.physicalSize = const Size(1280, 800);
      tester.view.devicePixelRatio = 1;
      addTearDown(tester.view.resetPhysicalSize);
      addTearDown(tester.view.resetDevicePixelRatio);
      final semanticsHandle = tester.ensureSemantics();
      final seedColor = ValueNotifier(const Color(0xFF0B57D0));
      try {
        await tester.pumpWidget(
          ValueListenableBuilder(
            valueListenable: seedColor,
            builder: (context, color, child) => MaterialApp(
              theme: buildAmeTheme(seedColor: color),
              home: Scaffold(body: _buildNavigation()),
            ),
          ),
        );
        expect(find.text(LibraryStrings.synchronized), findsOneWidget);

        seedColor.value = const Color(0xFF8E4D92);
        await tester.pumpAndSettle();

        final mouse = await tester.createGesture(kind: PointerDeviceKind.mouse);
        await mouse.addPointer();
        await mouse.moveTo(
          tester.getCenter(find.byKey(const ValueKey("source-title-root-1"))),
        );
        await tester.pump(const Duration(milliseconds: 600));
        await mouse.moveTo(
          tester.getCenter(find.byKey(const ValueKey("source-expand-root-1"))),
        );
        await tester.pump();
        await tester.pump(const Duration(milliseconds: 25));
        await tester.pump(const Duration(milliseconds: 50));
        await tester.pump(const Duration(milliseconds: 600));
        await mouse.moveTo(
          tester.getCenter(find.byKey(const ValueKey("source-more-root-1"))),
        );
        await tester.pump();
        await tester.pump(const Duration(milliseconds: 25));
        await tester.pump(const Duration(milliseconds: 50));

        await tester.tap(find.byKey(const ValueKey("source-more-root-1")));
        await tester.pumpAndSettle();
        expect(find.text("更新图库"), findsOneWidget);
        await tester.tap(find.byKey(const ValueKey("source-more-root-1")));
        await tester.pumpAndSettle();

        await tester.tap(find.byKey(const ValueKey("source-expand-root-1")));
        await tester.pumpAndSettle();
        final folderTitle = find.byKey(
          const ValueKey(
            "folder-title-root-1-Long album name that needs a path tooltip",
          ),
        );
        expect(folderTitle, findsOneWidget);
        expect(
          find.byKey(
            const ValueKey(
              "folder-navigation-root-1-Long album name that needs a path tooltip",
            ),
          ),
          findsOneWidget,
        );
        await mouse.moveTo(tester.getCenter(folderTitle));
        await tester.pump(const Duration(seconds: 1));
        await tester.tap(folderTitle, buttons: kSecondaryMouseButton);
        await tester.pumpAndSettle();
        expect(find.text("在文件资源管理器中打开"), findsOneWidget);

        RetainedSemanticsUpdateValidator.instance.verifyLatestUpdate();
        await mouse.removePointer();
      } finally {
        seedColor.dispose();
        semanticsHandle.dispose();
      }
    },
  );

  testWidgets("keeps the folder menu bound to its opening target", (
    tester,
  ) async {
    final callbackRevision = ValueNotifier(0);
    final openedRevisions = <int>[];
    addTearDown(callbackRevision.dispose);

    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: ValueListenableBuilder<int>(
            valueListenable: callbackRevision,
            builder: (context, revision, child) => LibraryFolderNavigationTile(
              key: const ValueKey("stable-folder-target"),
              root: const LibraryRoot(
                id: "root-1",
                path: "C:\\Pictures",
                displayPath: "C:\\Pictures",
                createdUnixMs: 1,
                assetCount: 1,
                issueCount: 0,
              ),
              folder: const LibraryFolder(
                rootId: "root-1",
                relativePath: "Album",
                name: "Album",
                directAssetCount: 1,
                descendantAssetCount: 1,
              ),
              depth: 0,
              isSelected: false,
              isExpanded: false,
              isBusy: false,
              onSelect: _noop,
              onToggleExpansion: null,
              onOpen: () => openedRevisions.add(revision),
            ),
          ),
        ),
      ),
    );

    await tester.tap(
      find.byKey(const ValueKey("folder-title-root-1-Album")),
      buttons: kSecondaryMouseButton,
    );
    await tester.pumpAndSettle();
    callbackRevision.value = 1;
    await tester.pump();
    await tester.tap(find.text(LibraryStrings.openInExplorer));
    await tester.pumpAndSettle();

    expect(openedRevisions, [0]);
  });

  testWidgets("keeps adjacent list tooltips reachable", (tester) async {
    final semanticsHandle = tester.ensureSemantics();
    try {
      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(
            body: ListView(
              children: [
                Row(
                  children: [
                    AmeTooltip(
                      key: const Key("tooltip-a"),
                      message: "Tooltip A",
                      waitDuration: Duration.zero,
                      child: const SizedBox.square(
                        dimension: 100,
                        child: ColoredBox(color: Colors.blue),
                      ),
                    ),
                    const SizedBox(width: 16),
                    AmeTooltip(
                      key: const Key("tooltip-b"),
                      message: "Tooltip B",
                      waitDuration: Duration.zero,
                      child: const SizedBox.square(
                        dimension: 100,
                        child: ColoredBox(color: Colors.grey),
                      ),
                    ),
                  ],
                ),
              ],
            ),
          ),
        ),
      );
      final mouse = await tester.createGesture(kind: PointerDeviceKind.mouse);
      await mouse.addPointer();
      await mouse.moveTo(tester.getCenter(find.byKey(const Key("tooltip-a"))));
      await tester.pump();
      await mouse.moveTo(tester.getCenter(find.byKey(const Key("tooltip-b"))));
      await tester.pump();
      await tester.pump(const Duration(milliseconds: 25));
      await tester.pump(const Duration(milliseconds: 50));
      RetainedSemanticsUpdateValidator.instance.verifyLatestUpdate();
      await mouse.removePointer();
    } finally {
      semanticsHandle.dispose();
    }
  });

  testWidgets("shows reconciliation when startup fails before root status", (
    tester,
  ) async {
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: _buildNavigation(
            hasSynchronizationFailure: true,
            includeRootStatus: false,
          ),
        ),
      ),
    );

    expect(find.text(LibraryStrings.needsReconciliation), findsOneWidget);
    expect(find.text(LibraryStrings.synchronizing), findsNothing);
  });

  testWidgets("keeps the populated application semantics reachable", (
    tester,
  ) async {
    tester.view.physicalSize = const Size(1280, 800);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);
    const systemThemeChannel = MethodChannel("cedarflake_ame/system_theme");
    tester.binding.defaultBinaryMessenger.setMockMethodCallHandler(
      systemThemeChannel,
      (call) async => 0xFF8E4D92,
    );
    addTearDown(
      () => tester.binding.defaultBinaryMessenger.setMockMethodCallHandler(
        systemThemeChannel,
        null,
      ),
    );
    final semanticsHandle = tester.ensureSemantics();
    try {
      await tester.pumpWidget(
        ProviderScope(
          overrides: [
            initialLibraryStateProvider.overrideWithValue(
              _populatedLibraryState(),
            ),
          ],
          child: const AmeApp(),
        ),
      );
      await tester.pump();
      await tester.pump(const Duration(milliseconds: 250));

      final mouse = await tester.createGesture(kind: PointerDeviceKind.mouse);
      await mouse.addPointer();
      const hoverTrace = <Key>[
        Key("window-minimize"),
        Key("window-maximize"),
        Key("library-sort-menu"),
        Key("library-layout-menu"),
        Key("library-more-menu"),
        Key("timeline-previous"),
        Key("timeline-next"),
      ];
      for (final (step, key) in hoverTrace.indexed) {
        await mouse.moveTo(tester.getCenter(find.byKey(key)));
        await tester.pump(const Duration(milliseconds: 600));
        await tester.pump();
        await tester.pump(const Duration(milliseconds: 25));
        await tester.pump(const Duration(milliseconds: 50));
        RetainedSemanticsUpdateValidator.instance.verifyLatestUpdate(
          trace: "application-overlay-trace hover[$step]=$key",
        );
      }

      for (final key in const [
        Key("library-sort-menu"),
        Key("library-layout-menu"),
        Key("library-more-menu"),
      ]) {
        await tester.tap(find.byKey(key));
        await tester.pumpAndSettle();
        RetainedSemanticsUpdateValidator.instance.verifyLatestUpdate(
          trace: "application-overlay-trace menu-open=$key",
        );
        await tester.tap(find.byKey(key));
        await tester.pumpAndSettle();
        RetainedSemanticsUpdateValidator.instance.verifyLatestUpdate(
          trace: "application-overlay-trace menu-close=$key",
        );
      }

      final slider = tester.widget<Slider>(
        find.byKey(const Key("timeline-slider")),
      );
      slider.onChangeStart?.call(slider.value);
      slider.onChanged?.call(0.5);
      slider.onChangeEnd?.call(0.5);
      await tester.pump();
      RetainedSemanticsUpdateValidator.instance.verifyLatestUpdate(
        trace: "application-overlay-trace timeline-slider",
      );

      final tile = find.byType(LibraryPhotoTile).hitTestable().first;
      final tileRect = tester.getRect(tile);
      await tester.tapAt(tileRect.topLeft + const Offset(16, 16));
      await tester.pump();
      await tester.pump();
      expect(find.byKey(const Key("viewer-back-button")), findsOneWidget);
      RetainedSemanticsUpdateValidator.instance.verifyLatestUpdate(
        trace: "application-overlay-trace viewer-open",
      );

      final viewerSlider = tester.widget<Slider>(
        find.descendant(
          of: find.byKey(const Key("viewer-zoom-controls")),
          matching: find.byType(Slider),
        ),
      );
      viewerSlider.onChangeStart?.call(viewerSlider.value);
      viewerSlider.onChanged?.call(1.25);
      viewerSlider.onChangeEnd?.call(1.25);
      await tester.pump();
      RetainedSemanticsUpdateValidator.instance.verifyLatestUpdate(
        trace: "application-overlay-trace viewer-slider",
      );

      var viewerHoverStep = 0;
      for (final finder in [
        find.byKey(const Key("viewer-back-button")),
        find.byKey(const Key("viewer-more-menu")),
        find.byTooltip("缩小（- / Ctrl+-）"),
        find.byTooltip("放大（+ / Ctrl++）"),
      ]) {
        await mouse.moveTo(tester.getCenter(finder));
        await tester.pump();
        await tester.pump(const Duration(milliseconds: 600));
        RetainedSemanticsUpdateValidator.instance.verifyLatestUpdate(
          trace: "application-overlay-trace viewer-hover[$viewerHoverStep]",
        );
        viewerHoverStep += 1;
      }

      await tester.tap(find.byKey(const Key("viewer-more-menu")));
      await tester.pump();
      await tester.pump(const Duration(milliseconds: 500));
      RetainedSemanticsUpdateValidator.instance.verifyLatestUpdate(
        trace: "application-overlay-trace viewer-menu-open",
      );
      await tester.tap(find.byKey(const Key("viewer-more-menu")));
      await tester.pump();
      await tester.pump(const Duration(milliseconds: 500));
      RetainedSemanticsUpdateValidator.instance.verifyLatestUpdate(
        trace: "application-overlay-trace viewer-menu-close",
      );
      await tester.tap(find.byKey(const Key("viewer-back-button")));
      await tester.pump();
      await tester.pump();
      expect(find.byKey(const ValueKey("location-1")), findsOneWidget);

      RetainedSemanticsUpdateValidator.instance.verifyLatestUpdate(
        trace: "application-overlay-trace viewer-close",
      );

      final reopenedTile = find.byType(LibraryPhotoTile).hitTestable().first;
      final reopenedTileRect = tester.getRect(reopenedTile);
      await tester.tapAt(reopenedTileRect.topLeft + const Offset(16, 16));
      await tester.pump();
      RetainedSemanticsUpdateValidator.instance.verifyLatestUpdate(
        trace: "application-overlay-trace viewer-reopen-frame-1",
      );
      await tester.pump();
      expect(find.byKey(const Key("viewer-back-button")), findsOneWidget);
      RetainedSemanticsUpdateValidator.instance.verifyLatestUpdate(
        trace: "application-overlay-trace viewer-reopen-frame-2",
      );
      await tester.tap(find.byKey(const Key("viewer-back-button")));
      await tester.pump();
      RetainedSemanticsUpdateValidator.instance.verifyLatestUpdate(
        trace: "application-overlay-trace viewer-reclose-frame-1",
      );
      await tester.pump();
      expect(find.byKey(const ValueKey("location-1")), findsOneWidget);
      RetainedSemanticsUpdateValidator.instance.verifyLatestUpdate(
        trace: "application-overlay-trace viewer-reclose-frame-2",
      );
      await mouse.removePointer();
    } finally {
      semanticsHandle.dispose();
    }
  });
}

LibraryState _populatedLibraryState() {
  final snapshot = LibrarySnapshot(
    catalogPath: "C:\\AmeData\\ame.sqlite3",
    revision: BigInt.one,
    queryId: "query-1",
    roots: const [
      LibraryRoot(
        id: "root-1",
        path: "C:\\Pictures",
        displayPath: "C:\\Pictures",
        activeScanId: "scan-1",
        createdUnixMs: 1,
        assetCount: 1,
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
        displayPath: "C:\\Pictures\\one.png",
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
      totalItems: 1,
      buckets: const [LibraryTimeBucket(itemCount: 1, aspectRatioSum: 1)],
    ),
  );
}

Widget _buildNavigation({
  bool hasSynchronizationFailure = false,
  bool includeRootStatus = true,
}) {
  const folderPath = "Long album name that needs a path tooltip";
  return Align(
    alignment: Alignment.topLeft,
    child: LibraryNavigation(
      isCompact: false,
      width: 260,
      isSettingsSelected: false,
      roots: const [
        LibraryRoot(
          id: "root-1",
          path: "C:\\Pictures\\Long source name that needs a path tooltip",
          displayPath:
              "C:\\Pictures\\Long source name that needs a path tooltip",
          createdUnixMs: 1,
          assetCount: 1,
          issueCount: 0,
          availability: LibraryRootAvailability.available,
        ),
      ],
      rootSynchronizationStatuses: includeRootStatus
          ? {
              "root-1": LibraryRootSynchronizationStatus(
                rootId: "root-1",
                rootGeneration: BigInt.one,
                availability: LibraryRootAvailability.available,
                freshness: LibraryCatalogFreshness.synchronized,
                freshnessCause: LibraryCatalogFreshnessCause.noPendingChanges,
                sourceStatus: LibraryChangeSourceStatus.healthy,
                pendingChangeCount: BigInt.zero,
                retryWaitCount: BigInt.zero,
                freshnessUnknownCount: BigInt.zero,
              ),
            }
          : const {},
      hasSynchronizationFailure: hasSynchronizationFailure,
      selectedRootId: "root-1",
      selectedFolderRelativePath: null,
      transientRootPath: null,
      folderTree: LibraryFolderTreeState(
        revision: BigInt.one,
        branches: {
          const LibraryFolderBranchKey(
            rootId: "root-1",
            parentRelativePath: "",
          ): const LibraryFolderBranch(
            folders: [
              LibraryFolder(
                rootId: "root-1",
                relativePath: folderPath,
                name: folderPath,
                directAssetCount: 1,
                descendantAssetCount: 2,
              ),
            ],
            hasLoaded: true,
          ),
        },
      ),
      isBusy: false,
      onSelectLibrary: _noop,
      onSelectRoot: (_) {},
      onSelectFolder: (_, _) {},
      onExpandFolder: (_, _) async {},
      onLoadMoreFolders: (_, _) async {},
      onAddSource: _noop,
      onOpenSettings: _noop,
      onUpdateRoot: (_) {},
      onOpenRoot: (_) {},
      onOpenFolder: (_, _) {},
      onRemoveRoot: (_) {},
    ),
  );
}

void _noop() {}
