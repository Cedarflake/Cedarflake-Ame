import "package:cedarflake_ame/app/presentation/ame_theme.dart";
import "package:cedarflake_ame/features/library/domain/library_models.dart";
import "package:cedarflake_ame/features/library/presentation/gallery_view_options.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_gallery_header.dart";
import "package:flutter/material.dart";
import "package:flutter_test/flutter_test.dart";

void main() {
  for (final scale in [1.0, 2.0]) {
    testWidgets(
      "desktop header keeps its columns while resizing at scale $scale",
      (tester) async {
        addTearDown(() => tester.binding.setSurfaceSize(null));
        final review = ValueNotifier(false);
        addTearDown(review.dispose);
        double? initialHeight;
        for (final width in [900.0, 806.0, 780.0, 600.0]) {
          await tester.binding.setSurfaceSize(Size(width, 600));
          await tester.pumpWidget(_app(review, scale: scale));
          await tester.pumpAndSettle();
          final title = tester.getRect(
            find.byKey(const Key("library-gallery-title")),
          );
          final summary = tester.getRect(
            find.byKey(const Key("library-summary")),
          );
          final primary = tester.getRect(
            find.byKey(const Key("library-browsing-toolbar")),
          );
          final height = tester
              .getSize(find.byKey(const Key("library-gallery-header")))
              .height;
          initialHeight ??= height;
          expect(
            primary.center.dy,
            closeTo(title.expandToInclude(summary).center.dy, 0.01),
          );
          expect(height, initialHeight);
          expect(tester.takeException(), isNull);
        }
      },
    );

    testWidgets("header aligns corresponding rows at text scale $scale", (
      tester,
    ) async {
      await tester.binding.setSurfaceSize(const Size(1500, 800));
      addTearDown(() => tester.binding.setSurfaceSize(null));
      final review = ValueNotifier(false);
      addTearDown(review.dispose);
      await tester.pumpWidget(_app(review, scale: scale));
      final title = find.byKey(const Key("library-gallery-title"));
      final summary = find.byKey(const Key("library-summary"));
      final primary = find.byKey(const Key("library-browsing-toolbar"));
      final secondary = find.byKey(const Key("second-row"));
      final gallery = find.byKey(const Key("gallery"));
      final ordinaryTop = tester.getTopLeft(gallery).dy;

      review.value = true;
      await tester.pump();
      expect(tester.getTopLeft(gallery).dy, ordinaryTop);
      await tester.pump(const Duration(milliseconds: 40));
      final earlyTop = tester.getTopLeft(gallery).dy;
      expect(earlyTop, greaterThan(ordinaryTop));
      await tester.pumpWidget(_app(review, scale: scale));
      expect(tester.getTopLeft(gallery).dy, earlyTop);
      await tester.pump(const Duration(milliseconds: 220));
      final completedTop = tester.getTopLeft(gallery).dy;
      await tester.pumpAndSettle();
      expect(tester.getTopLeft(gallery).dy, completedTop);
      expect(
        tester.getCenter(title).dy,
        closeTo(tester.getCenter(primary).dy, 0.01),
      );
      expect(
        tester.getCenter(summary).dy,
        closeTo(tester.getCenter(secondary).dy, 0.01),
      );
      expect(tester.getTopLeft(title).dx, tester.getTopLeft(summary).dx);
      expect(
        tester.getBottomRight(primary).dx,
        tester.getBottomRight(secondary).dx,
      );

      final expandedTop = tester.getTopLeft(gallery).dy;
      review.value = false;
      await tester.pump();
      expect(tester.getTopLeft(gallery).dy, expandedTop);
      await tester.pump(const Duration(milliseconds: 60));
      final reversingTop = tester.getTopLeft(gallery).dy;
      expect(reversingTop, inExclusiveRange(ordinaryTop, expandedTop));
      review.value = true;
      await tester.pump();
      expect(tester.getTopLeft(gallery).dy, reversingTop);
      await tester.pumpAndSettle();
      expect(tester.getTopLeft(gallery).dy, expandedTop);
      review.value = false;
      await tester.pumpAndSettle();
      expect(tester.getTopLeft(gallery).dy, ordinaryTop);
      expect(secondary, findsNothing);
      expect(tester.takeException(), isNull);
    });
  }
}

Widget _app(
  ValueNotifier<bool> reviewing, {
  required double scale,
}) => MaterialApp(
  theme: buildAmeTheme(),
  home: Scaffold(
    body: MediaQuery(
      data: MediaQueryData(textScaler: TextScaler.linear(scale)),
      child: ValueListenableBuilder(
        valueListenable: reviewing,
        builder: (_, active, _) => Column(
          children: [
            LibraryGalleryHeader(
              galleryTitle: "图库",
              totalItems: 72,
              selectedCount: 0,
              isSelecting: false,
              isReviewing: active,
              reviewCounts: active
                  ? const LibraryOrganizationCounts(
                      total: 72,
                      organized: 5,
                      pending: 65,
                      deferred: 1,
                      ignored: 1,
                    )
                  : null,
              reviewToolbar: active
                  ? Row(
                      key: const Key("second-row"),
                      mainAxisSize: MainAxisSize.min,
                      children: [
                        TextButton(onPressed: () {}, child: const Text("待整理")),
                        TextButton(onPressed: () {}, child: const Text("退出整理")),
                        TextButton(onPressed: () {}, child: const Text("继续整理")),
                      ],
                    )
                  : null,
              layoutShape: GalleryLayoutShape.square,
              thumbnailSize: GalleryThumbnailSize.medium,
              sortKey: LibraryGallerySortKey.captureTime,
              sortDirection: LibraryGallerySortDirection.descending,
              onBeginSelection: () {},
              onCancelSelection: () {},
              onSelectAll: () {},
              onLayoutShapeChanged: (_) {},
              onThumbnailSizeChanged: (_) {},
              onSortKeyChanged: (_) {},
              onSortDirectionChanged: (_) {},
            ),
            const Expanded(child: SizedBox.expand(key: Key("gallery"))),
          ],
        ),
      ),
    ),
  ),
);
