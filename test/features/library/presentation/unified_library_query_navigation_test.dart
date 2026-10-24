import "dart:ui" show CheckedState;

import "package:cedarflake_ame/app/ame_app.dart";
import "package:cedarflake_ame/app/presentation/ame_menu.dart";
import "package:cedarflake_ame/features/library/application/library_catalog.dart";
import "package:cedarflake_ame/features/library/application/library_controller.dart";
import "package:cedarflake_ame/features/library/application/library_view_preferences.dart";
import "package:cedarflake_ame/features/library/domain/library_models.dart";
import "package:cedarflake_ame/features/library/domain/library_state.dart";
import "package:cedarflake_ame/features/library/presentation/library_strings.dart";
import "package:flutter/material.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:flutter_test/flutter_test.dart";
import "package:material_symbols_icons/symbols.dart";

import "support/library_screen_fixture.dart";
import "support/library_screen_menu_fixture.dart";

void main() {
  testWidgets("favorites clears source scope and library restores all images", (
    tester,
  ) async {
    tester.view.physicalSize = const Size(1280, 800);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);
    final initialState = populatedScreenState(totalItems: 1);
    final catalog = RecordingQueryCatalog(
      LibrarySnapshot(
        catalogPath: initialState.catalogPath ?? "",
        revision: initialState.catalogRevision ?? BigInt.zero,
        queryId: "collection-query",
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
    await tester.tap(find.byKey(const Key("library-sidebar-favorites")));
    await tester.pumpAndSettle();
    expect(
      catalog.loadQueries.last.collection,
      isA<LibraryFavoritesCollection>(),
    );
    expect(catalog.loadQueries.last.rootId, isNull);
    expect(catalog.loadQueries.last.folderRelativePath, isNull);
    await tester.tap(find.byKey(const Key("library-sidebar-library")));
    await tester.pumpAndSettle();
    expect(catalog.loadQueries.last.collection, isA<LibraryAllCollection>());
    expect(catalog.loadQueries.last.rootId, isNull);
    expect(tester.takeException(), isNull);
  });

  testWidgets("connects source search and Material sort to one gallery query", (
    tester,
  ) async {
    tester.view.physicalSize = const Size(1280, 800);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);
    final initialState = populatedScreenState(totalItems: 1);
    final catalog = RecordingQueryCatalog(
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

    final sortMenuButton = find.byKey(const Key("library-sort-menu"));
    await tester.tap(sortMenuButton);
    await tester.pumpAndSettle();
    expectSelectedMenuChoice(
      tester,
      label: LibraryStrings.captureDate,
      icon: Symbols.calendar_month_rounded,
    );
    expectSelectedMenuChoice(
      tester,
      label: LibraryStrings.descending,
      icon: Symbols.arrow_downward_rounded,
    );
    _expectUnselectedMenuChoice(
      tester,
      label: LibraryStrings.fileName,
      icon: Symbols.text_fields_rounded,
    );
    await tester.tapAt(tester.getCenter(sortMenuButton));
    await tester.pumpAndSettle();
    expect(find.text(LibraryStrings.createdDate), findsNothing);

    await tester.tap(sortMenuButton);
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
            populatedScreenState(totalItems: 1),
          ),
        ],
        child: const AmeApp(),
      ),
    );
    await tester.pump();

    final button = find.byKey(const Key("library-sort-menu"));
    expect(button, findsOneWidget);
    expect(find.byType(MenuAnchor), findsNothing);
    final viewRect = Offset.zero & tester.view.physicalSize;
    expect(viewRect.contains(tester.getCenter(button)), isTrue);
    await tester.tap(button);
    await tester.pump();
    final route = ModalRoute.of(
      tester.element(find.text(LibraryStrings.fileName).last),
    );
    expect(route?.transitionDuration, AmeMenuTransition.duration);
    await tester.pumpAndSettle();

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
    final item = activeMenuItem(tester, LibraryStrings.fileName);
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
    await tester.pumpAndSettle();
    expect(find.text(LibraryStrings.fileName), findsNothing);
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
          galleryAsset(
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
      final catalog = AnchorResolvingQueryCatalog(initialSnapshot);

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
      final scrollPosition = galleryScrollPosition(tester);
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
      await selectPopupMenuItem(tester, fileNameItem);

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
      await selectPopupMenuItem(tester, createdDateItem);

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
        galleryAsset(
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
    final catalog = AnchorResolvingQueryCatalog(snapshot)
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
    final scrollPosition = galleryScrollPosition(tester);
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
    await selectPopupMenuItem(tester, fileNameItem);
    expect(catalog.requestedLocationIds, hasLength(1));
    final frozenLocationId = catalog.requestedLocationIds.single;
    final frozenFinder = find.byKey(ValueKey(frozenLocationId));
    expect(visibleBefore, contains(frozenLocationId));

    final captureDateItem = await _openSortMenu(
      tester,
      LibraryStrings.captureDate,
    );
    await selectPopupMenuItem(tester, captureDateItem);
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
}

void _expectUnselectedMenuChoice(
  WidgetTester tester, {
  required String label,
  required IconData icon,
}) {
  final item = menuItem(label);
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

Future<Finder> _openSortMenu(WidgetTester tester, String expectedItem) async {
  await tester.pump(const Duration(milliseconds: 300));
  var item = activeMenuItemOrNull(tester, expectedItem);
  if (item != null) {
    return menuChoiceTarget(expectedItem);
  }
  final button = find.byKey(const Key("library-sort-menu"));
  await tester.ensureVisible(button);
  await tester.tap(button);
  await tester.pump();
  await tester.pump(AmeMenuTransition.duration);
  await tester.pump();
  activeMenuItem(tester, expectedItem);
  return menuChoiceTarget(expectedItem);
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
