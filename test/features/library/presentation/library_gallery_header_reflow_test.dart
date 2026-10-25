import "package:cedarflake_ame/features/library/presentation/gallery_selection.dart";
import "package:cedarflake_ame/features/library/presentation/gallery_view_options.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_gallery_header_actions.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_gallery_layout.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_gallery_wall.dart";
import "package:flutter/material.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:flutter_test/flutter_test.dart";

import "support/gallery_wall_fixture.dart";

void main() {
  for (final hasManifest in [false, true]) {
    for (final reducedMotion in [false, true]) {
      testWidgets("header motion keeps photo rows attached to the viewport: "
          "manifest=$hasManifest, reducedMotion=$reducedMotion", (
        tester,
      ) async {
        final reviewing = ValueNotifier(false);
        final scroll = ScrollController();
        final controller = RecordingGalleryController();
        final state = galleryBoundaryState(
          itemCount: 120,
          windowStart: 0,
          windowEnd: 120,
          hasPrevious: false,
          hasNext: false,
        );
        final manifest = hasManifest ? galleryManifest(120) : null;
        LibraryGalleryLayoutMetrics? metrics;
        addTearDown(() async {
          reviewing.dispose();
          scroll.dispose();
          await tester.binding.setSurfaceSize(null);
        });
        await tester.binding.setSurfaceSize(const Size(1000, 700));
        await tester.pumpWidget(
          ProviderScope(
            child: MaterialApp(
              home: Scaffold(
                body: MediaQuery(
                  data: MediaQueryData(disableAnimations: reducedMotion),
                  child: ValueListenableBuilder(
                    valueListenable: reviewing,
                    builder: (context, active, _) => Column(
                      children: [
                        LibraryGalleryHeaderActions(
                          title: const Text("图库"),
                          summary: const Text("120 张图片"),
                          primary: const SizedBox(width: 200, height: 48),
                          secondary: active
                              ? const SizedBox(width: 280, height: 40)
                              : null,
                        ),
                        Expanded(
                          child: LibraryGalleryWall(
                            state: state,
                            controller: controller,
                            scrollController: scroll,
                            layoutShape: GalleryLayoutShape.equalHeight,
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
                            onLoadPrevious: controller.loadPreviousPage,
                            onLayoutChanged: (layout, _) => metrics = layout,
                          ),
                        ),
                      ],
                    ),
                  ),
                ),
              ),
            ),
          ),
        );
        await tester.pumpAndSettle();
        scroll.jumpTo(
          metrics!.offsetForGlobalItemIndex(60)! -
              scroll.position.viewportDimension / 2 +
              10,
        );
        await tester.pumpAndSettle();
        final tile = find.byKey(const ValueKey("location-60"));
        final wall = find.byKey(const Key("library-photo-wall"));
        final initialTile = tester.getRect(tile);
        final initialRelativeTop = initialTile.top - tester.getTopLeft(wall).dy;
        final initialScroll = scroll.offset;
        final initialExtent = scroll.position.viewportDimension;

        Future<void> expectAttachedFrame() async {
          await tester.pump(const Duration(milliseconds: 16));
          final rect = tester.getRect(tile);
          expect(rect.width, closeTo(initialTile.width, 0.01));
          expect(rect.height, closeTo(initialTile.height, 0.01));
          expect(rect.left, closeTo(initialTile.left, 0.01));
          final anchorShift = hasManifest
              ? (scroll.position.viewportDimension - initialExtent) / 2
              : 0.0;
          expect(
            rect.top - tester.getTopLeft(wall).dy,
            closeTo(initialRelativeTop + anchorShift, 0.01),
            reason: "The existing anchor follows the current frame's viewport",
          );
          expect(scroll.offset, closeTo(initialScroll - anchorShift, 0.01));
        }

        for (final active in [true, false, true, false]) {
          reviewing.value = active;
          await tester.pump();
          for (var frame = 0; frame < 5; frame++) {
            await expectAttachedFrame();
          }
        }
        await tester.pump(const Duration(milliseconds: 300));
        await expectAttachedFrame();
        expect(tester.binding.transientCallbackCount, 0);
        expect(controller.nextPageRequests, 0);
        expect(controller.previousPageRequests, 0);
        expect(tester.takeException(), isNull);
        await tester.pumpWidget(const SizedBox.shrink());
      });
    }
  }
}
