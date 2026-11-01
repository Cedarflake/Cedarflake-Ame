import "dart:async";

import "package:cedarflake_ame/app/ame_app.dart";
import "package:cedarflake_ame/features/library/application/library_catalog.dart";
import "package:cedarflake_ame/features/library/application/library_classification_configuration.dart";
import "package:cedarflake_ame/features/library/application/library_classification_settings.dart";
import "package:cedarflake_ame/features/library/application/library_controller.dart";
import "package:cedarflake_ame/features/library/application/library_organization.dart";
import "package:cedarflake_ame/features/library/application/library_scanner.dart";
import "package:cedarflake_ame/features/library/domain/library_models.dart";
import "package:cedarflake_ame/features/library/domain/library_organization.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_gallery_header.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_smart_album_navigation.dart";
import "package:flutter/material.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:flutter_test/flutter_test.dart";

import "../support/library_organization_fixture.dart";
import "support/library_screen_fixture.dart";

void main() {
  testWidgets(
    "smart categories use the shared gallery and retain headings during refresh",
    (tester) async {
      tester.view.physicalSize = const Size(1280, 800);
      tester.view.devicePixelRatio = 1;
      addTearDown(tester.view.resetPhysicalSize);
      addTearDown(tester.view.resetDevicePixelRatio);
      final state = populatedScreenState(totalItems: 1);
      final catalog = RecordingQueryCatalog(
        LibrarySnapshot(
          catalogPath: state.catalogPath!,
          revision: state.catalogRevision!,
          queryId: "smart-query",
          roots: state.roots,
          assets: state.assets,
        ),
      );
      final albums = OrganizationServiceFixture()
        ..albumItems = [const LibraryOrganizationAlbum("manual", "Manual")];
      Future<LibraryClassificationSettings?> settings = Future.value(
        _settings("插画"),
      );
      final container = ProviderContainer(
        overrides: [
          initialLibraryStateProvider.overrideWithValue(state),
          libraryCatalogProvider.overrideWithValue(catalog),
          libraryOrganizationProvider.overrideWithValue(albums),
          libraryScannerProvider.overrideWithValue(const NoopLibraryScanner()),
          libraryClassificationSettingsProvider.overrideWith((ref) => settings),
        ],
      );
      addTearDown(container.dispose);
      await tester.pumpWidget(
        UncontrolledProviderScope(container: container, child: const AmeApp()),
      );
      await tester.pumpAndSettle();
      await tester.tap(find.byKey(const Key("workbench-albums")));
      await tester.pumpAndSettle();
      final smartEntry = find.byKey(const Key("library-sidebar-smart-drawing"));
      final headingText = find.byKey(
        const Key("library-sidebar-smart-heading"),
      );
      expect(tester.widget(headingText), isA<Text>());
      expect(
        tester.getTopLeft(smartEntry).dy,
        greaterThan(
          tester
              .getTopLeft(find.byKey(const Key("library-sidebar-album-manual")))
              .dy,
        ),
      );
      await tester.tap(smartEntry);
      await tester.pumpAndSettle();
      String heading() => tester
          .widget<LibraryGalleryHeader>(find.byType(LibraryGalleryHeader))
          .galleryTitle;
      expect(heading(), "插画");
      expect(
        catalog.loadQueries.last.collection,
        const LibraryGalleryCollection.smartAlbum(categoryId: "drawing"),
      );
      expect(catalog.loadQueries.last.foldsDuplicates, isTrue);

      final queriesBeforeHeadingTap = catalog.loadQueries.length;
      await tester.tap(headingText, warnIfMissed: false);
      await tester.pumpAndSettle();
      expect(catalog.loadQueries.length, queriesBeforeHeadingTap);
      expect(heading(), "插画");

      final queryBeforeCollapse = catalog.loadQueries.last;
      await tester.tap(find.byKey(const Key("library-sidebar-smart-expand")));
      await tester.pumpAndSettle();
      expect(smartEntry, findsNothing);
      expect(heading(), "插画");
      expect(catalog.loadQueries.last, queryBeforeCollapse);
      await tester.tap(find.byKey(const Key("library-sidebar-smart-expand")));
      await tester.pumpAndSettle();
      expect(smartEntry, findsOneWidget);

      await tester.tap(find.byKey(const Key("library-more-menu")));
      await tester.pumpAndSettle();
      expect(find.text("智能相册设置"), findsOneWidget);
      expect(find.text("全选"), findsOneWidget);
      expect(find.text("整理模式"), findsNothing);
      await tester.tapAt(Offset.zero);
      await tester.pumpAndSettle();

      await tester.tap(find.byKey(const Key("library-filter-menu")));
      await tester.pumpAndSettle();
      expect(find.text("合并重复图片"), findsNothing);
      await tester.tap(find.text("待确认"));
      await tester.pumpAndSettle();
      expect(
        catalog.loadQueries.last.collection,
        const LibraryGalleryCollection.smartAlbum(
          categoryId: "drawing",
          review: LibraryClassificationReviewFilter.pending,
        ),
      );

      final delayed = Completer<LibraryClassificationSettings?>();
      settings = delayed.future;
      container.invalidate(libraryClassificationSettingsProvider);
      await tester.pump();
      expect(heading(), "插画");
      await tester.tap(find.byKey(const Key("workbench-gallery")));
      await tester.pumpAndSettle();
      await tester.tap(find.byKey(const Key("workbench-albums")));
      await tester.pumpAndSettle();
      expect(heading(), "插画");
      delayed.complete(_settings("绘画"));
      await tester.pumpAndSettle();
      expect(heading(), "绘画");

      final categoryQuery = catalog.loadQueries.last;
      await tester.tap(find.byKey(const Key("library-more-menu")));
      await tester.pumpAndSettle();
      await tester.tap(find.text("全部分类结果"));
      await tester.pumpAndSettle();
      expect(
        catalog.loadQueries.last,
        categoryQuery.copyWith(
          collection: const LibraryGalleryCollection.smartAlbum(),
        ),
      );
      expect(heading(), "智能相册");
      expect(tester.widget<ListTile>(smartEntry).selected, isFalse);
      await tester.tap(find.byKey(const Key("library-more-menu")));
      await tester.pumpAndSettle();
      expect(find.text("全部分类结果"), findsNothing);
      await tester.tapAt(Offset.zero);
      await tester.pumpAndSettle();

      await tester.tap(smartEntry);
      await tester.pumpAndSettle();
      expect(heading(), "绘画");
      expect(tester.widget<ListTile>(smartEntry).selected, isTrue);
      expect(tester.takeException(), isNull);
    },
  );

  testWidgets("an empty category can reach images without a category", (
    tester,
  ) async {
    tester.view.physicalSize = const Size(1280, 800);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);
    final state = populatedScreenState(totalItems: 1);
    final catalog = _UnclassifiedCatalog(
      LibrarySnapshot(
        catalogPath: state.catalogPath!,
        revision: state.catalogRevision!,
        queryId: "all-results",
        roots: state.roots,
        assets: state.assets,
      ),
    );
    await tester.pumpWidget(
      ProviderScope(
        overrides: [
          initialLibraryStateProvider.overrideWithValue(state),
          libraryCatalogProvider.overrideWithValue(catalog),
          libraryOrganizationProvider.overrideWithValue(
            OrganizationServiceFixture(),
          ),
          libraryScannerProvider.overrideWithValue(const NoopLibraryScanner()),
          libraryClassificationSettingsProvider.overrideWith(
            (ref) async => _settings("插画"),
          ),
        ],
        child: const AmeApp(),
      ),
    );
    await tester.pumpAndSettle();
    await tester.tap(find.byKey(const Key("workbench-albums")));
    await tester.pumpAndSettle();
    await tester.tap(find.byKey(const Key("library-sidebar-smart-drawing")));
    await tester.pumpAndSettle();
    expect(find.byKey(const ValueKey("location-1")), findsNothing);

    await tester.tap(find.byKey(const Key("library-more-menu")));
    await tester.pumpAndSettle();
    await tester.tap(find.text("全部分类结果"));
    await tester.pumpAndSettle();
    expect(find.byKey(const ValueKey("location-1")), findsOneWidget);
    expect(
      catalog.loadQueries.last.collection,
      const LibraryGalleryCollection.smartAlbum(),
    );
    expect(catalog.loadQueries.last.foldsDuplicates, isTrue);
    expect(tester.takeException(), isNull);
  });

  testWidgets(
    "empty smart albums use a noninteractive sidebar row without sample categories",
    (tester) async {
      LibraryClassificationCategory? selected;
      await tester.pumpWidget(
        ProviderScope(
          overrides: [
            libraryClassificationSettingsProvider.overrideWith(
              (ref) async => LibraryClassificationSettings(
                revision: BigInt.zero,
                dispatchRevision: BigInt.zero,
                mode: LibraryClassificationMode.disabled,
                categories: const [],
                runId: null,
                installationSelected: false,
              ),
            ),
          ],
          child: MaterialApp(
            home: Scaffold(
              body: SizedBox(
                width: 260,
                child: LibrarySmartAlbumNavigation(
                  collection: const LibraryGalleryCollection.library(),
                  enabled: true,
                  onSelect: (category) => selected = category,
                ),
              ),
            ),
          ),
        ),
      );
      await tester.pumpAndSettle();
      for (final label in ["插画", "漫画", "表情包", "截图", "实拍", "其他"]) {
        expect(find.text(label), findsNothing);
      }
      expect(selected, isNull);
      expect(find.byType(ListTile), findsOneWidget);
      expect(find.text("还没有分类结果"), findsOneWidget);
      expect(
        tester
            .widget<ListTile>(
              find.byKey(const Key("library-sidebar-smart-empty")),
            )
            .onTap,
        isNull,
      );
      expect(
        find.byKey(const Key("library-sidebar-smart-expand")),
        findsOneWidget,
      );
      await tester.tap(find.byKey(const Key("library-sidebar-smart-expand")));
      await tester.pumpAndSettle();
      expect(
        find.byKey(const Key("library-sidebar-smart-empty")),
        findsNothing,
      );
      await tester.tap(find.byKey(const Key("library-sidebar-smart-expand")));
      await tester.pumpAndSettle();
      expect(
        find.byKey(const Key("library-sidebar-smart-empty")),
        findsOneWidget,
      );
      expect(tester.takeException(), isNull);
    },
  );
}

LibraryClassificationSettings _settings(String label) =>
    LibraryClassificationSettings(
      revision: BigInt.one,
      dispatchRevision: BigInt.one,
      mode: LibraryClassificationMode.paused,
      installationSelected: true,
      runId: "run",
      categories: [
        LibraryClassificationCategory(
          id: "drawing",
          label: label,
          descriptions: const ["drawing"],
        ),
      ],
    );

class _UnclassifiedCatalog extends RecordingQueryCatalog {
  _UnclassifiedCatalog(super.snapshot);

  LibrarySnapshot _results(LibraryGalleryQuery query) {
    final collection = query.collection;
    if (collection is! LibrarySmartAlbumCollection ||
        collection.categoryId == null) {
      return snapshot;
    }
    return LibrarySnapshot(
      catalogPath: snapshot.catalogPath,
      revision: snapshot.revision,
      queryId: "empty-category",
      roots: snapshot.roots,
      assets: const [],
    );
  }

  @override
  Future<LibrarySnapshot> load({
    required int maxItems,
    required LibraryGalleryQuery query,
    LibraryCatalogCursor? after,
    LibraryCatalogCursor? before,
  }) async {
    loadQueries.add(query);
    return _results(query);
  }

  @override
  Future<LibraryTimeline> loadTimeline(LibraryGalleryQuery query) async {
    final result = _results(query);
    return LibraryTimeline(
      revision: result.revision,
      queryId: result.queryId,
      totalItems: result.assets.length,
      buckets: const [],
    );
  }
}
