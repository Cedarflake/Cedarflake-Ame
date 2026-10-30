import "package:cedarflake_ame/features/library/application/library_catalog.dart";
import "package:cedarflake_ame/features/library/application/library_metadata_filter_mapping.dart";
import "package:cedarflake_ame/features/library/domain/library_models.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_gallery_filter.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_gallery_states.dart";
import "package:flutter/material.dart";
import "package:flutter_test/flutter_test.dart";

void main() {
  testWidgets("metadata conditions validate before publishing a scoped query", (
    tester,
  ) async {
    await tester.binding.setSurfaceSize(const Size(1280, 800));
    addTearDown(() => tester.binding.setSurfaceSize(null));
    var query = const LibraryGalleryQuery(
      rootId: "root",
      searchText: "photo",
      collection: LibraryGalleryCollection.album("album"),
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
    await _enter(tester, "extension", ".JPG");
    await _enter(tester, "minimum-bytes", "0.000001");
    await _enter(tester, "maximum-bytes", "2.5");
    await _enter(tester, "camera", " Camera%_One ");
    await _enter(tester, "start", "2026-02-29");
    await tester.tap(find.byKey(const Key("library-filter-apply")));
    await tester.pumpAndSettle();
    expect(find.text("填写有效日期，例如 2026-10-30"), findsOneWidget);
    expect(query.metadataFilter.isActive, isFalse);
    await _enter(tester, "start", "2024-02-29");
    await _enter(tester, "end", "2024-03-01");
    await tester.tap(find.byKey(const Key("library-filter-apply")));
    await tester.pumpAndSettle();
    expect(query.rootId, "root");
    expect(query.collection, const LibraryGalleryCollection.album("album"));
    expect(query.searchText, "photo");
    expect(query.sortKey, LibraryGallerySortKey.fileName);
    expect(query.metadataFilter.extension, "jpg");
    expect(
      query.metadataFilter.bytes,
      const LibraryNumberRange(minimum: 1, maximum: 2500000),
    );
    expect(query.metadataFilter.cameraText, "Camera%_One");
    expect(
      mapRustMetadataFilter(mapLibraryGalleryQueryToRust(query).metadataFilter),
      query.metadataFilter,
    );

    await _open(tester);
    await _enter(tester, "extension", "png");
    await tester.tap(find.text("取消"));
    await tester.pumpAndSettle();
    expect(query.metadataFilter.extension, "jpg");
    await _open(tester);
    await tester.tap(find.byKey(const Key("library-filter-clear")));
    await tester.pumpAndSettle();
    expect(query.metadataFilter.isActive, isFalse);
    expect(query.rootId, "root");
    expect(query.searchText, "photo");
  });

  testWidgets(
    "metadata range errors stay editable and empty results explain the filter",
    (tester) async {
      await tester.binding.setSurfaceSize(const Size(1100, 720));
      addTearDown(() => tester.binding.setSurfaceSize(null));
      const query = LibraryGalleryQuery(
        metadataFilter: LibraryMetadataFilter(extension: "png"),
      );
      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(
            body: LibraryGalleryFilter(query: query, onChanged: (_) {}),
          ),
        ),
      );
      await _open(tester);
      await _enter(tester, "minimum-width", "400");
      await _enter(tester, "maximum-width", "200");
      await tester.tap(find.byKey(const Key("library-filter-apply")));
      await tester.pumpAndSettle();
      expect(find.text("最大值不能小于最小值"), findsOneWidget);
      expect(tester.takeException(), isNull);
      await tester.pumpWidget(
        const MaterialApp(
          home: Scaffold(body: NoGalleryResults(query: query)),
        ),
      );
      expect(find.text("没有符合条件的图片"), findsOneWidget);
    },
  );
}

Future<void> _open(WidgetTester tester) async {
  await tester.tap(find.byKey(const Key("library-filter-menu")));
  await tester.pumpAndSettle();
  await tester.tap(find.text("筛选条件…"));
  await tester.pumpAndSettle();
}

Future<void> _enter(WidgetTester tester, String field, String value) async {
  final target = find.byKey(Key("library-filter-$field"));
  await tester.ensureVisible(target);
  await tester.enterText(target, value);
}
