import "package:cedarflake_ame/features/library/domain/library_models.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_gallery_filter.dart";
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
      findsNWidgets(4),
    );
    expect(find.text("筛选条件…"), findsNothing);
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
}

Future<void> _open(WidgetTester tester) async {
  await tester.tap(find.byKey(const Key("library-filter-menu")));
  await tester.pumpAndSettle();
}
