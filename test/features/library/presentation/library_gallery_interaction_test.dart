import "package:cedarflake_ame/app/presentation/ame_scroll_motion.dart";
import "package:cedarflake_ame/features/library/domain/library_state.dart";
import "package:cedarflake_ame/features/library/presentation/gallery_selection.dart";
import "package:cedarflake_ame/features/library/presentation/gallery_view_options.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_gallery_wall.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_virtual_gallery_geometry.dart";
import "package:flutter/gestures.dart";
import "package:flutter/material.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:flutter_test/flutter_test.dart";

import "support/gallery_wall_fixture.dart";

void main() {
  for (final withManifest in [false, true]) {
    for (final shape in GalleryLayoutShape.values) {
      testWidgets(
        "wheel preserves feedback and loads both edges ($withManifest, $shape)",
        (tester) async {
          final fixture = await _mountWall(tester, withManifest, shape);
          fixture.scroll.jumpTo(fixture.geometry.leadingExtent + 100);
          await tester.pump();
          expect(fixture.controller.previousPageRequests, 0);
          expect(fixture.controller.nextPageRequests, 0);
          fixture.positions.clear();
          fixture.controller.previewDemandRequests.clear();

          await _wheel(tester, -60);
          expect(fixture.controller.previousPageRequests, 1);
          expect(fixture.controller.nextPageRequests, 0);
          expect(fixture.positions, isNotEmpty);
          expect(
            fixture.controller.previewDemandRequests.last.visible,
            isNotEmpty,
          );
          final position = fixture.positions.last;
          expect(position.queryId, "resize-query");
          expect(position.revision, BigInt.one);
          expect(
            fixture.ranges.last.containsGlobalItemIndex(
              position.globalItemIndex,
            ),
            isTrue,
          );

          fixture.scroll.jumpTo(
            fixture.geometry.loadedEndExtent -
                fixture.scroll.position.viewportDimension -
                100,
          );
          await tester.pump();
          final before = fixture.scroll.offset;
          await _wheel(tester, 60);
          expect(fixture.scroll.offset, closeTo(before + 60, 0.01));
          expect(fixture.controller.nextPageRequests, 1);
          expect(fixture.controller.previousPageRequests, 1);
          expect(tester.takeException(), isNull);
        },
      );

      testWidgets(
        "direct drag loads an edge without a wheel ($withManifest, $shape)",
        (tester) async {
          final fixture = await _mountWall(tester, withManifest, shape);
          fixture.scroll.jumpTo(fixture.geometry.leadingExtent + 100);
          await tester.pump();
          await tester.drag(
            find.byKey(const Key("library-photo-wall")),
            const Offset(0, 100),
          );
          await tester.pumpAndSettle();
          expect(fixture.controller.previousPageRequests, greaterThan(0));
          expect(fixture.controller.nextPageRequests, 0);
          expect(fixture.positions, isNotEmpty);
          expect(tester.takeException(), isNull);
        },
      );

      for (final blocked in ["loading-next", "loading-previous", "errors"]) {
        testWidgets(
          "wheel keeps moving without automatic retry during $blocked ($withManifest, $shape)",
          (tester) async {
            final state = _state().copyWith(
              isLoadingPage: blocked == "loading-next",
              isLoadingPreviousPage: blocked == "loading-previous",
              pageErrorMessage: blocked == "errors" ? "next page failed" : null,
              previousPageErrorMessage: blocked == "errors"
                  ? "previous page failed"
                  : null,
            );
            final fixture = await _mountWall(
              tester,
              withManifest,
              shape,
              state: state,
            );
            fixture.scroll.jumpTo(fixture.geometry.leadingExtent + 100);
            await tester.pump();
            await _wheel(tester, -60);
            fixture.scroll.jumpTo(
              fixture.geometry.loadedEndExtent -
                  fixture.scroll.position.viewportDimension -
                  100,
            );
            await tester.pump();
            final before = fixture.scroll.offset;
            await _wheel(tester, 60);
            expect(fixture.scroll.offset, closeTo(before + 60, 0.01));
            expect(fixture.controller.previousPageRequests, 0);
            expect(fixture.controller.nextPageRequests, 0);
            expect(tester.takeException(), isNull);
          },
        );
      }
    }
  }
}

LibraryState _state() => galleryBoundaryState(
  itemCount: 1000,
  windowStart: 200,
  windowEnd: 300,
  hasPrevious: true,
  hasNext: true,
);

class _WallFixture {
  final controller = RecordingGalleryController();
  final scroll = AmeScrollController();
  final positions = <LibraryGalleryVisiblePosition>[];
  final ranges = <LibraryGalleryVisibleRange>[];
  late LibraryVirtualGalleryGeometry geometry;
}

Future<_WallFixture> _mountWall(
  WidgetTester tester,
  bool withManifest,
  GalleryLayoutShape shape, {
  LibraryState? state,
}) async {
  final fixture = _WallFixture();
  final currentState = state ?? _state();
  addTearDown(fixture.scroll.dispose);
  await tester.pumpWidget(
    ProviderScope(
      child: MaterialApp(
        home: MediaQuery(
          data: const MediaQueryData(disableAnimations: true),
          child: LibraryGalleryWall(
            state: currentState,
            controller: fixture.controller,
            scrollController: fixture.scroll,
            layoutShape: shape,
            thumbnailSize: GalleryThumbnailSize.medium,
            selection: GallerySelection.empty(currentState.queryId),
            isSelecting: false,
            layoutManifest: withManifest ? galleryManifest(1000) : null,
            onOpen: (_) {},
            onToggleSelection: (_) {},
            onViewInformation: (_) {},
            onCopyPath: (_) {},
            onRevealFile: (_) {},
            onVisiblePositionChanged: fixture.positions.add,
            onVisibleRangeChanged: fixture.ranges.add,
            onLoadPrevious: fixture.controller.loadPreviousPage,
            onLayoutChanged: (_, geometry) => fixture.geometry = geometry,
          ),
        ),
      ),
    ),
  );
  await tester.pump();
  return fixture;
}

Future<void> _wheel(WidgetTester tester, double delta) async {
  await tester.sendEventToBinding(
    PointerScrollEvent(
      position: tester.getCenter(find.byKey(const Key("library-photo-wall"))),
      scrollDelta: Offset(0, delta),
    ),
  );
  await tester.pumpAndSettle();
}
