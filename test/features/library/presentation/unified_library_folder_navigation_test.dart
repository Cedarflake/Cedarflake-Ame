import "package:cedarflake_ame/app/ame_app.dart";
import "package:cedarflake_ame/app/presentation/ame_menu.dart";
import "package:cedarflake_ame/features/library/adapters/windows_library_platform_actions.dart";
import "package:cedarflake_ame/features/library/application/library_catalog.dart";
import "package:cedarflake_ame/features/library/application/library_controller.dart";
import "package:cedarflake_ame/features/library/application/library_platform_actions.dart";
import "package:cedarflake_ame/features/library/application/library_view_preferences.dart";
import "package:cedarflake_ame/features/library/domain/library_folder_models.dart";
import "package:cedarflake_ame/features/library/domain/library_models.dart";
import "package:cedarflake_ame/features/library/domain/library_state.dart";
import "package:cedarflake_ame/features/library/presentation/library_strings.dart";
import "package:flutter/gestures.dart";
import "package:flutter/material.dart";
import "package:flutter/services.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:flutter_test/flutter_test.dart";

import "support/library_screen_fixture.dart";

void main() {
  testWidgets(
    "uses folder rows as gallery scopes and keeps Explorer in the context menu",
    (tester) async {
      tester.view.physicalSize = const Size(1280, 800);
      tester.view.devicePixelRatio = 1;
      addTearDown(tester.view.resetPhysicalSize);
      addTearDown(tester.view.resetDevicePixelRatio);
      final initialState = populatedScreenState(totalItems: 3);
      final queryCatalog = RecordingQueryCatalog(
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

      expect(folderCatalog.requests, [
        (rootId: "root-1", parentRelativePath: ""),
      ]);
      expect(
        find.byKey(const ValueKey("folder-title-root-1-Album")),
        findsNothing,
      );
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
      expect(
        find.byKey(const ValueKey("folder-expand-root-1-Album/Sub")),
        findsNothing,
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
      expect(find.byType(AmeMenuItemContent), findsOneWidget);
      expect(
        find.descendant(
          of: find.byKey(const ValueKey("folder-tile-root-1-Album/Sub")),
          matching: find.byType(MenuAnchor),
        ),
        findsNothing,
      );
      await tester.tap(find.text(LibraryStrings.openInExplorer).last);
      await tester.pumpAndSettle();
      expect(platformActions.openedLibraryFolders, [
        (rootPath: "C:\\Pictures", relativePath: "Album/Sub"),
      ]);

      await tester.sendKeyEvent(LogicalKeyboardKey.contextMenu);
      await tester.pumpAndSettle();
      expect(find.text(LibraryStrings.openInExplorer), findsOneWidget);
      await tester.sendKeyEvent(LogicalKeyboardKey.escape);
      await tester.pumpAndSettle();
    },
  );

  testWidgets(
    "starts a selected folder at the top instead of reusing the prior time anchor",
    (tester) async {
      tester.view.physicalSize = const Size(1280, 800);
      tester.view.devicePixelRatio = 1;
      addTearDown(tester.view.resetPhysicalSize);
      addTearDown(tester.view.resetDevicePixelRatio);
      final assets = [
        for (var index = 0; index < 120; index++)
          galleryAsset(
            id: "folder-scope-$index",
            captureLocalTime: "2026-08-05T00:00:00.000000000",
          ),
      ];
      final initialSnapshot = LibrarySnapshot(
        catalogPath: "C:\\AmeData\\ame.sqlite3",
        revision: BigInt.one,
        queryId: "folder-scope-initial",
        roots: const [
          LibraryRoot(
            id: "root-1",
            path: "C:\\Pictures",
            displayPath: "C:\\Pictures",
            activeScanId: "scan-1",
            createdUnixMs: 1,
            assetCount: 120,
            issueCount: 0,
            availability: LibraryRootAvailability.available,
          ),
        ],
        assets: assets,
      );
      final initialState = LibraryState.fromSnapshot(initialSnapshot).copyWith(
        timeline: LibraryTimeline(
          revision: BigInt.one,
          queryId: initialSnapshot.queryId,
          totalItems: assets.length,
          buckets: const [
            LibraryTimeBucket(
              monthKey: "2026-08",
              itemCount: 120,
              aspectRatioSum: 160,
            ),
          ],
        ),
      );
      final catalog = AnchorResolvingQueryCatalog(initialSnapshot);
      final folderCatalog = _RecordingFolderCatalog();
      final container = ProviderContainer(
        overrides: [
          initialLibraryStateProvider.overrideWithValue(initialState),
          initialLibraryViewPreferencesProvider.overrideWithValue(
            const LibraryViewPreferences(
              layoutShape: GalleryLayoutShape.square,
              thumbnailSize: GalleryThumbnailSize.medium,
            ),
          ),
          libraryCatalogProvider.overrideWithValue(catalog),
          libraryFolderCatalogProvider.overrideWithValue(folderCatalog),
        ],
      );
      addTearDown(container.dispose);

      await tester.pumpWidget(
        UncontrolledProviderScope(container: container, child: const AmeApp()),
      );
      await tester.pump();

      final scrollPosition = galleryScrollPosition(tester);
      scrollPosition.jumpTo(scrollPosition.maxScrollExtent * 0.55);
      await tester.pump();
      await tester.pump();
      expect(scrollPosition.pixels, greaterThan(0));

      await tester.tap(find.byKey(const ValueKey("source-expand-root-1")));
      await tester.pumpAndSettle();
      await tester.tap(find.byKey(const ValueKey("folder-title-root-1-Album")));
      await tester.pumpAndSettle();

      expect(catalog.requestedLocationIds, isEmpty);
      expect(
        container.read(libraryControllerProvider).query.folderRelativePath,
        "Album",
      );
      expect(
        scrollPosition.pixels,
        closeTo(scrollPosition.minScrollExtent, 0.01),
      );
      expect(
        tester.widget<Slider>(find.byKey(const Key("timeline-slider"))).value,
        1,
      );
    },
  );
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
  Future<void> revealDirectory(String path) async {}

  @override
  Future<void> revealLibraryFolder(String rootPath, String relativePath) async {
    openedLibraryFolders.add((rootPath: rootPath, relativePath: relativePath));
  }

  @override
  Future<void> revealFile(String path) async {}
}
