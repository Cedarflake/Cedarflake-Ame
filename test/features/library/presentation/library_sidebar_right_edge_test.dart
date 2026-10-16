import "package:cedarflake_ame/features/library/domain/library_models.dart";
import "package:cedarflake_ame/features/library/presentation/gallery_view_options.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/annotated_time_rail.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_gallery_header.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_sidebar_transition.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_sidebar_content.dart";
import "package:flutter/material.dart";
import "package:flutter_test/flutter_test.dart";

void main() {
  for (final width in [800.0, 1000.0, 1280.0]) {
    for (final scale in [1.0, 2.0]) {
      testWidgets(
        "sidebar keeps toolbar and time rail anchored at $width and ${scale}x text",
        (tester) async {
          await tester.binding.setSurfaceSize(Size(width, 900));
          addTearDown(() => tester.binding.setSurfaceSize(null));
          final expanded = ValueNotifier(true);
          addTearDown(expanded.dispose);
          await tester.pumpWidget(_fixture(expanded, scale));
          await tester.pumpAndSettle();
          final toolbar = find.byKey(const Key("library-more-menu"));
          final rail = find.byType(AnnotatedTimeRail);
          void expectRightEdges() {
            expect(tester.getTopRight(toolbar).dx, closeTo(width - 20, 0.01));
            expect(tester.getTopRight(rail).dx, closeTo(width, 0.01));
            for (final key in ["library-gallery-title", "library-summary"]) {
              expect(
                tester.getTopLeft(find.byKey(Key(key))).dx,
                closeTo(
                  tester
                          .getTopLeft(
                            find.byKey(const Key("library-gallery-transition")),
                          )
                          .dx +
                      28,
                  0.01,
                ),
              );
            }
            expect(
              tester.getTopLeft(find.byKey(const Key("first-photo"))).dx,
              closeTo(
                tester
                        .getTopLeft(
                          find.byKey(const Key("library-gallery-transition")),
                        )
                        .dx +
                    24,
                0.01,
              ),
            );
            expect(tester.takeException(), isNull);
          }

          expectRightEdges();
          for (final nextExpanded in [false, true]) {
            expanded.value = nextExpanded;
            await tester.pump();
            expectRightEdges();
            for (var frame = 0; frame < 6; frame++) {
              await tester.pump(const Duration(milliseconds: 30));
              expectRightEdges();
            }
            await tester.pumpAndSettle();
            expectRightEdges();
          }
          await tester.pumpWidget(const SizedBox.shrink());
        },
      );
    }
  }
}

Widget _fixture(ValueNotifier<bool> expanded, double scale) => MaterialApp(
  builder: (context, child) => MediaQuery(
    data: MediaQuery.of(context).copyWith(textScaler: TextScaler.linear(scale)),
    child: child!,
  ),
  home: Scaffold(
    body: ValueListenableBuilder(
      valueListenable: expanded,
      builder: (_, value, _) => LibrarySidebarTransition(
        expanded: value,
        sidebarWidth: 260,
        sidebar: const Text("Folders"),
        resizeHandle: const SizedBox.shrink(),
        gallery: Column(
          children: [
            LibraryGalleryHeader(
              galleryTitle: "图库",
              totalItems: 72,
              selectedCount: 0,
              isSelecting: false,
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
            Expanded(
              child: Row(
                children: [
                  const Expanded(
                    child: LibrarySidebarContent(
                      child: Padding(
                        padding: EdgeInsets.only(left: 24),
                        child: Align(
                          alignment: Alignment.topLeft,
                          child: SizedBox(
                            key: Key("first-photo"),
                            width: 100,
                            height: 100,
                          ),
                        ),
                      ),
                    ),
                  ),
                  AnnotatedTimeRail(
                    value: 0,
                    onChanged: (_) {},
                    buckets: const [
                      TimelineRailBucket(
                        id: "first",
                        label: "2026年1月",
                        contentExtent: 500,
                        year: 2026,
                      ),
                      TimelineRailBucket(
                        id: "last",
                        label: "2025年1月",
                        contentExtent: 500,
                        year: 2025,
                      ),
                    ],
                  ),
                ],
              ),
            ),
          ],
        ),
      ),
    ),
  ),
);
