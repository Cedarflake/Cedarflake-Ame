import "package:cedarflake_ame/features/library/domain/library_models.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_gallery_filter.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_metadata_filter_summary.dart";
import "package:flutter/material.dart";
import "package:flutter_test/flutter_test.dart";

void main() {
  testWidgets("folder filters apply directly and preserve browsing scope", (
    tester,
  ) async {
    var query = const LibraryGalleryQuery(
      rootId: "root",
      folderRelativePath: "photos",
      searchText: "photo",
      sortKey: LibraryGallerySortKey.fileName,
    );
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: StatefulBuilder(
            builder: (context, setState) => LibraryGalleryFilter(
              query: query,
              onChanged: (value) => setState(() => query = value),
            ),
          ),
        ),
      ),
    );
    await _open(tester);
    expect(
      find.byWidgetPredicate((widget) => widget is PopupMenuItem),
      findsNWidgets(5),
    );
    expect(find.text("更多筛选"), findsOneWidget);
    await tester.tap(find.text("隐藏子目录"));
    await tester.pumpAndSettle();
    expect(query.includeDescendants, isFalse);
    expect(find.byType(Dialog), findsNothing);
    await _open(tester);
    await tester.tap(find.text("全部图片"));
    await tester.pumpAndSettle();
    expect(query.duplicateMode, LibraryGalleryDuplicateMode.all);
    expect(query.rootId, "root");
    expect(query.folderRelativePath, "photos");
    expect(query.searchText, "photo");
    expect(query.sortKey, LibraryGallerySortKey.fileName);
  });

  testWidgets("albums keep merged groups while Favorites allow all images", (
    tester,
  ) async {
    var changed = false;
    for (final collection in [
      const LibraryGalleryCollection.album("album"),
      const LibraryGalleryCollection.favorites(),
    ]) {
      final query = LibraryGalleryQuery(collection: collection);
      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(
            body: LibraryGalleryFilter(
              query: query,
              onChanged: (_) => changed = true,
            ),
          ),
        ),
      );
      await _open(tester);
      final isAlbum = collection is LibraryAlbumCollection;
      for (final label in ["显示子目录", "隐藏子目录", "全部图片", "合并重复图片"]) {
        final item = find.ancestor(
          of: find.text(label),
          matching: find.byWidgetPredicate((widget) => widget is PopupMenuItem),
        );
        final isSource = label.endsWith("子目录");
        expect(
          tester.widget<PopupMenuItem<Object?>>(item).enabled,
          !isSource && !isAlbum,
        );
      }
      expect(
        tester
            .widget<Semantics>(find.byKey(const ValueKey("menu-choice-合并重复图片")))
            .properties
            .checked,
        isAlbum,
      );
      await tester.tap(find.text("全部图片"));
      await tester.pumpAndSettle();
      expect(changed, !isAlbum);
      await tester.tapAt(Offset.zero);
      await tester.pumpAndSettle();
    }
  });

  testWidgets(
    "metadata edits apply once, cancel safely and remove only chosen conditions",
    (tester) async {
      await tester.binding.setSurfaceSize(const Size(1100, 900));
      addTearDown(() => tester.binding.setSurfaceSize(null));
      var query = const LibraryGalleryQuery(
        rootId: "root",
        folderRelativePath: "photos",
        searchText: "sunset",
        duplicateMode: LibraryGalleryDuplicateMode.all,
        sortKey: LibraryGallerySortKey.fileName,
      );
      var changes = 0;
      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(
            body: StatefulBuilder(
              builder: (context, setState) {
                void change(LibraryGalleryQuery value) => setState(() {
                  query = value;
                  changes++;
                });
                return Column(
                  children: [
                    LibraryGalleryFilter(query: query, onChanged: change),
                    LibraryMetadataFilterSummary(
                      filter: query.metadataFilter,
                      onChanged: (value) =>
                          change(query.copyWith(metadataFilter: value)),
                    ),
                  ],
                );
              },
            ),
          ),
        ),
      );
      await _edit(tester);
      await _enter(tester, "dateStart", "2025-02-29");
      await _enter(tester, "extension", ".PNG");
      await _enter(tester, "minimumBytes", "1.5");
      await _enter(tester, "minimumWidth", "2000");
      await _enter(tester, "maximumWidth", "1000");
      expect(changes, 0);
      await tester.tap(find.text("应用"));
      await tester.pumpAndSettle();
      expect(find.text("最大值不能小于最小值"), findsOneWidget);
      expect(find.text("请输入有效日期，如 2026-01-31").hitTestable(), findsOneWidget);
      expect(changes, 0);
      await _enter(tester, "dateStart", "2024-02-29");
      await _enter(tester, "maximumWidth", "3000");
      await tester.tap(find.text("应用"));
      await tester.pumpAndSettle();
      expect(changes, 1);
      expect(
        query.metadataFilter,
        const LibraryMetadataFilter(
          extension: "png",
          bytes: LibraryNumberRange(minimum: 1572864),
          width: LibraryNumberRange(minimum: 2000, maximum: 3000),
          date: LibraryDateRange(start: "2024-02-29"),
        ),
      );
      expect(query.rootId, "root");
      expect(query.folderRelativePath, "photos");
      expect(query.searchText, "sunset");
      expect(query.duplicateMode, LibraryGalleryDuplicateMode.all);
      expect(query.sortKey, LibraryGallerySortKey.fileName);

      await _edit(tester);
      await _enter(tester, "extension", "jpg");
      await tester.tap(find.text("取消"));
      await tester.pumpAndSettle();
      expect(changes, 1);
      expect(query.metadataFilter.extension, "png");
      await tester.tap(find.byTooltip("移除扩展名：png"));
      await tester.pumpAndSettle();
      expect(query.metadataFilter.extension, isEmpty);
      expect(query.metadataFilter.width.minimum, 2000);
      expect(query.metadataFilter.bytes.minimum, 1572864);
      expect(changes, 2);

      await _edit(tester);
      await tester.tap(
        find.descendant(
          of: find.byType(AlertDialog),
          matching: find.text("清除筛选"),
        ),
      );
      await tester.pumpAndSettle();
      expect(query.metadataFilter.isActive, isFalse);
      expect(query.folderRelativePath, "photos");
      expect(query.searchText, "sunset");
      expect(query.duplicateMode, LibraryGalleryDuplicateMode.all);
      expect(find.byKey(const Key("library-filter-summary")), findsNothing);
      expect(changes, 3);
    },
  );

  testWidgets("an editor from a retired query cannot change the new scope", (
    tester,
  ) async {
    var query = const LibraryGalleryQuery(rootId: "original");
    var changes = 0;
    late StateSetter update;
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: StatefulBuilder(
            builder: (context, setState) {
              update = setState;
              return LibraryGalleryFilter(
                query: query,
                onChanged: (_) => changes++,
              );
            },
          ),
        ),
      ),
    );
    await _edit(tester);
    await _enter(tester, "extension", "png");
    update(() => query = const LibraryGalleryQuery(rootId: "replacement"));
    await tester.pump();
    await tester.tap(find.text("应用"));
    await tester.pumpAndSettle();
    expect(changes, 0);
    expect(query.rootId, "replacement");
    expect(find.byType(AlertDialog), findsNothing);
  });
}

Future<void> _open(WidgetTester tester) async {
  await tester.tap(find.byKey(const Key("library-filter-menu")));
  await tester.pumpAndSettle();
}

Future<void> _edit(WidgetTester tester) async {
  await _open(tester);
  await tester.tap(find.text("更多筛选"));
  await tester.pumpAndSettle();
}

Future<void> _enter(WidgetTester tester, String field, String text) async {
  final input = find.byKey(ValueKey("library-filter-$field"));
  await tester.ensureVisible(input);
  await tester.enterText(input, text);
  await tester.pump();
}
