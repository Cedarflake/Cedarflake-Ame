import "dart:async";
import "dart:io";
import "dart:typed_data";

import "package:cedarflake_ame/app/ame_app.dart";
import "package:cedarflake_ame/features/library/application/library_catalog.dart";
import "package:cedarflake_ame/features/library/application/library_controller.dart";
import "package:cedarflake_ame/features/library/application/library_layout_manifest_catalog.dart";
import "package:cedarflake_ame/features/library/application/library_previewer.dart";
import "package:cedarflake_ame/features/library/application/library_scanner.dart";
import "package:cedarflake_ame/features/library/application/library_view_preferences.dart";
import "package:cedarflake_ame/features/library/domain/gallery_layout_manifest.dart";
import "package:cedarflake_ame/features/library/domain/library_models.dart";
import "package:cedarflake_ame/features/library/domain/library_state.dart";
import "package:cedarflake_ame/features/library/presentation/gallery_selection.dart";
import "package:cedarflake_ame/features/library/presentation/library_strings.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_gallery_layout_snapshot.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_gallery_wall.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_photo_tile.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_time_navigation.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_virtual_gallery_geometry.dart";
import "package:cedarflake_ame/features/settings/application/ame_preferences.dart";
import "package:cedarflake_ame/features/settings/presentation/widgets/settings_section.dart";
import "package:flutter/gestures.dart";
import "package:flutter/material.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:flutter/services.dart";
import "package:flutter_test/flutter_test.dart";
import "package:integration_test/integration_test.dart";

void main() {
  IntegrationTestWidgetsFlutterBinding.ensureInitialized();

  testWidgets("keeps the native Windows accessibility tree synchronized", (
    tester,
  ) async {
    const itemCount = 1200;
    const wallSize = Size(1000, 700);
    final state = _galleryState(itemCount);
    final manifest = _manifest(itemCount);
    final snapshot = LibrarySnapshot(
      catalogPath: "C:\\AmeData\\ame.sqlite3",
      revision: BigInt.one,
      queryId: state.queryId,
      roots: state.roots,
      assets: state.assets,
    );
    final layoutSnapshot = LibraryGalleryLayoutSnapshot.build(
      manifest: manifest,
      availableWidth: wallSize.width - 80 - 40,
      thumbnailSize: GalleryThumbnailSize.medium,
      layoutShape: GalleryLayoutShape.square,
      sortKey: LibraryGallerySortKey.captureTime,
    );
    final scrollController = ScrollController();
    addTearDown(scrollController.dispose);
    tester.view.physicalSize = const Size(1280, 800);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);
    final semanticsHandle = tester.ensureSemantics();
    try {
      await tester.pumpWidget(
        ProviderScope(
          overrides: [
            initialLibraryStateProvider.overrideWithValue(state),
            libraryCatalogProvider.overrideWithValue(
              _StaticCatalog(snapshot, state.timeline!),
            ),
            libraryScannerProvider.overrideWithValue(const _NoopScanner()),
            libraryPreviewerProvider.overrideWithValue(
              _StaticPreviewer(state.assets),
            ),
          ],
          child: MaterialApp(
            home: Scaffold(
              body: Align(
                alignment: Alignment.topLeft,
                child: SizedBox.fromSize(
                  size: wallSize,
                  child: Consumer(
                    builder: (context, ref, child) {
                      final controller = ref.read(
                        libraryControllerProvider.notifier,
                      );
                      return Row(
                        children: [
                          Expanded(
                            child: LibraryGalleryWall(
                              state: state,
                              controller: controller,
                              scrollController: scrollController,
                              layoutShape: GalleryLayoutShape.square,
                              thumbnailSize: GalleryThumbnailSize.medium,
                              selection: GallerySelection.empty(state.queryId),
                              isSelecting: false,
                              layoutManifest: manifest,
                              onOpen: (_) {},
                              onToggleSelection: (_) {},
                              onViewInformation: (_) {},
                              onCopyPath: (_) {},
                              onRevealFile: (_) {},
                              onVisiblePositionChanged: (_) {},
                              onLoadPrevious: () async {},
                              onLayoutChanged: (_, _) {},
                            ),
                          ),
                          LibraryTimeNavigation(
                            isLoading: false,
                            scrollController: scrollController,
                            layoutMetrics: layoutSnapshot.metrics,
                            timeline: state.timeline,
                            layoutShape: GalleryLayoutShape.square,
                            virtualGeometry: LibraryVirtualGalleryGeometry(
                              totalContentExtent:
                                  layoutSnapshot.metrics.contentExtent,
                              viewportExtent: wallSize.height,
                              leadingExtent: 0,
                              loadedContentExtent:
                                  layoutSnapshot.metrics.contentExtent,
                              trailingExtent: 0,
                              windowStartItemOffset: 0,
                              loadedItemCount: itemCount,
                              totalItemCount: itemCount,
                              queryId: state.queryId,
                            ),
                            windowStartItemOffset: 0,
                            loadedItemCount: itemCount,
                            onSeek: (_, _) async => true,
                          ),
                        ],
                      );
                    },
                  ),
                ),
              ),
            ),
          ),
        ),
      );
      await tester.pumpAndSettle();
      expect(find.byType(MenuAnchor), findsNothing);

      for (final fraction in <double>[0.15, 0.45, 0.75, 1, 0.6, 0.3, 0.9, 0]) {
        scrollController.jumpTo(
          scrollController.position.maxScrollExtent * fraction,
        );
        await tester.pump();
        await tester.pump();

        final visibleTiles = find.byType(LibraryPhotoTile).hitTestable();
        expect(visibleTiles, findsWidgets);
        await tester.tap(visibleTiles.first, buttons: kSecondaryMouseButton);
        await tester.pumpAndSettle();
        expect(find.byType(MenuAnchor), findsNothing);

        await tester.sendKeyEvent(LogicalKeyboardKey.escape);
        await tester.pumpAndSettle();
      }

      await tester.pumpWidget(const SizedBox.shrink());
      await tester.pumpAndSettle();
    } finally {
      semanticsHandle.dispose();
    }
  });

  testWidgets(
    "keeps the populated application accessibility tree synchronized",
    (tester) async {
      const itemCount = 1200;
      const initialLoadedCount = 160;
      final nativeUiaProbe = _WindowsUiaProbe.fromEnvironment();
      final allAssets = [
        for (var index = 0; index < itemCount; index++) _asset(index),
      ];
      final state = _galleryState(
        itemCount,
        assets: allAssets.take(initialLoadedCount).toList(growable: false),
      );
      final manifest = _manifest(itemCount);
      final snapshot = LibrarySnapshot(
        catalogPath: "C:\\AmeData\\ame.sqlite3",
        revision: BigInt.one,
        queryId: state.queryId,
        roots: state.roots,
        assets: state.assets,
      );
      final catalog = _ControlledWindowCatalog(
        initialSnapshot: snapshot,
        timeline: state.timeline!,
        allAssets: allAssets,
      );
      tester.view.physicalSize = const Size(1280, 800);
      tester.view.devicePixelRatio = 1;
      addTearDown(tester.view.resetPhysicalSize);
      addTearDown(tester.view.resetDevicePixelRatio);
      final semanticsHandle = tester.ensureSemantics();
      try {
        await tester.pumpWidget(
          ProviderScope(
            overrides: [
              initialLibraryStateProvider.overrideWithValue(state),
              initialLibraryViewPreferencesProvider.overrideWithValue(
                const LibraryViewPreferences(
                  layoutShape: GalleryLayoutShape.square,
                ),
              ),
              libraryCatalogProvider.overrideWithValue(catalog),
              libraryScannerProvider.overrideWithValue(const _NoopScanner()),
              libraryPreviewerProvider.overrideWithValue(
                _StaticPreviewer(allAssets),
              ),
              libraryGalleryLayoutManifestLoaderProvider.overrideWithValue(
                _StaticManifestLoader(manifest),
              ),
            ],
            child: const AmeApp(),
          ),
        );
        await tester.pumpAndSettle();
        expect(find.byType(MenuAnchor), findsNothing);
        await nativeUiaProbe.checkpoint(tester, "application-ready");

        final mouse = await tester.createGesture(kind: PointerDeviceKind.mouse);
        await mouse.addPointer();
        for (final key in const <Key>[
          Key("library-sort-menu"),
          Key("library-layout-menu"),
          Key("library-more-menu"),
        ]) {
          final control = find.byKey(key).hitTestable();
          expect(control, findsOneWidget);
          final controlCenter = tester.getCenter(control);
          await mouse.moveTo(controlCenter);
          await tester.pump(const Duration(milliseconds: 600));
          await tester.tap(control);
          await tester.pumpAndSettle();
          await tester.tapAt(controlCenter);
          await tester.pumpAndSettle();
        }

        for (final rootId in const ["root-1", "root-2", "root-3"]) {
          for (final controlName in const ["title", "expand", "more"]) {
            final control = find.byKey(ValueKey("source-$controlName-$rootId"));
            expect(control, findsOneWidget);
            if (controlName != "title") {
              expect(control.hitTestable(), findsOneWidget);
            }
            await mouse.moveTo(tester.getCenter(control));
            await tester.pump(const Duration(milliseconds: 600));
          }
        }

        final firstSourceMenu = find.byKey(
          const ValueKey("source-more-root-1"),
        );
        final secondSourceMenu = find.byKey(
          const ValueKey("source-more-root-2"),
        );
        expect(firstSourceMenu.hitTestable(), findsOneWidget);
        expect(secondSourceMenu.hitTestable(), findsOneWidget);
        await tester.tap(firstSourceMenu);
        await tester.pumpAndSettle();
        expect(find.text(LibraryStrings.updateLibrary), findsOneWidget);
        await nativeUiaProbe.checkpoint(tester, "source-menu-open");
        await mouse.moveTo(tester.getCenter(secondSourceMenu));
        await tester.pump(const Duration(milliseconds: 600));
        await tester.tapAt(const Offset(1272, 792));
        await _pumpUntil(
          tester,
          () => find.text(LibraryStrings.updateLibrary).evaluate().isEmpty,
        );
        expect(find.text(LibraryStrings.updateLibrary), findsNothing);
        await nativeUiaProbe.checkpoint(tester, "source-menu-closed");
        expect(secondSourceMenu.hitTestable(), findsOneWidget);
        await tester.tap(secondSourceMenu);
        await tester.pumpAndSettle();
        expect(find.text(LibraryStrings.updateLibrary), findsOneWidget);
        await tester.tapAt(const Offset(1272, 792));
        await _pumpUntil(
          tester,
          () => find.text(LibraryStrings.updateLibrary).evaluate().isEmpty,
        );
        expect(find.text(LibraryStrings.updateLibrary), findsNothing);

        final wall = tester.widget<LibraryGalleryWall>(
          find.byType(LibraryGalleryWall),
        );
        wall.scrollController.jumpTo(
          wall.scrollController.position.maxScrollExtent * 0.8,
        );
        await tester.pump();
        await tester.pump();
        expect(catalog.hasPendingTimeLoad, isTrue);
        expect(find.byKey(const Key("library-top-loading")), findsOneWidget);
        await nativeUiaProbe.checkpoint(tester, "square-range-loading");

        final unloadedSlots = find.descendant(
          of: find.byKey(const Key("library-photo-wall")),
          matching: find.byWidgetPredicate((widget) {
            final key = widget.key;
            return widget is! LibraryPhotoTile &&
                key is ValueKey<String> &&
                key.value.startsWith("location-");
          }),
        );
        expect(unloadedSlots, findsWidgets);
        expect(unloadedSlots.hitTestable(), findsNothing);
        expect(
          find.descendant(
            of: unloadedSlots,
            matching: find.byType(DecoratedBox),
          ),
          findsNothing,
        );
        final wallRect = tester.getRect(
          find.byKey(const Key("library-photo-wall")),
        );
        final unobscuredWallRect = Rect.fromLTRB(
          wallRect.left + 4,
          wallRect.top + 8,
          wallRect.right - 96,
          wallRect.bottom - 4,
        );
        Key? unloadedSlotKey;
        Rect? unloadedSlotRect;
        var nearestDistance = double.infinity;
        for (final element in unloadedSlots.evaluate()) {
          final key = element.widget.key;
          if (key == null) {
            continue;
          }
          final candidateRect = tester.getRect(find.byKey(key));
          if (!unobscuredWallRect.contains(candidateRect.center)) {
            continue;
          }
          final distance = (candidateRect.center - unobscuredWallRect.center)
              .distanceSquared;
          if (distance < nearestDistance) {
            nearestDistance = distance;
            unloadedSlotKey = key;
            unloadedSlotRect = candidateRect;
          }
        }
        final replacementKey = unloadedSlotKey;
        final originalRect = unloadedSlotRect;
        if (replacementKey == null || originalRect == null) {
          throw StateError("No unloaded slot is visible in the gallery");
        }

        catalog.completePendingTimeLoad();
        await tester.pumpAndSettle();
        final replacement = find.byKey(replacementKey).hitTestable();
        expect(replacement, findsOneWidget);
        expect(tester.widget(replacement), isA<LibraryPhotoTile>());
        final replacementRect = tester.getRect(replacement);
        expect(replacementRect.left, closeTo(originalRect.left, 1));
        expect(replacementRect.top, closeTo(originalRect.top, 1));
        expect(replacementRect.width, closeTo(originalRect.width, 1));
        expect(replacementRect.height, closeTo(originalRect.height, 1));

        for (final fraction in <double>[
          0.15,
          0.45,
          0.75,
          1,
          0.6,
          0.3,
          0.9,
          0,
        ]) {
          wall.scrollController.jumpTo(
            wall.scrollController.position.maxScrollExtent * fraction,
          );
          await tester.pump();
          await tester.pump();
          await tester.pump();

          await tester.tap(
            find.byType(LibraryPhotoTile).hitTestable().first,
            buttons: kSecondaryMouseButton,
          );
          await tester.pumpAndSettle();
          await tester.sendKeyEvent(LogicalKeyboardKey.escape);
          await tester.pumpAndSettle();
        }

        final timelineSliderFinder = find
            .byKey(const Key("timeline-slider"))
            .hitTestable();
        final timelineValueBefore = tester
            .widget<Slider>(timelineSliderFinder)
            .value;
        final timelineRect = tester.getRect(timelineSliderFinder);
        await mouse.moveTo(timelineRect.center);
        await mouse.down(timelineRect.center);
        await mouse.moveTo(
          timelineRect.center + Offset(0, timelineRect.height * 0.25),
        );
        await mouse.up();
        await tester.pumpAndSettle();
        expect(
          tester.widget<Slider>(timelineSliderFinder).value,
          isNot(closeTo(timelineValueBefore, 0.001)),
        );

        for (var cycle = 0; cycle < 2; cycle += 1) {
          final tile = find.byType(LibraryPhotoTile).hitTestable().first;
          final tileRect = tester.getRect(tile);
          await tester.tapAt(tileRect.topLeft + const Offset(16, 16));
          await tester.pump();
          await tester.pump();
          expect(find.byKey(const Key("viewer-back-button")), findsOneWidget);
          if (cycle == 0) {
            await nativeUiaProbe.checkpoint(tester, "viewer-open");
          }

          final viewerSliderFinder = find
              .descendant(
                of: find.byKey(const Key("viewer-zoom-controls")),
                matching: find.byType(Slider),
              )
              .hitTestable();
          final viewerValueBefore = tester
              .widget<Slider>(viewerSliderFinder)
              .value;
          final viewerSliderRect = tester.getRect(viewerSliderFinder);
          await mouse.moveTo(viewerSliderRect.center);
          await mouse.down(viewerSliderRect.center);
          await mouse.moveTo(
            viewerSliderRect.center + Offset(viewerSliderRect.width * 0.25, 0),
          );
          await mouse.up();
          await tester.pump();
          await tester.pump(const Duration(milliseconds: 200));
          expect(
            tester.widget<Slider>(viewerSliderFinder).value,
            isNot(closeTo(viewerValueBefore, 0.001)),
          );

          final viewerMore = find.byKey(const Key("viewer-more-menu"));
          final viewerMoreCenter = tester.getCenter(viewerMore);
          await mouse.moveTo(viewerMoreCenter);
          await tester.pump(const Duration(milliseconds: 600));
          await tester.tap(viewerMore);
          await tester.pump();
          await tester.pump(const Duration(milliseconds: 250));
          expect(find.text(LibraryStrings.copyPath), findsOneWidget);
          if (cycle == 0) {
            await nativeUiaProbe.checkpoint(tester, "viewer-menu-open");
          }
          final viewerBack = find.byKey(const Key("viewer-back-button"));
          await mouse.moveTo(tester.getCenter(viewerBack));
          await tester.pump(const Duration(milliseconds: 600));
          await tester.tapAt(viewerMoreCenter);
          await tester.pump();
          await tester.pump(const Duration(milliseconds: 250));
          expect(find.text(LibraryStrings.copyPath), findsNothing);
          expect(viewerBack, findsOneWidget);
          expect(viewerBack.hitTestable(), findsOneWidget);
          if (cycle == 0) {
            await nativeUiaProbe.checkpoint(tester, "viewer-menu-closed");
          }

          await tester.tap(viewerBack);
          await tester.pump();
          await tester.pump();
          expect(viewerBack, findsNothing);
          expect(find.byType(LibraryPhotoTile), findsWidgets);
        }
        await mouse.removePointer();

        await tester.tap(
          find.byKey(const Key("library-sidebar-settings")).hitTestable(),
        );
        final settingsPage = find.byKey(const Key("ame-settings-page"));
        await _pumpUntil(tester, () => settingsPage.evaluate().isNotEmpty);
        expect(settingsPage, findsOneWidget);
        final themeChoice = find.byType(SettingsChoice<AmeThemePreference>);
        expect(themeChoice, findsOneWidget);
        await tester.tap(
          find
              .descendant(
                of: themeChoice,
                matching: find.byType(OutlinedButton),
              )
              .hitTestable(),
        );
        final lightThemeLabel = find.text("浅色");
        final lightThemeItem = find.ancestor(
          of: lightThemeLabel.last,
          matching: find.byWidgetPredicate((widget) => widget is PopupMenuItem),
        );
        final lightThemeChoice = find
            .descendant(of: lightThemeItem, matching: find.byType(InkWell))
            .hitTestable();
        await _pumpUntil(tester, () => lightThemeChoice.evaluate().isNotEmpty);
        expect(lightThemeChoice, findsOneWidget);
        expect(find.byType(MenuAnchor), findsNothing);
        await nativeUiaProbe.checkpoint(tester, "settings-menu-open");
        await tester.sendKeyEvent(LogicalKeyboardKey.escape);
        await _pumpUntil(tester, () => lightThemeLabel.evaluate().isEmpty);
        expect(lightThemeLabel, findsNothing);
        await nativeUiaProbe.checkpoint(tester, "settings-menu-closed");

        await tester.pumpWidget(const SizedBox.shrink());
        await tester.pumpAndSettle();
      } finally {
        semanticsHandle.dispose();
      }
    },
  );
}

class _WindowsUiaProbe {
  _WindowsUiaProbe._({required this.directory, required this.token});

  static const _directoryEnvironment =
      "CEDARFLAKE_AME_WINDOWS_UIA_PROBE_DIRECTORY";
  static const _tokenEnvironment = "CEDARFLAKE_AME_WINDOWS_UIA_PROBE_TOKEN";
  static const _protocolPrefix = "AME_WINDOWS_UIA_PROBE_V1";
  static const _checkpointTimeout = Duration(seconds: 20);
  static const _pollInterval = Duration(milliseconds: 50);

  factory _WindowsUiaProbe.fromEnvironment() {
    final directory = Platform.environment[_directoryEnvironment];
    final token = Platform.environment[_tokenEnvironment];
    if (directory == null && token == null) {
      return _WindowsUiaProbe._(directory: null, token: null);
    }
    if (directory == null ||
        directory.isEmpty ||
        token == null ||
        token.isEmpty) {
      throw StateError("Windows UIA probe environment is incomplete");
    }
    return _WindowsUiaProbe._(directory: Directory(directory), token: token);
  }

  final Directory? directory;
  final String? token;
  var _sequence = 0;

  Future<void> checkpoint(WidgetTester tester, String phase) async {
    final probeDirectory = directory;
    final probeToken = token;
    if (probeDirectory == null || probeToken == null) {
      return;
    }
    _sequence += 1;
    final sequence = _sequence.toString().padLeft(2, "0");
    final request = File(
      "${probeDirectory.path}${Platform.pathSeparator}request-$sequence.txt",
    );
    final requestDraft = File(
      "${probeDirectory.path}${Platform.pathSeparator}request-$sequence.tmp",
    );
    final acknowledgement = File(
      "${probeDirectory.path}${Platform.pathSeparator}ack-$sequence.txt",
    );
    await requestDraft.writeAsString(
      "$_protocolPrefix|$probeToken|$sequence|$phase\n",
      flush: true,
    );
    await requestDraft.rename(request.path);

    final deadline = DateTime.now().add(_checkpointTimeout);
    while (DateTime.now().isBefore(deadline)) {
      await tester.pump(_pollInterval);
      if (!await acknowledgement.exists()) {
        continue;
      }
      final response = (await acknowledgement.readAsString()).trim();
      final expected = "$_protocolPrefix|$probeToken|$sequence|ok";
      if (response != expected) {
        throw StateError(
          "Windows UIA probe returned an invalid acknowledgement for $phase",
        );
      }
      return;
    }
    throw StateError("Windows UIA probe timed out at $phase");
  }
}

Future<void> _pumpUntil(WidgetTester tester, bool Function() condition) async {
  const interval = Duration(milliseconds: 50);
  const attempts = 40;
  for (var attempt = 0; attempt < attempts; attempt += 1) {
    await tester.pump(interval);
    if (condition()) {
      return;
    }
  }
}

LibraryState _galleryState(int itemCount, {List<LibraryAsset>? assets}) {
  final revision = BigInt.one;
  return LibraryState(
    status: LibraryStatus.completed,
    roots: [
      LibraryRoot(
        id: "root-1",
        path: "C:\\Pictures",
        displayPath: "C:\\Pictures",
        createdUnixMs: 1,
        assetCount: itemCount,
        issueCount: 0,
        availability: LibraryRootAvailability.available,
      ),
      const LibraryRoot(
        id: "root-2",
        path: "C:\\Documents",
        displayPath: "C:\\Documents",
        createdUnixMs: 2,
        assetCount: 0,
        issueCount: 0,
        availability: LibraryRootAvailability.available,
      ),
      const LibraryRoot(
        id: "root-3",
        path: "C:\\Archive",
        displayPath: "C:\\Archive",
        createdUnixMs: 3,
        assetCount: 0,
        issueCount: 0,
        availability: LibraryRootAvailability.available,
      ),
    ],
    catalogRevision: revision,
    queryId: "windows-accessibility-query",
    assets:
        assets ??
        [for (var index = 0; index < itemCount; index++) _asset(index)],
    timeline: LibraryTimeline(
      revision: revision,
      queryId: "windows-accessibility-query",
      totalItems: itemCount,
      buckets: [
        LibraryTimeBucket(
          monthKey: "2026-08",
          itemCount: itemCount,
          aspectRatioSum: itemCount.toDouble(),
        ),
      ],
    ),
  );
}

LibraryAsset _asset(int index) {
  return LibraryAsset(
    assetId: "asset-$index",
    locationId: "location-$index",
    rootId: "root-1",
    activeScanId: "scan-1",
    sourcePath: "C:\\Pictures\\$index.jpg",
    displayPath: "C:\\Pictures\\$index.jpg",
    relativePath: "$index.jpg",
    previewPath: "",
    fileSize: BigInt.one,
    modifiedUnixMs: 1,
    sourceRevision: null,
    sourceGeneration: BigInt.one,
    width: 4,
    height: 3,
    previewStatus: LibraryPreviewStatus.pending,
  );
}

LibraryGalleryLayoutManifest _manifest(int itemCount) {
  final revision = BigInt.one;
  final builder = LibraryGalleryLayoutManifestBuilder(
    revision: revision,
    queryId: "windows-accessibility-query",
    totalItems: itemCount,
  );
  builder.append(
    LibraryGalleryLayoutManifestChunk(
      revision: revision,
      queryId: "windows-accessibility-query",
      totalItems: itemCount,
      startOrdinal: 0,
      locationIds: [
        for (var index = 0; index < itemCount; index++) "location-$index",
      ],
      aspectRatioMilli: Uint16List.fromList(List.filled(itemCount, 1333)),
      dateGroupIndices: Uint16List(itemCount),
      dateGroups: const ["2026-08-10"],
      flags: Uint8List.fromList(
        List.filled(itemCount, libraryGalleryLayoutDimensionsKnownFlag),
      ),
    ),
  );
  return builder.build();
}

class _StaticCatalog implements LibraryCatalog {
  const _StaticCatalog(this.snapshot, this.timeline);

  final LibrarySnapshot snapshot;
  final LibraryTimeline timeline;

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
  Future<LibraryTimeline> loadTimeline(LibraryGalleryQuery query) async =>
      timeline;

  @override
  Future<bool> unregisterRoot(String rootId) async => false;
}

class _ControlledWindowCatalog implements LibraryCatalog {
  _ControlledWindowCatalog({
    required this.initialSnapshot,
    required this.timeline,
    required this.allAssets,
  });

  final LibrarySnapshot initialSnapshot;
  final LibraryTimeline timeline;
  final List<LibraryAsset> allAssets;
  Completer<LibrarySnapshot>? _pendingTimeLoad;
  LibrarySnapshot? _pendingTimeSnapshot;
  bool _shouldDelayNextTimeLoad = true;

  bool get hasPendingTimeLoad => _pendingTimeLoad != null;

  void completePendingTimeLoad() {
    final pending = _pendingTimeLoad;
    final snapshot = _pendingTimeSnapshot;
    if (pending == null || snapshot == null) {
      throw StateError("No time-window load is pending");
    }
    _pendingTimeLoad = null;
    _pendingTimeSnapshot = null;
    _shouldDelayNextTimeLoad = false;
    pending.complete(snapshot);
  }

  @override
  Future<LibrarySnapshot> load({
    required int maxItems,
    required LibraryGalleryQuery query,
    LibraryCatalogCursor? after,
    LibraryCatalogCursor? before,
  }) async => initialSnapshot;

  @override
  Future<LibrarySnapshot> loadAtTime({
    required int maxItems,
    required LibraryGalleryQuery query,
    required LibraryTimeAnchor anchor,
  }) {
    final start = anchor.itemOffset.clamp(0, allAssets.length - 1).toInt();
    final end = (start + maxItems).clamp(start + 1, allAssets.length).toInt();
    final snapshot = LibrarySnapshot(
      catalogPath: initialSnapshot.catalogPath,
      revision: initialSnapshot.revision,
      queryId: initialSnapshot.queryId,
      roots: initialSnapshot.roots,
      assets: allAssets.sublist(start, end),
    );
    if (!_shouldDelayNextTimeLoad) {
      return Future.value(snapshot);
    }
    final existing = _pendingTimeLoad;
    if (existing != null) {
      return existing.future;
    }
    final pending = Completer<LibrarySnapshot>();
    _pendingTimeLoad = pending;
    _pendingTimeSnapshot = snapshot;
    return pending.future;
  }

  @override
  Future<LibraryTimeline> loadTimeline(LibraryGalleryQuery query) async =>
      timeline;

  @override
  Future<bool> unregisterRoot(String rootId) async => false;
}

class _StaticPreviewer implements LibraryPreviewer {
  _StaticPreviewer(Iterable<LibraryAsset> assets)
    : _assets = {for (final asset in assets) asset.locationId: asset};

  final Map<String, LibraryAsset> _assets;

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
  }) async => _assets[locationId]!;
}

class _StaticManifestLoader implements LibraryGalleryLayoutManifestLoader {
  const _StaticManifestLoader(this.manifest);

  final LibraryGalleryLayoutManifest manifest;

  @override
  Future<LibraryGalleryLayoutManifest> load(
    LibraryGalleryQuery query, {
    bool Function()? isCancelled,
  }) async => manifest;
}

class _NoopScanner implements LibraryScanner {
  const _NoopScanner();

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
  }) => const Stream.empty();

  @override
  Stream<LibraryScanUpdate> resume({
    required String scanId,
    required String rootPath,
    required int? itemLimit,
    required int? entryLimit,
    required int previewEdge,
  }) => const Stream.empty();
}
