import "dart:async";
import "dart:typed_data";
import "dart:ui" show CheckedState;

import "package:cedarflake_ame/app/ame_app.dart";
import "package:cedarflake_ame/app/presentation/ame_menu.dart";
import "package:cedarflake_ame/features/library/adapters/windows_library_platform_actions.dart";
import "package:cedarflake_ame/features/library/application/library_catalog.dart";
import "package:cedarflake_ame/features/library/application/library_controller.dart";
import "package:cedarflake_ame/features/library/application/library_layout_manifest_catalog.dart";
import "package:cedarflake_ame/features/library/application/library_platform_actions.dart";
import "package:cedarflake_ame/features/library/application/library_previewer.dart";
import "package:cedarflake_ame/features/library/application/library_scanner.dart";
import "package:cedarflake_ame/features/library/application/library_view_preferences.dart";
import "package:cedarflake_ame/features/library/domain/gallery_layout_manifest.dart";
import "package:cedarflake_ame/features/library/domain/library_folder_models.dart";
import "package:cedarflake_ame/features/library/domain/library_models.dart";
import "package:cedarflake_ame/features/library/domain/library_state.dart";
import "package:cedarflake_ame/features/library/presentation/library_strings.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_exact_extent_sliver.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_gallery_header.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_gallery_layout.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_gallery_layout_snapshot.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_main_surface.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_navigation_resize_handle.dart";
import "package:cedarflake_ame/features/settings/application/ame_preferences.dart";
import "package:flutter/gestures.dart";
import "package:flutter/material.dart";
import "package:flutter/services.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:flutter_test/flutter_test.dart";
import "package:material_symbols_icons/symbols.dart";

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
    expect(find.byKey(const Key("library-import-button")), findsNothing);
    expect(find.byKey(const Key("library-sidebar-import")), findsOneWidget);
    expect(find.byKey(const Key("library-sidebar-settings")), findsOneWidget);
    expect(find.text("Cedarflake Ame"), findsNothing);
    expect(find.text("Ame"), findsOneWidget);
    expect(
      find.ancestor(
        of: find.byKey(const Key("window-close")),
        matching: find.byKey(const Key("library-global-bar")),
      ),
      findsOneWidget,
    );
    final searchCenter = tester.getCenter(
      find.byKey(const Key("library-search")),
    );
    final globalBarCenter = tester.getCenter(
      find.byKey(const Key("library-global-bar")),
    );
    final closeCenter = tester.getCenter(find.byKey(const Key("window-close")));
    final minimizeCenter = tester.getCenter(
      find.byKey(const Key("window-minimize")),
    );
    final notificationCenter = tester.getCenter(
      find.byKey(const Key("notification-history-button")),
    );
    expect(searchCenter.dy, closeTo(globalBarCenter.dy, 0.1));
    expect(tester.getSize(find.byKey(const Key("library-search"))).height, 44);
    expect(closeCenter.dy, closeTo(globalBarCenter.dy, 0.1));
    expect(notificationCenter.dx, lessThan(minimizeCenter.dx));
    expect(
      notificationCenter.dx,
      greaterThan(
        tester.getTopRight(find.byKey(const Key("library-search"))).dx,
      ),
    );
    expect(find.byKey(const Key("notification-read-icon")), findsOneWidget);
    expect(find.text("通知历史"), findsNothing);
    expect(
      find.ancestor(
        of: find.byKey(const Key("library-search")),
        matching: find.byKey(const Key("library-global-bar")),
      ),
      findsOneWidget,
    );
    await tester.tap(find.byKey(const Key("library-sidebar-settings")));
    await tester.pump();
    expect(find.byKey(const Key("ame-settings-page")), findsOneWidget);
    expect(find.byType(AlertDialog), findsNothing);
    expect(find.byKey(const Key("library-global-bar")), findsOneWidget);
    expect(
      tester
          .widget<ListTile>(find.byKey(const Key("library-sidebar-settings")))
          .selected,
      isTrue,
    );

    await tester.tap(find.byKey(const Key("library-sidebar-library")));
    await tester.pump();
    expect(find.byKey(const Key("ame-settings-page")), findsNothing);
    expect(find.byKey(const Key("library-empty-state")), findsOneWidget);
  });

  testWidgets(
    "batches recovered preview dimensions into dynamic gallery geometry",
    (tester) async {
      tester.view.physicalSize = const Size(1280, 800);
      tester.view.devicePixelRatio = 1;
      addTearDown(tester.view.resetPhysicalSize);
      addTearDown(tester.view.resetDevicePixelRatio);
      final assets = [
        for (var index = 0; index < 80; index++)
          LibraryAsset(
            assetId: "asset-$index",
            locationId: "location-$index",
            rootId: "root-1",
            sourcePath: "C:\\Pictures\\$index.jpg",
            displayPath: "C:\\Pictures\\$index.jpg",
            relativePath: "$index.jpg",
            previewPath: "",
            fileSize: BigInt.one,
            modifiedUnixMs: 1,
            width: 0,
            height: 0,
            previewStatus: LibraryPreviewStatus.pending,
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
            displayPath: "C:\\Pictures",
            activeScanId: "scan-1",
            createdUnixMs: 1,
            assetCount: 80,
            issueCount: 0,
          ),
        ],
        assets: assets,
      );
      final initialState = LibraryState.fromSnapshot(snapshot).copyWith(
        timeline: LibraryTimeline(
          revision: BigInt.one,
          queryId: "query-1",
          totalItems: assets.length,
          buckets: [
            LibraryTimeBucket(
              itemCount: assets.length,
              aspectRatioSum: assets.length.toDouble(),
            ),
          ],
        ),
      );
      final previewer = _ControlledLibraryPreviewer(assets);
      final manifest = _unknownDimensionManifest(assets);

      await tester.pumpWidget(
        ProviderScope(
          overrides: [
            initialLibraryStateProvider.overrideWithValue(initialState),
            libraryPreviewerProvider.overrideWithValue(previewer),
            libraryGalleryLayoutManifestLoaderProvider.overrideWithValue(
              _FixedLayoutManifestLoader(manifest),
            ),
          ],
          child: const AmeApp(),
        ),
      );
      await tester.pump();
      await tester.pump();

      final galleryWall = find.byKey(const Key("library-photo-wall"));
      final scrollPosition = _galleryScrollPosition(tester);
      final initialLayoutSnapshot = LibraryGalleryLayoutSnapshot.build(
        manifest: manifest,
        availableWidth: tester.getSize(galleryWall).width - 40,
        thumbnailSize: GalleryThumbnailSize.medium,
        sortKey: LibraryGallerySortKey.captureTime,
      );
      final initialVisibleEnd = initialLayoutSnapshot.metrics
          .rowEndGlobalItemIndexExclusive(
            initialLayoutSnapshot.metrics.itemIndexForScrollOffset(
              scrollPosition.pixels + scrollPosition.viewportDimension,
            ),
          )!;
      expect(previewer.requests, hasLength(2));
      final firstLocation = previewer.requests[0];
      final secondLocation = previewer.requests[1];
      final firstTile = find.byKey(ValueKey(firstLocation));
      final secondTile = find.byKey(ValueKey(secondLocation));
      expect(firstTile, findsOneWidget);
      expect(secondTile, findsOneWidget);
      final firstWidthBefore = tester.getSize(firstTile).width;
      final secondWidthBefore = tester.getSize(secondTile).width;

      previewer.succeed(firstLocation, width: 800, height: 400);
      previewer.succeed(secondLocation, width: 200, height: 400);
      await tester.pump();
      await tester.pump(const Duration(milliseconds: 159));
      expect(tester.getSize(firstTile).width, firstWidthBefore);
      expect(tester.getSize(secondTile).width, secondWidthBefore);

      expect(previewer.requests, hasLength(4));
      await tester.pump(const Duration(milliseconds: 1));
      await tester.pump();
      await tester.pump();
      await tester.pump();
      expect(tester.getSize(firstTile).width, greaterThan(firstWidthBefore));
      expect(tester.getSize(secondTile).width, lessThan(secondWidthBefore));

      final gesture = await tester.startGesture(tester.getCenter(galleryWall));
      await gesture.moveBy(const Offset(0, -24));
      await tester.pump();
      final thirdLocation = previewer.requests[2];
      final fourthLocation = previewer.requests[3];
      final thirdTile = find.byKey(ValueKey(thirdLocation));
      final fourthTile = find.byKey(ValueKey(fourthLocation));
      final thirdWidthBefore = tester.getSize(thirdTile).width;
      final fourthWidthBefore = tester.getSize(fourthTile).width;
      previewer.succeed(thirdLocation, width: 800, height: 400);
      previewer.succeed(fourthLocation, width: 200, height: 400);
      await tester.pump();
      await tester.pump(const Duration(milliseconds: 1700));
      expect(tester.getSize(thirdTile).width, thirdWidthBefore);
      expect(tester.getSize(fourthTile).width, fourthWidthBefore);

      await gesture.up();
      await tester.pump();
      expect(tester.getSize(thirdTile).width, thirdWidthBefore);
      expect(tester.getSize(fourthTile).width, fourthWidthBefore);
      for (
        var attempt = 0;
        attempt < 20 && scrollPosition.isScrollingNotifier.value;
        attempt += 1
      ) {
        await tester.pump(const Duration(milliseconds: 50));
        expect(tester.getSize(thirdTile).width, thirdWidthBefore);
        expect(tester.getSize(fourthTile).width, fourthWidthBefore);
      }
      expect(scrollPosition.isScrollingNotifier.value, isFalse);
      await tester.pump(const Duration(milliseconds: 159));
      expect(tester.getSize(thirdTile).width, thirdWidthBefore);
      expect(tester.getSize(fourthTile).width, fourthWidthBefore);

      await tester.pump(const Duration(milliseconds: 1));
      await tester.pump();
      await tester.pump();
      await tester.pump();

      expect(tester.getSize(thirdTile).width, greaterThan(thirdWidthBefore));
      expect(tester.getSize(fourthTile).width, lessThan(fourthWidthBefore));

      final extentAfterVisibleRecovery = scrollPosition.maxScrollExtent;
      final completedLocations = {
        firstLocation,
        secondLocation,
        thirdLocation,
        fourthLocation,
      };
      String? deferredLocation;
      var requestCursor = 4;
      for (
        var attempt = 0;
        attempt < 80 && deferredLocation == null;
        attempt++
      ) {
        while (requestCursor < previewer.requests.length &&
            deferredLocation == null) {
          final locationId = previewer.requests[requestCursor];
          requestCursor += 1;
          if (!completedLocations.add(locationId)) {
            continue;
          }
          final itemIndex = int.parse(locationId.substring("location-".length));
          if (itemIndex < initialVisibleEnd) {
            previewer.succeed(locationId, width: 1000, height: 1000);
          } else {
            deferredLocation = locationId;
            previewer.succeed(locationId, width: 5000, height: 1000);
          }
        }
        await tester.pump();
        await tester.pump();
      }
      expect(deferredLocation, isNotNull);

      await tester.pump(const Duration(milliseconds: 1700));
      await tester.pump();
      await tester.pump();
      expect(
        scrollPosition.maxScrollExtent,
        closeTo(extentAfterVisibleRecovery, 0.01),
        reason:
            "reflow-exposed rows must remain outside the frozen recovery epoch",
      );
    },
  );

  testWidgets(
    "publishes recovered dimensions after a timeline jump without scrolling",
    (tester) async {
      tester.view.physicalSize = const Size(1280, 800);
      tester.view.devicePixelRatio = 1;
      addTearDown(tester.view.resetPhysicalSize);
      addTearDown(tester.view.resetDevicePixelRatio);
      final assets = [
        for (var index = 0; index < 80; index++)
          LibraryAsset(
            assetId: "asset-$index",
            locationId: "location-$index",
            rootId: "root-1",
            sourcePath: "C:\\Pictures\\$index.jpg",
            displayPath: "C:\\Pictures\\$index.jpg",
            relativePath: "$index.jpg",
            previewPath: "",
            fileSize: BigInt.one,
            modifiedUnixMs: 1,
            width: 0,
            height: 0,
            previewStatus: LibraryPreviewStatus.pending,
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
            displayPath: "C:\\Pictures",
            activeScanId: "scan-1",
            createdUnixMs: 1,
            assetCount: 80,
            issueCount: 0,
          ),
        ],
        assets: assets,
      );
      final initialState = LibraryState.fromSnapshot(snapshot).copyWith(
        timeline: LibraryTimeline(
          revision: BigInt.one,
          queryId: "query-1",
          totalItems: assets.length,
          buckets: [
            LibraryTimeBucket(
              itemCount: assets.length,
              aspectRatioSum: assets.length.toDouble(),
            ),
          ],
        ),
      );
      final previewer = _ControlledLibraryPreviewer(assets);

      await tester.pumpWidget(
        ProviderScope(
          overrides: [
            initialLibraryStateProvider.overrideWithValue(initialState),
            libraryPreviewerProvider.overrideWithValue(previewer),
            libraryGalleryLayoutManifestLoaderProvider.overrideWithValue(
              _FixedLayoutManifestLoader(_unknownDimensionManifest(assets)),
            ),
          ],
          child: const AmeApp(),
        ),
      );
      await tester.pump();
      await tester.pump();
      expect(previewer.requests, hasLength(2));
      final scrollPosition = _galleryScrollPosition(tester);
      final initialPixels = scrollPosition.pixels;

      final slider = tester.widget<Slider>(
        find.byKey(const Key("timeline-slider")),
      );
      slider.onChangeStart?.call(0.8);
      slider.onChanged?.call(0.8);
      slider.onChangeEnd?.call(0.8);
      for (var attempt = 0; attempt < 12; attempt += 1) {
        await tester.pump(const Duration(milliseconds: 16));
        if (scrollPosition.pixels > initialPixels + 100 &&
            previewer.requests.length > 2) {
          break;
        }
      }

      expect(scrollPosition.pixels, greaterThan(initialPixels + 100));
      expect(
        previewer.requests.length,
        greaterThan(2),
        reason: "a timeline jump must replace the visible preview demand",
      );

      final targetLocation = previewer.requests.firstWhere(
        (locationId) =>
            int.parse(locationId.substring("location-".length)) >= 20,
        orElse: () => throw StateError(
          "Timeline jump did not request a deep preview: "
          "${previewer.requests.join(', ')}",
        ),
      );
      final targetTile = find.byKey(ValueKey(targetLocation));
      expect(targetTile, findsOneWidget);
      final widthBefore = tester.getSize(targetTile).width;

      previewer.succeed(targetLocation, width: 800, height: 400);
      await tester.pump();
      await tester.pump(const Duration(milliseconds: 159));
      expect(tester.getSize(targetTile).width, widthBefore);

      await tester.pump(const Duration(milliseconds: 1));
      await tester.pump();
      await tester.pump();
      await tester.pump();
      expect(tester.getSize(targetTile).width, greaterThan(widthBefore));
    },
  );

  testWidgets("pins desktop caption controls to the maximized right edge", (
    tester,
  ) async {
    tester.view.physicalSize = const Size(2048, 1200);
    tester.view.devicePixelRatio = 1;
    tester.view.padding = const FakeViewPadding(top: 8, right: 24);
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);
    addTearDown(tester.view.resetPadding);

    await tester.pumpWidget(const ProviderScope(child: AmeApp()));

    expect(
      tester.getTopLeft(find.byKey(const Key("library-global-bar"))).dy,
      0,
    );
    expect(tester.getTopRight(find.byKey(const Key("window-close"))).dx, 2040);
  });

  testWidgets("separates the application shell with Material surfaces", (
    tester,
  ) async {
    tester.view.physicalSize = const Size(1280, 800);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);

    await tester.pumpWidget(const ProviderScope(child: AmeApp()));
    await tester.pump();

    final context = tester.element(
      find.byKey(const Key("library-main-surface")),
    );
    final colorScheme = Theme.of(context).colorScheme;
    final scaffold = tester.widget<Scaffold>(find.byType(Scaffold).first);
    final globalSurface = tester.widget<Material>(
      find.byKey(const Key("library-global-surface")),
    );
    final navigationSurface = tester.widget<Material>(
      find.byKey(const Key("library-navigation-surface")),
    );
    final mainSurface = tester.widget<Material>(
      find.byKey(const Key("library-main-surface")),
    );

    expect(scaffold.backgroundColor, colorScheme.surfaceContainerLow);
    expect(globalSurface.color, colorScheme.surfaceContainerLow);
    expect(navigationSurface.color, colorScheme.surfaceContainerLow);
    expect(mainSurface.color, colorScheme.surfaceContainerLowest);
    expect(mainSurface.clipBehavior, Clip.antiAlias);
    expect(
      (mainSurface.shape as RoundedRectangleBorder).borderRadius,
      LibraryMainSurface.borderRadius,
    );
    expect(
      find.ancestor(
        of: find.byKey(const Key("library-gallery-header")),
        matching: find.byKey(const Key("library-main-surface")),
      ),
      findsOneWidget,
    );
  });

  testWidgets("resizes, resets, and persists the expanded sidebar width", (
    tester,
  ) async {
    tester.view.physicalSize = const Size(1280, 800);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);
    const preferences = AmePreferences(sidebarWidth: 300);
    final preferenceStore = _RecordingAmePreferenceStore(preferences);

    await tester.pumpWidget(
      ProviderScope(
        overrides: [
          initialAmePreferencesProvider.overrideWithValue(preferences),
          amePreferenceStoreProvider.overrideWithValue(preferenceStore),
        ],
        child: const AmeApp(),
      ),
    );
    await tester.pump();

    expect(
      tester.getSize(find.byKey(const Key("library-navigation"))).width,
      300,
    );
    final resizeHandle = find.byKey(const Key("library-sidebar-resize-handle"));
    final navigation = find.byKey(const Key("library-navigation"));
    final mainSurface = find.byKey(const Key("library-main-surface"));
    final boundaryX = tester.getTopRight(navigation).dx;
    expect(tester.getTopLeft(mainSurface).dx, boundaryX);
    expect(
      tester.getSize(resizeHandle).width,
      LibraryNavigationResizeHandle.hitTargetWidth,
    );
    expect(tester.getCenter(resizeHandle).dx, boundaryX);

    await tester.drag(resizeHandle, const Offset(64, 0));
    await tester.pump();

    expect(
      tester.getSize(find.byKey(const Key("library-navigation"))).width,
      closeTo(364, 1),
    );
    expect(preferenceStore.saved.last.sidebarWidth, closeTo(364, 1));
    expect(
      find.descendant(
        of: resizeHandle,
        matching: find.byType(AnimatedContainer),
      ),
      findsNothing,
    );

    final handleCenter = tester.getCenter(resizeHandle);
    await tester.tapAt(handleCenter);
    await tester.pump(const Duration(milliseconds: 50));
    await tester.tapAt(handleCenter);
    await tester.pump(const Duration(milliseconds: 100));
    expect(
      tester.getSize(find.byKey(const Key("library-navigation"))).width,
      ameDefaultSidebarWidth,
    );
    expect(preferenceStore.saved.last.sidebarWidth, ameDefaultSidebarWidth);

    final anchoredHandleCenter = tester.getCenter(resizeHandle);
    final gesture = await tester.startGesture(anchoredHandleCenter);
    await gesture.moveBy(const Offset(240, 0));
    await tester.pump();
    expect(
      tester.getSize(find.byKey(const Key("library-navigation"))).width,
      ameMaximumSidebarWidth,
    );

    await gesture.moveBy(const Offset(-40, 0));
    await tester.pump();
    expect(
      tester.getSize(find.byKey(const Key("library-navigation"))).width,
      ameMaximumSidebarWidth,
    );

    await gesture.moveBy(const Offset(-80, 0));
    await gesture.up();
    await tester.pump(const Duration(milliseconds: 100));
    expect(
      tester.getSize(find.byKey(const Key("library-navigation"))).width,
      closeTo(380, 1),
    );
    expect(preferenceStore.saved.last.sidebarWidth, closeTo(380, 1));
  });

  testWidgets("keeps the fused window bar usable at minimum width", (
    tester,
  ) async {
    tester.view.physicalSize = const Size(800, 560);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);

    await tester.pumpWidget(const ProviderScope(child: AmeApp()));
    await tester.pump();

    expect(tester.takeException(), isNull);
    expect(find.text("Cedarflake Ame"), findsNothing);
    expect(find.text("Ame"), findsNothing);
    expect(find.byKey(const Key("library-search")), findsOneWidget);
    expect(find.byKey(const Key("library-import-button")), findsNothing);
    expect(find.byKey(const Key("library-sidebar-import")), findsOneWidget);
    expect(find.byKey(const Key("library-sidebar-settings")), findsOneWidget);
    expect(find.byKey(const Key("window-minimize")), findsOneWidget);
    expect(find.byKey(const Key("window-maximize")), findsOneWidget);
    expect(find.byKey(const Key("window-close")), findsOneWidget);
    final galleryHeader = tester.getRect(
      find.byKey(const Key("library-gallery-header")),
    );
    final galleryTitle = tester.getRect(
      find.byKey(const Key("library-gallery-title")),
    );
    expect(galleryHeader.left, 77);
    expect(galleryHeader.right, 800);
    expect(galleryTitle.left, 105);
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
              displayRootPath: "C:\\Pictures",
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

  testWidgets("shows determinate progress while validating staged images", (
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
              displayRootPath: "C:\\Pictures",
              visitedEntries: 50304,
              stagedAssetCount: 48384,
              scanPhase: LibraryScanPhase.finalizing,
              validatedAssetCount: 128,
              validationAssetCount: 48384,
            ),
          ),
        ],
        child: const AmeApp(),
      ),
    );

    expect(find.text("正在核对文件夹“Pictures”…"), findsOneWidget);
    expect(find.text("正在核对 128 / 48384 张图片 · 已检查 50304 个文件"), findsOneWidget);
    final progress = tester.widget<LinearProgressIndicator>(
      find.byType(LinearProgressIndicator),
    );
    expect(progress.value, closeTo(128 / 48384, 0.000001));
  });

  testWidgets("keeps completed import feedback until it is acknowledged", (
    tester,
  ) async {
    tester.view.physicalSize = const Size(1280, 800);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);
    final state = _populatedState(totalItems: 64).copyWith(
      scanId: "scan-completed",
      rootPath: "C:\\Pictures",
      displayRootPath: "C:\\Pictures",
      visitedEntries: 128,
      stagedAssetCount: 64,
    );

    await tester.pumpWidget(
      ProviderScope(
        overrides: [
          initialLibraryStateProvider.overrideWithValue(state),
          libraryScannerProvider.overrideWithValue(const _NoopLibraryScanner()),
        ],
        child: const AmeApp(),
      ),
    );
    await tester.pump();

    final container = ProviderScope.containerOf(
      tester.element(find.byType(AmeApp)),
    );
    expect(container.read(libraryControllerProvider).scanId, "scan-completed");
    expect(find.byKey(const Key("library-task-surface")), findsOneWidget);
    expect(find.text("导入完成"), findsOneWidget);
    expect(find.text("已检查 128 个文件 · 已导入 64 张图片"), findsOneWidget);
    expect(
      find.byKey(const Key("library-task-dismiss-button")),
      findsOneWidget,
    );
    final taskSurface = tester.widget<Material>(
      find.byKey(const Key("library-task-surface")),
    );
    final theme = Theme.of(
      tester.element(find.byKey(const Key("library-task-surface"))),
    );
    expect(taskSurface.color, theme.snackBarTheme.backgroundColor);
    expect(taskSurface.elevation, theme.snackBarTheme.elevation);
    expect(
      tester.getSize(find.byKey(const Key("library-task-surface"))).width,
      theme.snackBarTheme.width,
    );

    await tester.tap(find.byKey(const Key("library-task-dismiss-button")));
    await tester.pump();

    expect(find.text("导入完成"), findsNothing);
    expect(find.byKey(const Key("library-task-dismiss-button")), findsNothing);
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
    _expectSelectedMenuChoice(
      tester,
      label: LibraryStrings.captureDate,
      icon: Symbols.calendar_month_rounded,
    );
    _expectSelectedMenuChoice(
      tester,
      label: LibraryStrings.descending,
      icon: Symbols.arrow_downward_rounded,
    );
    _expectUnselectedMenuChoice(
      tester,
      label: LibraryStrings.fileName,
      icon: Symbols.text_fields_rounded,
    );
    await tester.tap(find.byKey(const Key("library-sort-menu")));
    await tester.pumpAndSettle();
    expect(find.text(LibraryStrings.createdDate), findsNothing);

    await tester.tap(find.byKey(const Key("library-sort-menu")));
    await tester.pumpAndSettle();
    await tester.tap(find.text(LibraryStrings.fileName).last);
    await tester.pumpAndSettle();
    expect(catalog.loadQueries.last.sortKey, LibraryGallerySortKey.fileName);
    expect(catalog.loadQueries.last.searchText, "one");
    expect(catalog.loadQueries.last.rootId, "root-1");
    expect(find.text(LibraryStrings.unknownCaptureDate), findsNothing);
  });

  testWidgets("keeps the sort menu hit-testable inside the desktop viewport", (
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

    final button = find.byKey(const Key("library-sort-menu"));
    final sortAnchor = find.ancestor(
      of: button,
      matching: find.byType(MenuAnchor),
    );
    expect(button, findsOneWidget);
    expect(sortAnchor, findsOneWidget);
    final controller = tester.widget<MenuAnchor>(sortAnchor).controller;
    expect(controller?.isOpen, isFalse);
    final viewRect = Offset.zero & tester.view.physicalSize;
    expect(viewRect.contains(tester.getCenter(button)), isTrue);
    await tester.tap(button);
    await tester.pump(const Duration(milliseconds: 300));
    expect(controller?.isOpen, isTrue);

    for (final label in const [
      LibraryStrings.captureDate,
      LibraryStrings.createdDate,
      LibraryStrings.modifiedDate,
      LibraryStrings.fileName,
      LibraryStrings.ascending,
      LibraryStrings.descending,
    ]) {
      _expectMenuLabelFullyVisible(tester, label);
    }
    final item = _activeMenuItem(tester, LibraryStrings.fileName);
    final itemRect = tester.getRect(item);
    expect(itemRect.left, greaterThanOrEqualTo(AmeMenuMetrics.viewportPadding));
    expect(
      itemRect.right,
      lessThanOrEqualTo(viewRect.right - AmeMenuMetrics.viewportPadding),
    );
    expect(itemRect.top, greaterThanOrEqualTo(AmeMenuMetrics.viewportPadding));
    expect(
      itemRect.bottom,
      lessThanOrEqualTo(viewRect.bottom - AmeMenuMetrics.viewportPadding),
    );
    await tester.tapAt(itemRect.center);
    await tester.pump();
    expect(controller?.isOpen, isFalse);
  });

  testWidgets(
    "keeps the visible location anchored through real sort query transitions",
    (tester) async {
      final previousHitTestWarningPolicy =
          WidgetController.hitTestWarningShouldBeFatal;
      WidgetController.hitTestWarningShouldBeFatal = true;
      tester.view.physicalSize = const Size(1280, 800);
      tester.view.devicePixelRatio = 1;
      addTearDown(tester.view.resetPhysicalSize);
      addTearDown(tester.view.resetDevicePixelRatio);
      addTearDown(() {
        WidgetController.hitTestWarningShouldBeFatal =
            previousHitTestWarningPolicy;
      });
      final assets = [
        for (var index = 0; index < 120; index++)
          _galleryAsset(
            id: "sort-anchor-$index",
            captureLocalTime: "2026-08-05T00:00:00.000000000",
          ),
      ];
      final initialSnapshot = LibrarySnapshot(
        catalogPath: "C:\\AmeData\\ame.sqlite3",
        revision: BigInt.one,
        queryId: "sort-anchor-initial",
        roots: const [
          LibraryRoot(
            id: "root-1",
            path: "C:\\Pictures",
            displayPath: "C:\\Pictures",
            createdUnixMs: 1,
            assetCount: 120,
            issueCount: 0,
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
      final catalog = _AnchorResolvingQueryCatalog(initialSnapshot);

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
        ],
      );
      addTearDown(container.dispose);
      await tester.pumpWidget(
        UncontrolledProviderScope(container: container, child: const AmeApp()),
      );
      await tester.pump();

      final wall = find.byKey(const Key("library-photo-wall"));
      final scrollPosition = _galleryScrollPosition(tester);
      scrollPosition.jumpTo(scrollPosition.maxScrollExtent * 0.55);
      await tester.pump();
      await tester.pump();
      final wallTop = tester.getTopLeft(wall).dy;
      final visibleTopsBefore = <String, double>{};
      for (final asset in assets) {
        final finder = find.byKey(ValueKey(asset.locationId));
        if (finder.evaluate().length == 1) {
          visibleTopsBefore[asset.locationId] =
              tester.getTopLeft(finder).dy - wallTop;
        }
      }
      expect(visibleTopsBefore, isNotEmpty);
      final deepScrollPixels = scrollPosition.pixels;

      final fileNameItem = await _openSortMenu(tester, LibraryStrings.fileName);
      await tester.tapAt(tester.getCenter(fileNameItem));
      await tester.pump();

      expect(catalog.requestedLocationIds, hasLength(1));
      expect(
        container.read(libraryControllerProvider).queryId,
        catalog.lastSnapshot!.queryId,
      );
      await tester.pump();
      final resolvedLocationId = catalog.requestedLocationIds.single;
      expect(visibleTopsBefore, contains(resolvedLocationId));
      final resolvedFinder = find.byKey(ValueKey(resolvedLocationId));
      expect(resolvedFinder, findsOneWidget);
      final topAfterFirstLayout =
          tester.getTopLeft(resolvedFinder).dy - tester.getTopLeft(wall).dy;
      expect(
        topAfterFirstLayout,
        closeTo(visibleTopsBefore[resolvedLocationId]!, 2),
      );
      expect(scrollPosition.pixels, greaterThan(0));
      expect(scrollPosition.pixels, lessThan(scrollPosition.maxScrollExtent));
      final pixelsAfterFirstLayout = scrollPosition.pixels;
      await tester.pump();
      expect(scrollPosition.pixels, closeTo(pixelsAfterFirstLayout, 0.01));
      expect(
        tester.getTopLeft(resolvedFinder).dy - tester.getTopLeft(wall).dy,
        closeTo(topAfterFirstLayout, 0.01),
      );

      catalog.shouldResolveAnchor = false;
      final createdDateItem = await _openSortMenu(
        tester,
        LibraryStrings.createdDate,
      );
      await tester.tapAt(tester.getCenter(createdDateItem));
      await tester.pump();

      expect(catalog.requestedLocationIds, hasLength(2));
      expect(catalog.lastSnapshot?.queryAnchorResolution, isNull);
      expect(
        container.read(libraryControllerProvider).queryId,
        catalog.lastSnapshot!.queryId,
      );
      await tester.pump();
      final fallbackFirst = catalog.lastSnapshot!.assets.first.locationId;
      expect(find.byKey(ValueKey(fallbackFirst)), findsOneWidget);
      final fallbackTop =
          tester.getTopLeft(find.byKey(ValueKey(fallbackFirst))).dy -
          tester.getTopLeft(wall).dy;
      expect(fallbackTop, closeTo(0, 0.01));
      expect(scrollPosition.pixels, greaterThanOrEqualTo(0));
      expect(scrollPosition.pixels, lessThan(deepScrollPixels));
      final fallbackPixels = scrollPosition.pixels;
      await tester.pump();
      expect(scrollPosition.pixels, closeTo(fallbackPixels, 0.01));
    },
  );

  testWidgets("keeps the frozen anchor when a pending sort returns to base", (
    tester,
  ) async {
    final previousHitTestWarningPolicy =
        WidgetController.hitTestWarningShouldBeFatal;
    WidgetController.hitTestWarningShouldBeFatal = true;
    tester.view.physicalSize = const Size(1280, 800);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);
    addTearDown(() {
      WidgetController.hitTestWarningShouldBeFatal =
          previousHitTestWarningPolicy;
    });
    final assets = [
      for (var index = 0; index < 120; index++)
        _galleryAsset(
          id: "cancel-anchor-$index",
          captureLocalTime: "2026-08-05T00:00:00.000000000",
        ),
    ];
    final snapshot = LibrarySnapshot(
      catalogPath: "C:\\AmeData\\ame.sqlite3",
      revision: BigInt.one,
      queryId: "cancel-anchor-initial",
      roots: const [
        LibraryRoot(
          id: "root-1",
          path: "C:\\Pictures",
          displayPath: "C:\\Pictures",
          createdUnixMs: 1,
          assetCount: 120,
          issueCount: 0,
        ),
      ],
      assets: assets,
    );
    final initialState = LibraryState.fromSnapshot(snapshot).copyWith(
      timeline: LibraryTimeline(
        revision: BigInt.one,
        queryId: snapshot.queryId,
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
    final catalog = _AnchorResolvingQueryCatalog(snapshot)
      ..holdNextAnchorRequest = true;
    addTearDown(catalog.releaseHeldAnchorRequest);

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
      ],
    );
    addTearDown(container.dispose);
    await tester.pumpWidget(
      UncontrolledProviderScope(container: container, child: const AmeApp()),
    );
    await tester.pump();

    final wall = find.byKey(const Key("library-photo-wall"));
    final scrollPosition = _galleryScrollPosition(tester);
    scrollPosition.jumpTo(scrollPosition.maxScrollExtent * 0.55);
    await tester.pump();
    await tester.pump();
    final visibleBefore = <String, double>{};
    final wallTop = tester.getTopLeft(wall).dy;
    for (final asset in assets) {
      final finder = find.byKey(ValueKey(asset.locationId));
      if (finder.evaluate().length == 1) {
        visibleBefore[asset.locationId] =
            tester.getTopLeft(finder).dy - wallTop;
      }
    }

    final fileNameItem = await _openSortMenu(tester, LibraryStrings.fileName);
    await tester.tapAt(tester.getCenter(fileNameItem));
    await tester.pump();
    expect(catalog.requestedLocationIds, hasLength(1));
    final frozenLocationId = catalog.requestedLocationIds.single;
    final frozenFinder = find.byKey(ValueKey(frozenLocationId));
    expect(visibleBefore, contains(frozenLocationId));

    final captureDateItem = await _openSortMenu(
      tester,
      LibraryStrings.captureDate,
    );
    await tester.tapAt(tester.getCenter(captureDateItem));
    await tester.pump();
    expect(container.read(libraryControllerProvider).queryId, snapshot.queryId);
    await tester.pump();

    expect(frozenFinder, findsOneWidget);
    final topAfterCancel =
        tester.getTopLeft(frozenFinder).dy - tester.getTopLeft(wall).dy;
    expect(topAfterCancel, closeTo(visibleBefore[frozenLocationId]!, 2));
    expect(scrollPosition.pixels, greaterThan(0));
    expect(scrollPosition.pixels, lessThan(scrollPosition.maxScrollExtent));
    final pixelsAfterCancel = scrollPosition.pixels;

    catalog.releaseHeldAnchorRequest();
    await tester.pump();
    await tester.pump();
    expect(scrollPosition.pixels, closeTo(pixelsAfterCancel, 0.01));
    expect(
      tester.getTopLeft(frozenFinder).dy - tester.getTopLeft(wall).dy,
      closeTo(topAfterCancel, 0.01),
    );
  });

  testWidgets(
    "freezes the native offset when layout controls change geometry",
    (tester) async {
      final previousHitTestWarningPolicy =
          WidgetController.hitTestWarningShouldBeFatal;
      WidgetController.hitTestWarningShouldBeFatal = true;
      tester.view.physicalSize = const Size(1280, 800);
      tester.view.devicePixelRatio = 1;
      addTearDown(tester.view.resetPhysicalSize);
      addTearDown(tester.view.resetDevicePixelRatio);
      addTearDown(() {
        WidgetController.hitTestWarningShouldBeFatal =
            previousHitTestWarningPolicy;
      });
      final assets = [
        for (var index = 0; index < 120; index++)
          _galleryAsset(
            id: "geometry-anchor-$index",
            captureLocalTime: "2026-08-05T00:00:00.000000000",
          ),
      ];
      final snapshot = LibrarySnapshot(
        catalogPath: "C:\\AmeData\\ame.sqlite3",
        revision: BigInt.one,
        queryId: "geometry-anchor-initial",
        roots: const [
          LibraryRoot(
            id: "root-1",
            path: "C:\\Pictures",
            displayPath: "C:\\Pictures",
            createdUnixMs: 1,
            assetCount: 120,
            issueCount: 0,
          ),
        ],
        assets: assets,
      );
      final initialState = LibraryState.fromSnapshot(snapshot).copyWith(
        timeline: LibraryTimeline(
          revision: BigInt.one,
          queryId: snapshot.queryId,
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

      await tester.pumpWidget(
        ProviderScope(
          overrides: [
            initialLibraryStateProvider.overrideWithValue(initialState),
            initialLibraryViewPreferencesProvider.overrideWithValue(
              const LibraryViewPreferences(
                layoutShape: GalleryLayoutShape.equalHeight,
                thumbnailSize: GalleryThumbnailSize.medium,
              ),
            ),
          ],
          child: const AmeApp(),
        ),
      );
      await tester.pump();

      final wall = find.byKey(const Key("library-photo-wall"));
      final scrollPosition = _galleryScrollPosition(tester);
      scrollPosition.jumpTo(scrollPosition.maxScrollExtent * 0.55);
      final wallTop = tester.getTopLeft(wall).dy;

      final squareItem = await _openLayoutMenu(tester, LibraryStrings.square);
      final squareAnchor = _viewportCenterRowAnchor(tester, assets);
      await tester.tapAt(tester.getCenter(squareItem));
      await _pumpUntilGalleryGeometry(
        tester,
        previousShape: GalleryLayoutShape.equalHeight,
        expectedShape: GalleryLayoutShape.square,
        expectedSize: GalleryThumbnailSize.medium,
      );
      final squareFinder = find.byKey(ValueKey(squareAnchor.locationId));
      expect(squareFinder, findsOneWidget);
      final squareRect = tester.getRect(squareFinder);
      final squareTop = tester.getTopLeft(squareFinder).dy - wallTop;
      expect(
        squareRect.top,
        closeTo(
          squareAnchor.viewportCenterY -
              squareRect.height * squareAnchor.itemFraction,
          2,
        ),
      );
      final squarePixels = scrollPosition.pixels;
      await tester.pump();
      expect(scrollPosition.pixels, closeTo(squarePixels, 0.01));
      expect(
        tester.getTopLeft(squareFinder).dy - wallTop,
        closeTo(squareTop, 0.01),
      );

      final largeItem = await _openLayoutMenu(tester, LibraryStrings.large);
      final largeAnchor = _viewportCenterRowAnchor(tester, assets);
      await tester.tapAt(tester.getCenter(largeItem));
      await _pumpUntilGalleryGeometry(
        tester,
        previousShape: GalleryLayoutShape.square,
        expectedShape: GalleryLayoutShape.square,
        previousSize: GalleryThumbnailSize.medium,
        expectedSize: GalleryThumbnailSize.large,
      );
      final largeFinder = find.byKey(ValueKey(largeAnchor.locationId));
      expect(largeFinder, findsOneWidget);
      final largeRect = tester.getRect(largeFinder);
      final largeTop = tester.getTopLeft(largeFinder).dy - wallTop;
      expect(
        largeRect.top,
        closeTo(
          largeAnchor.viewportCenterY -
              largeRect.height * largeAnchor.itemFraction,
          2,
        ),
      );
      final largePixels = scrollPosition.pixels;
      await tester.pump();
      expect(scrollPosition.pixels, closeTo(largePixels, 0.01));
      expect(
        tester.getTopLeft(largeFinder).dy - wallTop,
        closeTo(largeTop, 0.01),
      );
    },
  );

  testWidgets("restores and saves stable gallery display preferences", (
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
        queryId: "preferences-result",
        roots: initialState.roots,
        assets: initialState.assets,
      ),
    );
    const initialPreferences = LibraryViewPreferences(
      layoutShape: GalleryLayoutShape.square,
      thumbnailSize: GalleryThumbnailSize.large,
    );
    final preferenceStore = _RecordingViewPreferenceStore(initialPreferences);

    await tester.pumpWidget(
      ProviderScope(
        overrides: [
          initialLibraryStateProvider.overrideWithValue(initialState),
          initialLibraryViewPreferencesProvider.overrideWithValue(
            initialPreferences,
          ),
          libraryCatalogProvider.overrideWithValue(catalog),
          libraryViewPreferenceStoreProvider.overrideWithValue(preferenceStore),
        ],
        child: const AmeApp(),
      ),
    );
    await tester.pump();

    await tester.tap(find.byKey(const Key("library-layout-menu")));
    await tester.pumpAndSettle();
    expect(find.byType(AmeMenuItemContent), findsNWidgets(5));
    _expectSelectedMenuChoice(
      tester,
      label: LibraryStrings.square,
      icon: Symbols.grid_view_rounded,
    );
    _expectSelectedMenuChoice(
      tester,
      label: LibraryStrings.large,
      icon: Symbols.crop_square_rounded,
    );
    await tester.tap(find.byKey(const Key("library-layout-menu")));
    await tester.pumpAndSettle();
    expect(find.text(LibraryStrings.equalHeight), findsNothing);

    await tester.tap(find.byKey(const Key("library-layout-menu")));
    await tester.pumpAndSettle();
    await tester.tap(find.text(LibraryStrings.equalHeight));
    await tester.pumpAndSettle();
    await tester.tap(find.byKey(const Key("library-layout-menu")));
    await tester.pumpAndSettle();
    await tester.tap(find.text(LibraryStrings.small));
    await tester.pumpAndSettle();
    await tester.tap(find.byKey(const Key("library-sort-menu")));
    await tester.pumpAndSettle();
    await tester.tap(find.text(LibraryStrings.fileName).last);
    await tester.pumpAndSettle();

    expect(preferenceStore.saved, isNotEmpty);
    expect(
      preferenceStore.saved.last.layoutShape,
      GalleryLayoutShape.equalHeight,
    );
    expect(
      preferenceStore.saved.last.thumbnailSize,
      GalleryThumbnailSize.small,
    );
    expect(preferenceStore.saved.last.sortKey, LibraryGallerySortKey.fileName);
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
          _galleryAsset(
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
      final catalog = _AnchorResolvingQueryCatalog(initialSnapshot);
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

      final scrollPosition = _galleryScrollPosition(tester);
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
      displayRootPath: "C:\\Documents",
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

  testWidgets("uses one top line for scroll-triggered page loading", (
    tester,
  ) async {
    tester.view.physicalSize = const Size(1280, 800);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);
    final state = _populatedState(
      totalItems: 2,
    ).copyWith(isLoadingPage: true, isLoadingPreviousPage: true);

    await tester.pumpWidget(
      ProviderScope(
        overrides: [initialLibraryStateProvider.overrideWithValue(state)],
        child: const AmeApp(),
      ),
    );
    await tester.pump();

    expect(find.byKey(const Key("library-top-loading")), findsOneWidget);
    expect(
      find.descendant(
        of: find.byKey(const Key("library-photo-wall")),
        matching: find.byType(CircularProgressIndicator),
      ),
      findsNothing,
    );
  });

  testWidgets(
    "keeps keyset continuation automatic without visible pagination",
    (tester) async {
      tester.view.physicalSize = const Size(1280, 800);
      tester.view.devicePixelRatio = 1;
      addTearDown(tester.view.resetPhysicalSize);
      addTearDown(tester.view.resetDevicePixelRatio);
      final fallbackUnixMs = DateTime(2024, 3, 4).millisecondsSinceEpoch;
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
            displayPath: "C:\\Pictures\\one.png",
            relativePath: "one.png",
            previewPath: "C:\\Missing\\one.jpg",
            fileSize: BigInt.one,
            modifiedUnixMs: fallbackUnixMs,
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
      expect(find.byKey(const Key("gallery-date-2024-03-04")), findsOneWidget);
      final summary = tester.widget<Text>(
        find.byKey(const Key("library-summary")),
      );
      expect(summary.data, "2 张图片");
    },
  );

  testWidgets("groups server-ordered assets under capture and fallback dates", (
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
          displayPath: "C:\\Pictures",
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
          displayPath: "C:\\Pictures\\one.png",
          relativePath: "one.png",
          previewPath: "C:\\Missing\\one.jpg",
          fileSize: BigInt.one,
          modifiedUnixMs: 3,
          width: 1,
          height: 1,
          previewStatus: LibraryPreviewStatus.pending,
          captureTime: capture,
        ),
        LibraryAsset(
          assetId: "asset-2",
          locationId: "location-2",
          rootId: "root-1",
          sourcePath: "C:\\Pictures\\two.png",
          displayPath: "C:\\Pictures\\two.png",
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
          sourcePath: "C:\\Pictures\\fallback.png",
          displayPath: "C:\\Pictures\\fallback.png",
          relativePath: "fallback.png",
          previewPath: "C:\\Missing\\fallback.jpg",
          fileSize: BigInt.one,
          createdUnixMs: DateTime(2024, 3, 4).millisecondsSinceEpoch,
          modifiedUnixMs: DateTime(2026, 5, 6).millisecondsSinceEpoch,
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
    final fallbackHeader = find.byKey(const Key("gallery-date-2024-03-04"));
    expect(datedHeader, findsOneWidget);
    expect(fallbackHeader, findsOneWidget);
    expect(find.byKey(const Key("gallery-date-unknown")), findsNothing);
    expect(
      tester.getTopLeft(datedHeader).dy,
      lessThan(tester.getTopLeft(fallbackHeader).dy),
    );
    expect(find.byKey(const ValueKey("location-1")), findsOneWidget);
    expect(find.byKey(const ValueKey("location-2")), findsOneWidget);
    expect(find.byKey(const ValueKey("location-3")), findsOneWidget);
    final gallerySliver = tester.widget<LibraryExactExtentSliver>(
      find.byType(LibraryExactExtentSliver),
    );
    expect(
      (gallerySliver.delegate as SliverChildBuilderDelegate).addSemanticIndexes,
      isFalse,
    );
    expect(
      find.descendant(
        of: find.byKey(const ValueKey("location-1")),
        matching: find.byType(CircularProgressIndicator),
      ),
      findsNothing,
    );
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
          displayPath: "E:\\DisconnectedPictures",
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
    expect(find.byIcon(Symbols.folder_off_rounded), findsOneWidget);
  });

  testWidgets("reveals a checkbox on hover and keeps selection visible", (
    tester,
  ) async {
    tester.view.physicalSize = const Size(1280, 800);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);
    final state = _populatedState(
      totalItems: 1,
      assetWidth: 1,
      assetHeight: 1000,
    );

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
    final tileRect = tester.getRect(tile);
    final checkboxRect = tester.getRect(checkbox);
    expect(checkboxRect.left, greaterThanOrEqualTo(tileRect.left));
    expect(checkboxRect.right, lessThanOrEqualTo(tileRect.right));
    expect(checkboxRect.top, greaterThanOrEqualTo(tileRect.top));
    expect(checkboxRect.bottom, lessThanOrEqualTo(tileRect.bottom));

    await tester.tap(checkbox);
    await tester.pump();
    expect(find.byKey(const Key("library-selection-toolbar")), findsOneWidget);
    expect(find.text("已选择 1 个项目"), findsOneWidget);
    expect(find.text("查看"), findsNothing);

    await mouse.moveTo(Offset.zero);
    await tester.pump();
    expect(checkbox, findsOneWidget);

    await tester.tap(checkbox);
    await tester.pump();
    expect(find.byKey(const Key("library-selection-toolbar")), findsNothing);
    expect(find.text("已选择 1 个项目"), findsNothing);
    expect(find.byKey(const Key("library-browsing-toolbar")), findsOneWidget);
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
    expect(find.byType(AmeMenuItemContent), findsNWidgets(4));
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

    final sourceMore = find.byKey(const ValueKey("source-more-root-1"));
    final sourceMoreRect = tester.getRect(sourceMore);
    await tester.tap(sourceMore);
    await tester.pumpAndSettle();
    expect(find.text("更新图库"), findsOneWidget);
    expect(find.text("在文件资源管理器中打开"), findsOneWidget);
    expect(find.text("从 Ame 中移除"), findsOneWidget);
    expect(find.byType(AmeMenuItemContent), findsNWidgets(3));
    final updateMenuItem = find.ancestor(
      of: find.text("更新图库"),
      matching: find.byType(MenuItemButton),
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
    expect(find.text("不选择任何项目"), findsNothing);
    final selectAllMenuItem = find.ancestor(
      of: find.text("全选"),
      matching: find.byType(MenuItemButton),
    );
    final shortcutFinder = find.text("Ctrl+A");
    final shortcut = tester.widget<Text>(shortcutFinder);
    expect(shortcut.softWrap, isFalse);
    expect(shortcut.overflow, isNull);
    expect(
      tester.getRect(shortcutFinder).right,
      lessThanOrEqualTo(tester.getRect(selectAllMenuItem).right),
    );
    expect(
      tester.view.physicalSize.width - tester.getRect(selectAllMenuItem).right,
      closeTo(AmeMenuMetrics.viewportPadding, 1),
    );
    final moreButtonCenter = tester.getCenter(
      find.byKey(const Key("library-more-menu")),
    );
    await tester.tapAt(moreButtonCenter);
    await tester.pumpAndSettle();
    expect(find.text("全选"), findsNothing);

    await tester.tap(find.byKey(const Key("library-more-menu")));
    await tester.pumpAndSettle();
    await tester.tap(find.text("全选"));
    await tester.pumpAndSettle();

    expect(find.text("已选择 79013 个项目"), findsOneWidget);
    expect(find.byKey(const Key("library-cancel-selection")), findsOneWidget);
  });

  testWidgets("renders the scroll-derived Material timeline", (tester) async {
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
    await tester.pumpAndSettle();

    expect(find.byKey(const Key("library-time-rail")), findsOneWidget);
    expect(find.byKey(const Key("timeline-slider")), findsOneWidget);
    expect(
      find.byKey(const Key("current-month-native-scrollbar")),
      findsNothing,
    );
    expect(find.byKey(const ValueKey("time-label-2026-08")), findsOneWidget);
    expect(find.byKey(const ValueKey("time-marker-2026-08")), findsOneWidget);
    expect(find.byKey(const ValueKey("time-marker-unknown")), findsNothing);
    expect(
      find.descendant(
        of: find.byKey(const Key("library-time-rail")),
        matching: find.byType(MenuAnchor),
      ),
      findsNothing,
    );
  });

  testWidgets(
    "preserves the first timeline target when the manifest takes ownership",
    (tester) async {
      tester.view.physicalSize = const Size(1280, 800);
      tester.view.devicePixelRatio = 1;
      addTearDown(tester.view.resetPhysicalSize);
      addTearDown(tester.view.resetDevicePixelRatio);
      final initialState = _populatedState(totalItems: 120);
      final snapshot = LibrarySnapshot(
        catalogPath: initialState.catalogPath ?? "",
        revision: initialState.catalogRevision ?? BigInt.zero,
        queryId: initialState.queryId,
        roots: initialState.roots,
        assets: initialState.assets,
      );
      final catalog = _RecordingQueryCatalog(snapshot);
      final manifestLoader = _ControlledLayoutManifestLoader();

      await tester.pumpWidget(
        ProviderScope(
          overrides: [
            initialLibraryStateProvider.overrideWithValue(initialState),
            libraryCatalogProvider.overrideWithValue(catalog),
            libraryGalleryLayoutManifestLoaderProvider.overrideWithValue(
              manifestLoader,
            ),
          ],
          child: const AmeApp(),
        ),
      );
      await tester.pump();
      await tester.pump();

      var slider = tester.widget<Slider>(
        find.byKey(const Key("timeline-slider")),
      );
      slider.onChangeStart?.call(slider.value);
      slider.onChanged?.call(0.25);
      slider.onChangeEnd?.call(0.25);
      await tester.pump();
      await tester.pumpAndSettle();

      expect(catalog.timeAnchors, hasLength(1));
      final selectedOffset = catalog.timeAnchors.single.itemOffset;
      expect(selectedOffset, greaterThan(0));

      manifestLoader.complete(
        _queryWideManifest(
          totalItems: 120,
          loadedItemIndex: selectedOffset,
          loadedLocationId: initialState.assets.single.locationId,
        ),
      );
      await tester.pump();
      await tester.pumpAndSettle();

      expect(_galleryScrollPosition(tester).pixels, greaterThan(0));
      expect(
        find.byKey(ValueKey(initialState.assets.single.locationId)),
        findsOneWidget,
      );
      expect(
        tester.widget<Slider>(find.byKey(const Key("timeline-slider"))).value,
        greaterThan(0),
      );
    },
  );

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
          displayPath: "C:\\Pictures\\$index.png",
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
          displayPath: "C:\\Pictures",
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
    expect(tester.widget<Slider>(timelineSlider).value, lessThan(1));
    expect(catalog.timeAnchors, isEmpty);

    final beforeSlider = galleryPosition.pixels;
    tester.widget<Slider>(timelineSlider).onChanged?.call(0.25);
    await tester.pump();
    expect(galleryPosition.pixels, greaterThan(beforeSlider));
    expect(tester.widget<Slider>(timelineSlider).value, closeTo(0.25, 0.001));

    final beforeArrow = galleryPosition.pixels;
    await tester.tap(find.byKey(const Key("timeline-previous")));
    await tester.pumpAndSettle();
    expect(galleryPosition.pixels, lessThan(beforeArrow));
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
          displayPath: "C:\\Pictures",
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

      tester.view.physicalSize = const Size(1180, 760);
      await tester.pump();
      await tester.pump();

      expect(tester.takeException(), isNull);
      expect(find.byKey(const ValueKey("anchor-0")), findsOneWidget);
      expect(catalog.beforeCursors, isEmpty);

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
    displayPath: "C:\\Pictures\\$id.png",
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

void _expectSelectedMenuChoice(
  WidgetTester tester, {
  required String label,
  required IconData icon,
}) {
  final item = _menuItem(label);
  expect(
    find.descendant(of: item, matching: find.byIcon(icon)),
    findsOneWidget,
  );
  expect(
    find.descendant(of: item, matching: find.byIcon(Symbols.circle_rounded)),
    findsOneWidget,
  );
  final semantics = tester.getSemantics(
    find.byKey(ValueKey("menu-choice-$label")),
  );
  expect(semantics.flagsCollection.isChecked, CheckedState.isTrue);
  expect(semantics.flagsCollection.isInMutuallyExclusiveGroup, isTrue);
}

void _expectUnselectedMenuChoice(
  WidgetTester tester, {
  required String label,
  required IconData icon,
}) {
  final item = _menuItem(label);
  expect(
    find.descendant(of: item, matching: find.byIcon(icon)),
    findsOneWidget,
  );
  expect(
    find.descendant(of: item, matching: find.byIcon(Symbols.circle_rounded)),
    findsNothing,
  );
  final semantics = tester.getSemantics(
    find.byKey(ValueKey("menu-choice-$label")),
  );
  expect(semantics.flagsCollection.isChecked, CheckedState.isFalse);
  expect(semantics.flagsCollection.isInMutuallyExclusiveGroup, isTrue);
}

Finder _menuItem(String label) {
  return find.ancestor(
    of: find.text(label).last,
    matching: find.byType(MenuItemButton),
  );
}

Future<Finder> _openSortMenu(WidgetTester tester, String expectedItem) async {
  await tester.pump(const Duration(milliseconds: 300));
  var item = _activeMenuItemOrNull(tester, expectedItem);
  if (item != null) {
    return item;
  }
  final button = find.byKey(const Key("library-sort-menu"));
  await tester.ensureVisible(button);
  await tester.tap(button);
  await tester.pump(const Duration(milliseconds: 300));
  return _activeMenuItem(tester, expectedItem);
}

Future<Finder> _openLayoutMenu(WidgetTester tester, String expectedItem) async {
  final button = find.byKey(const Key("library-layout-menu"));
  await tester.ensureVisible(button);
  await tester.tap(button);
  await tester.pump();
  return _activeMenuItem(tester, expectedItem);
}

({String locationId, double itemFraction, double viewportCenterY})
_viewportCenterRowAnchor(WidgetTester tester, List<LibraryAsset> assets) {
  final scrollable = find.descendant(
    of: find.byKey(const Key("library-photo-wall")),
    matching: find.byType(Scrollable),
  );
  final viewportRect = tester.getRect(scrollable);
  final viewportCenterY = viewportRect.top + viewportRect.height * 0.5;
  final candidates = <({String locationId, Rect rect})>[];
  for (final asset in assets) {
    final finder = find.byKey(ValueKey(asset.locationId));
    if (finder.evaluate().length == 1) {
      final rect = tester.getRect(finder);
      if (rect.top <= viewportCenterY &&
          viewportCenterY < rect.bottom + LibraryGalleryLayoutEntry.spacing) {
        candidates.add((locationId: asset.locationId, rect: rect));
      }
    }
  }
  expect(candidates, isNotEmpty);
  final rowTop = candidates
      .map((candidate) => candidate.rect.top)
      .reduce((first, second) => first > second ? first : second);
  final viewportCenterX = viewportRect.left + viewportRect.width * 0.5;
  final row =
      candidates
          .where((candidate) => (candidate.rect.top - rowTop).abs() < 0.01)
          .toList(growable: false)
        ..sort(
          (first, second) => (first.rect.center.dx - viewportCenterX)
              .abs()
              .compareTo((second.rect.center.dx - viewportCenterX).abs()),
        );
  final anchor = row.first;
  return (
    locationId: anchor.locationId,
    itemFraction: ((viewportCenterY - anchor.rect.top) / anchor.rect.height)
        .clamp(0.0, 1.0)
        .toDouble(),
    viewportCenterY: viewportCenterY,
  );
}

Future<void> _pumpUntilGalleryGeometry(
  WidgetTester tester, {
  required GalleryLayoutShape previousShape,
  required GalleryLayoutShape expectedShape,
  GalleryThumbnailSize? previousSize,
  required GalleryThumbnailSize expectedSize,
}) async {
  const maximumFrames = 3;
  for (var frame = 0; frame < maximumFrames; frame += 1) {
    await tester.pump();
    final header = tester.widget<LibraryGalleryHeader>(
      find.byType(LibraryGalleryHeader),
    );
    if (header.layoutShape == expectedShape &&
        header.thumbnailSize == expectedSize) {
      return;
    }
    expect(header.layoutShape, previousShape);
    if (previousSize != null) {
      expect(header.thumbnailSize, previousSize);
    }
  }
  fail("Gallery geometry was not published within $maximumFrames frames");
}

Finder _activeMenuItem(WidgetTester tester, String label) {
  final item = _activeMenuItemOrNull(tester, label);
  expect(item, isNotNull);
  return item!;
}

Finder? _activeMenuItemOrNull(WidgetTester tester, String label) {
  final candidates = find.ancestor(
    of: find.text(label),
    matching: find.byType(MenuItemButton),
  );
  if (candidates.evaluate().length != 1) {
    return null;
  }
  final offstageAncestors = find
      .ancestor(of: candidates, matching: find.byType(Offstage))
      .evaluate()
      .map((element) => element.widget as Offstage);
  expect(offstageAncestors.every((widget) => !widget.offstage), isTrue);
  final ignoringAncestors = find
      .ancestor(of: candidates, matching: find.byType(IgnorePointer))
      .evaluate()
      .map((element) => element.widget as IgnorePointer);
  expect(ignoringAncestors.every((widget) => !widget.ignoring), isTrue);
  final itemElement = candidates.evaluate().single;
  final hitPath = tester.hitTestOnBinding(tester.getCenter(candidates)).path;
  expect(
    hitPath.any((entry) {
      final target = entry.target;
      if (target is! RenderObject) {
        return false;
      }
      final creator = target.debugCreator;
      if (creator is! DebugCreator) {
        return false;
      }
      var belongsToItem = false;
      void visit(Element element) {
        if (identical(element, creator.element)) {
          belongsToItem = true;
          return;
        }
        element.visitChildElements(visit);
      }

      itemElement.visitChildElements(visit);
      return belongsToItem || identical(creator.element, itemElement);
    }),
    isTrue,
  );
  return candidates;
}

void _expectMenuLabelFullyVisible(WidgetTester tester, String label) {
  final finder = find.text(label).last;
  final text = tester.widget<Text>(finder);
  final context = tester.element(finder);
  final painter = TextPainter(
    text: TextSpan(
      text: text.data,
      style: DefaultTextStyle.of(context).style.merge(text.style),
    ),
    textDirection: Directionality.of(context),
    textScaler: MediaQuery.textScalerOf(context),
    maxLines: 1,
  )..layout();
  expect(
    tester.getSize(finder).width,
    greaterThanOrEqualTo(painter.width),
    reason: "$label should not be ellipsized in the sort menu",
  );
  painter.dispose();
}

LibraryState _populatedState({
  required int totalItems,
  int assetWidth = 4,
  int assetHeight = 3,
}) {
  final snapshot = LibrarySnapshot(
    catalogPath: "C:\\AmeData\\ame.sqlite3",
    revision: BigInt.one,
    queryId: "query-1",
    roots: [
      LibraryRoot(
        id: "root-1",
        path: "C:\\Pictures",
        displayPath: "C:\\Pictures",
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
        displayPath: "C:\\Pictures\\one.png",
        relativePath: "one.png",
        previewPath: "C:\\Missing\\one.jpg",
        fileSize: BigInt.one,
        modifiedUnixMs: 1,
        width: assetWidth,
        height: assetHeight,
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

LibraryGalleryLayoutManifest _unknownDimensionManifest(
  List<LibraryAsset> assets,
) {
  final builder = LibraryGalleryLayoutManifestBuilder(
    revision: BigInt.one,
    queryId: "query-1",
    totalItems: assets.length,
  );
  builder.append(
    LibraryGalleryLayoutManifestChunk(
      revision: BigInt.one,
      queryId: "query-1",
      totalItems: assets.length,
      startOrdinal: 0,
      locationIds: [for (final asset in assets) asset.locationId],
      aspectRatioMilli: Uint16List.fromList(List.filled(assets.length, 1000)),
      dateGroupIndices: Uint16List(assets.length),
      dateGroups: const [null],
      flags: Uint8List(assets.length),
    ),
  );
  return builder.build();
}

LibraryGalleryLayoutManifest _queryWideManifest({
  required int totalItems,
  required int loadedItemIndex,
  required String loadedLocationId,
}) {
  final builder = LibraryGalleryLayoutManifestBuilder(
    revision: BigInt.one,
    queryId: "query-1",
    totalItems: totalItems,
  );
  builder.append(
    LibraryGalleryLayoutManifestChunk(
      revision: BigInt.one,
      queryId: "query-1",
      totalItems: totalItems,
      startOrdinal: 0,
      locationIds: [
        for (var index = 0; index < totalItems; index++)
          index == loadedItemIndex
              ? loadedLocationId
              : "manifest-location-$index",
      ],
      aspectRatioMilli: Uint16List.fromList(List.filled(totalItems, 1333)),
      dateGroupIndices: Uint16List(totalItems),
      dateGroups: const ["2026-08-05"],
      flags: Uint8List.fromList(
        List.filled(totalItems, libraryGalleryLayoutDimensionsKnownFlag),
      ),
    ),
  );
  return builder.build();
}

ScrollPosition _galleryScrollPosition(WidgetTester tester) {
  final scrollable = find.descendant(
    of: find.byKey(const Key("library-photo-wall")),
    matching: find.byType(Scrollable),
  );
  return tester.state<ScrollableState>(scrollable).position;
}

class _FixedLayoutManifestLoader implements LibraryGalleryLayoutManifestLoader {
  const _FixedLayoutManifestLoader(this.manifest);

  final LibraryGalleryLayoutManifest manifest;

  @override
  Future<LibraryGalleryLayoutManifest> load(
    LibraryGalleryQuery query, {
    bool Function()? isCancelled,
  }) async {
    return manifest;
  }
}

class _ControlledLayoutManifestLoader
    implements LibraryGalleryLayoutManifestLoader {
  final Completer<LibraryGalleryLayoutManifest> _completer = Completer();

  @override
  Future<LibraryGalleryLayoutManifest> load(
    LibraryGalleryQuery query, {
    bool Function()? isCancelled,
  }) {
    return _completer.future;
  }

  void complete(LibraryGalleryLayoutManifest manifest) {
    _completer.complete(manifest);
  }
}

class _ControlledLibraryPreviewer implements LibraryPreviewer {
  _ControlledLibraryPreviewer(Iterable<LibraryAsset> assets)
    : _assets = {for (final asset in assets) asset.locationId: asset};

  final Map<String, LibraryAsset> _assets;
  final Map<String, Completer<LibraryAsset>> _completers = {};
  final List<String> requests = [];

  @override
  Future<LibraryAsset> materialize({
    required String locationId,
    required int previewEdge,
    bool retry = false,
    Iterable<String> protectedLocationIds = const [],
  }) {
    requests.add(locationId);
    return (_completers[locationId] ??= Completer<LibraryAsset>()).future;
  }

  void succeed(String locationId, {required int width, required int height}) {
    final asset = _assets[locationId];
    if (asset == null) {
      throw StateError("Unknown preview asset $locationId");
    }
    _completers[locationId]?.complete(
      asset.withPreview(
        previewPath: "C:\\AmeCache\\$locationId.jpg",
        width: width,
        height: height,
        previewStatus: LibraryPreviewStatus.ready,
      ),
    );
  }
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

class _AnchorResolvingQueryCatalog
    implements LibraryCatalog, LibraryQueryAnchorCatalog {
  _AnchorResolvingQueryCatalog(this.initialSnapshot);

  final LibrarySnapshot initialSnapshot;
  final List<String> requestedLocationIds = [];
  bool shouldResolveAnchor = true;
  bool holdNextAnchorRequest = false;
  Completer<void>? _heldAnchorRequest;
  int _generation = 0;
  LibrarySnapshot? lastSnapshot;

  @override
  Future<LibrarySnapshot> loadAroundLocation({
    required int maxItems,
    required LibraryGalleryQuery query,
    required String anchorLocationId,
  }) async {
    requestedLocationIds.add(anchorLocationId);
    if (holdNextAnchorRequest) {
      holdNextAnchorRequest = false;
      _heldAnchorRequest = Completer<void>();
      await _heldAnchorRequest!.future;
    }
    final reordered = List<LibraryAsset>.of(initialSnapshot.assets.reversed);
    final requestedIndex = reordered.indexWhere(
      (asset) => asset.locationId == anchorLocationId,
    );
    if (requestedIndex >= 0) {
      final targetOrdinal = (_generation.isEven ? 62 : 38).clamp(
        0,
        reordered.length - 1,
      );
      final requested = reordered.removeAt(requestedIndex);
      reordered.insert(targetOrdinal, requested);
    }
    _generation += 1;
    final queryId = "sort-anchor-result-$_generation";
    final resolution = shouldResolveAnchor
        ? LibraryQueryAnchorResolution(
            requestedLocationId: anchorLocationId,
            locationId: anchorLocationId,
            ordinal: reordered.indexWhere(
              (asset) => asset.locationId == anchorLocationId,
            ),
            windowStartItemOffset: 0,
          )
        : null;
    return lastSnapshot = LibrarySnapshot(
      catalogPath: initialSnapshot.catalogPath,
      revision: initialSnapshot.revision,
      queryId: queryId,
      roots: initialSnapshot.roots,
      assets: reordered,
      queryAnchorResolution: resolution,
    );
  }

  @override
  Future<LibrarySnapshot> load({
    required int maxItems,
    required LibraryGalleryQuery query,
    LibraryCatalogCursor? after,
    LibraryCatalogCursor? before,
  }) async {
    return initialSnapshot;
  }

  @override
  Future<LibrarySnapshot> loadAtTime({
    required int maxItems,
    required LibraryGalleryQuery query,
    required LibraryTimeAnchor anchor,
  }) async {
    return initialSnapshot;
  }

  @override
  Future<LibraryTimeline> loadTimeline(LibraryGalleryQuery query) async {
    final snapshot = lastSnapshot ?? initialSnapshot;
    return LibraryTimeline(
      revision: snapshot.revision,
      queryId: snapshot.queryId,
      totalItems: snapshot.assets.length,
      buckets: [
        LibraryTimeBucket(
          monthKey: "2026-08",
          itemCount: snapshot.assets.length,
          aspectRatioSum: snapshot.assets.length * 4 / 3,
        ),
      ],
    );
  }

  @override
  Future<bool> unregisterRoot(String rootId) async => true;

  void releaseHeldAnchorRequest() {
    final request = _heldAnchorRequest;
    if (request != null && !request.isCompleted) {
      request.complete();
    }
    _heldAnchorRequest = null;
  }
}

class _NoopLibraryScanner implements LibraryScanner {
  const _NoopLibraryScanner();

  @override
  bool cancel(String scanId) => false;

  @override
  Future<RecoverableLibraryScan?> loadPausedScan() async => null;

  @override
  Future<RecoverableLibraryScan?> loadRecoverableScan() async => null;

  @override
  bool pause(String scanId) => false;

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

class _RecordingViewPreferenceStore implements LibraryViewPreferenceStore {
  _RecordingViewPreferenceStore(this.initialPreferences);

  final LibraryViewPreferences initialPreferences;
  final List<LibraryViewPreferences> saved = [];

  @override
  Future<LibraryViewPreferences> loadLibraryViewPreferences() async {
    return initialPreferences;
  }

  @override
  Future<void> saveLibraryViewPreferences(
    LibraryViewPreferences preferences,
  ) async {
    saved.add(preferences);
  }
}

class _RecordingAmePreferenceStore implements AmePreferenceStore {
  _RecordingAmePreferenceStore(this.initialPreferences);

  final AmePreferences initialPreferences;
  final List<AmePreferences> saved = [];

  @override
  Future<AmePreferences> loadAmePreferences() async => initialPreferences;

  @override
  Future<void> saveAmePreferences(AmePreferences preferences) async {
    saved.add(preferences);
  }
}
