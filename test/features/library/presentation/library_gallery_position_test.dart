import "package:cedarflake_ame/features/library/presentation/widgets/library_gallery_position.dart";
import "package:flutter/widgets.dart";
import "package:flutter_test/flutter_test.dart";

void main() {
  testWidgets("later input resolves from the displayed geometry", (
    tester,
  ) async {
    final scroll = await _attachScroll(tester);
    final original = _position("original");
    final latest = _position("latest");
    final transition = LibraryGalleryLayoutTransition(
      generation: 1,
      position: original,
      scrollPosition: scroll.position,
    );
    final resolver = LibraryGalleryPositionResolver();
    var reads = 0;
    resolver.update(
      queryId: original.queryId,
      revision: original.revision,
      resolve: (offset, viewport) {
        reads += 1;
        expect(offset, 500);
        expect(viewport, scroll.position.viewportDimension);
        return latest;
      },
    );
    expect(
      transition.resolvePosition(
        current: scroll.position,
        resolver: resolver,
        fallback: original,
      ),
      same(original),
    );
    expect(reads, 0);
    scroll.jumpTo(500);
    expect(
      transition.resolvePosition(
        current: scroll.position,
        resolver: resolver,
        fallback: original,
      ),
      same(latest),
    );
    expect(reads, 1);
  });

  for (final identity in [
    (queryId: "new-query", revision: 1),
    (queryId: "query", revision: 2),
  ]) {
    testWidgets("rejects a later position from $identity", (tester) async {
      final scroll = await _attachScroll(tester);
      final transition = LibraryGalleryLayoutTransition(
        generation: 1,
        position: _position("original"),
        scrollPosition: scroll.position,
      );
      final incompatible = _position(
        "incompatible",
        queryId: identity.queryId,
        revision: identity.revision,
      );
      final resolver = LibraryGalleryPositionResolver()
        ..update(
          queryId: incompatible.queryId,
          revision: incompatible.revision,
          resolve: (_, _) => incompatible,
        );
      scroll.jumpTo(500);
      expect(
        transition.resolvePosition(
          current: scroll.position,
          resolver: resolver,
          fallback: incompatible,
        ),
        isNull,
      );
    });
  }

  testWidgets("detachment cannot read retired geometry", (tester) async {
    final scroll = await _attachScroll(tester);
    final original = _position("original");
    final fallback = _position("fallback");
    final transition = LibraryGalleryLayoutTransition(
      generation: 1,
      position: original,
      scrollPosition: scroll.position,
    );
    final resolver = LibraryGalleryPositionResolver()
      ..update(
        queryId: original.queryId,
        revision: original.revision,
        resolve: (_, _) => fail("Detached geometry was consulted"),
      );
    await tester.pumpWidget(const SizedBox());
    expect(scroll.hasClients, isFalse);
    expect(
      transition.resolvePosition(
        current: null,
        resolver: resolver,
        fallback: fallback,
      ),
      same(fallback),
    );
    expect(
      transition.resolvePosition(
        current: null,
        resolver: resolver,
        fallback: null,
      ),
      isNull,
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
