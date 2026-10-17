import "package:cedarflake_ame/features/library/presentation/library_gallery_layout_continuity.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_gallery_position.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_gallery_reflow.dart";
import "package:flutter/widgets.dart";
import "package:flutter_test/flutter_test.dart";

void main() {
  testWidgets("later scrolling retires edge intent before acknowledgement", (
    tester,
  ) async {
    final scroll = await _attachScroll(tester);
    final continuity = LibraryGalleryLayoutContinuity();
    addTearDown(continuity.clear);
    scroll.jumpTo(0);
    final transition = continuity.begin(
      generation: 1,
      current: _position("top"),
      scrollPosition: scroll.position,
    );
    expect(transition.scrollEdge, LibraryGalleryScrollEdge.start);
    scroll.jumpTo(300);
    continuity.complete(1);
    expect(transition.anchorForScrollPosition(scroll.position), isNull);
    expect(
      transition.constrainScrollOffset(
        current: scroll.position,
        imageOffset: 350,
        maximum: 1000,
      ),
      350,
    );
    expect(
      continuity
          .begin(
            generation: 2,
            current: _position("middle"),
            scrollPosition: scroll.position,
          )
          .scrollEdge,
      isNull,
    );
  });

  testWidgets("consecutive layouts retain one image through corrections", (
    tester,
  ) async {
    final scroll = await _attachScroll(tester);
    final continuity = LibraryGalleryLayoutContinuity();
    addTearDown(continuity.clear);
    final original = _position("original");
    continuity.begin(
      generation: 1,
      current: original,
      scrollPosition: scroll.position,
    );
    scroll.position.correctBy(150);
    continuity.complete(1);
    expect(
      continuity
          .begin(
            generation: 2,
            current: _position("new-center-cell"),
            scrollPosition: scroll.position,
          )
          .position,
      same(original),
    );
    scroll.position.correctBy(-100);
    continuity.complete(1);
    expect(
      continuity
          .begin(
            generation: 3,
            current: _position("another-center-cell"),
            scrollPosition: scroll.position,
          )
          .position,
      same(original),
      reason: "An older acknowledgement cannot end the latest layout change",
    );
  });

  for (final completeBeforeInput in [false, true]) {
    testWidgets("scroll releases the anchor, complete=$completeBeforeInput", (
      tester,
    ) async {
      final scroll = await _attachScroll(tester);
      final continuity = LibraryGalleryLayoutContinuity();
      addTearDown(continuity.clear);
      continuity.begin(
        generation: 1,
        current: _position("old"),
        scrollPosition: scroll.position,
      );
      if (completeBeforeInput) {
        continuity.complete(1);
      }
      scroll.jumpTo(500);
      continuity.complete(1);
      final latest = _position("after-scroll");
      expect(
        continuity
            .begin(
              generation: 2,
              current: latest,
              scrollPosition: scroll.position,
            )
            .position,
        same(latest),
      );
    });
  }

  for (final identity in [
    (queryId: "new-query", revision: 1),
    (queryId: "query", revision: 2),
  ]) {
    testWidgets("new publication releases the anchor: $identity", (
      tester,
    ) async {
      final scroll = await _attachScroll(tester);
      final continuity = LibraryGalleryLayoutContinuity();
      addTearDown(continuity.clear);
      continuity.begin(
        generation: 1,
        current: _position("old"),
        scrollPosition: scroll.position,
      );
      final latest = _position(
        "new",
        queryId: identity.queryId,
        revision: identity.revision,
      );
      expect(
        continuity
            .begin(
              generation: 2,
              current: latest,
              scrollPosition: scroll.position,
            )
            .position,
        same(latest),
      );
    });
  }

  testWidgets("detached and explicitly cleared anchors cannot be reused", (
    tester,
  ) async {
    final scroll = await _attachScroll(tester);
    final continuity = LibraryGalleryLayoutContinuity();
    addTearDown(continuity.clear);
    final latest = _position("new");
    continuity.begin(
      generation: 1,
      current: _position("old"),
      scrollPosition: scroll.position,
    );
    expect(
      continuity
          .begin(generation: 2, current: latest, scrollPosition: null)
          .position,
      same(latest),
    );
    continuity.begin(
      generation: 3,
      current: _position("old"),
      scrollPosition: scroll.position,
    );
    continuity.clear();
    continuity.complete(3);
    expect(
      continuity
          .begin(
            generation: 4,
            current: latest,
            scrollPosition: scroll.position,
          )
          .position,
      same(latest),
    );
  });
}

LibraryGalleryVisiblePosition _position(
  String locationId, {
  String queryId = "query",
  int revision = 1,
}) => LibraryGalleryVisiblePosition(
  queryId: queryId,
  revision: BigInt.from(revision),
  monthKey: null,
  locationId: locationId,
  globalItemIndex: 10,
  itemFraction: 0.3,
  viewportFraction: 0.5,
);

Future<ScrollController> _attachScroll(WidgetTester tester) async {
  final scroll = ScrollController(initialScrollOffset: 200);
  addTearDown(scroll.dispose);
  await tester.pumpWidget(
    Directionality(
      textDirection: TextDirection.ltr,
      child: ListView.builder(
        controller: scroll,
        itemCount: 100,
        itemExtent: 100,
        itemBuilder: (_, _) => const SizedBox(),
      ),
    ),
  );
  return scroll;
}
