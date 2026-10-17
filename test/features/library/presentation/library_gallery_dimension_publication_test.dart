import "dart:typed_data";

import "package:cedarflake_ame/features/library/domain/gallery_layout_manifest.dart";
import "package:cedarflake_ame/features/library/domain/library_state.dart";
import "package:cedarflake_ame/features/library/presentation/library_gallery_dimension_publication.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_gallery_wall.dart";
import "package:flutter_test/flutter_test.dart";

void main() {
  testWidgets("quiet publication retains its anchor and caches the manifest", (
    tester,
  ) async {
    final fixture = _DimensionFixture();
    addTearDown(fixture.owner.dispose);
    final originalAnchor = fixture.anchor;
    fixture.owner.accept(fixture.update(0));
    fixture.anchor = fixture.position(1);
    await tester.pump(const Duration(milliseconds: 159));
    expect(fixture.publishedAnchors, isEmpty);
    await tester.pump(const Duration(milliseconds: 1));
    expect(fixture.publishedAnchors, [originalAnchor]);
    final manifest = fixture.manifest();
    final updated = fixture.owner.manifestWithUpdates(manifest);
    expect(updated.hasKnownDimensionsAt(0), isTrue);
    expect(updated.aspectRatioAt(0), 2);
    expect(fixture.owner.manifestWithUpdates(manifest), same(updated));
  });

  testWidgets("continuous updates publish within the maximum delay", (
    tester,
  ) async {
    final fixture = _DimensionFixture(end: 8);
    addTearDown(fixture.owner.dispose);
    for (var index = 0; index < 6; index += 1) {
      fixture.owner.accept(fixture.update(index));
      await tester.pump(const Duration(milliseconds: 100));
    }
    expect(fixture.publishedAnchors, hasLength(1));
    final updated = fixture.owner.manifestWithUpdates(fixture.manifest());
    for (var index = 0; index < 6; index += 1) {
      expect(updated.hasKnownDimensionsAt(index), isTrue);
    }
  });

  testWidgets("scrolling reclassifies offscreen updates before publication", (
    tester,
  ) async {
    final fixture = _DimensionFixture();
    addTearDown(fixture.owner.dispose);
    fixture.owner.accept(fixture.update(0));
    fixture.owner.accept(fixture.update(4));
    fixture.owner.setUserScrolling(true);
    fixture.owner.updateVisibleRange(fixture.range(4, 6));
    fixture.anchor = fixture.position(4);
    await tester.pump(const Duration(seconds: 1));
    expect(fixture.publishedAnchors, isEmpty);
    fixture.owner.setUserScrolling(false);
    expect(fixture.events.sublist(fixture.events.length - 2), [
      "scroll:false",
      "capture",
    ]);
    await tester.pump(const Duration(milliseconds: 160));
    expect(fixture.publishedAnchors.single, same(fixture.anchor));
    var updated = fixture.owner.manifestWithUpdates(fixture.manifest());
    expect(updated.hasKnownDimensionsAt(0), isFalse);
    expect(updated.hasKnownDimensionsAt(4), isTrue);

    fixture.owner.setUserScrolling(true);
    fixture.owner.updateVisibleRange(fixture.range(0, 2));
    fixture.anchor = fixture.position(0);
    fixture.owner.setUserScrolling(false);
    await tester.pump(const Duration(milliseconds: 160));
    updated = fixture.owner.manifestWithUpdates(fixture.manifest());
    expect(updated.hasKnownDimensionsAt(0), isTrue);
    expect(updated.hasKnownDimensionsAt(4), isTrue);
  });

  for (final changesRevision in [false, true]) {
    testWidgets(
      "new context rejects old dimensions, revision: $changesRevision",
      (tester) async {
        final fixture = _DimensionFixture();
        addTearDown(fixture.owner.dispose);
        final stale = fixture.update(0);
        fixture.owner.accept(stale);
        fixture.state = fixture.state.copyWith(
          catalogRevision: changesRevision ? BigInt.two : BigInt.one,
          queryId: changesRevision ? "query" : "new-query",
        );
        fixture.owner.synchronizeContext(
          fixture.state.catalogRevision,
          fixture.state.queryId,
        );
        fixture.owner.accept(stale);
        await tester.pump(const Duration(seconds: 1));
        expect(fixture.publishedAnchors, isEmpty);
        fixture.owner.updateVisibleRange(fixture.range(0, 2));
        fixture.owner.accept(fixture.update(1));
        await tester.pump(const Duration(milliseconds: 160));
        final updated = fixture.owner.manifestWithUpdates(fixture.manifest());
        expect(updated.hasKnownDimensionsAt(0), isFalse);
        expect(updated.hasKnownDimensionsAt(1), isTrue);
      },
    );
  }

  for (final blocksAfterScheduling in [false, true]) {
    testWidgets(
      "a pending transition retains dimensions, late block: $blocksAfterScheduling",
      (tester) async {
        final fixture = _DimensionFixture();
        addTearDown(fixture.owner.dispose);
        fixture.hasPendingTransition = !blocksAfterScheduling;
        fixture.owner.accept(fixture.update(0));
        fixture.hasPendingTransition = true;
        await tester.pump(const Duration(seconds: 1));
        expect(fixture.publishedAnchors, isEmpty);
        fixture.hasPendingTransition = false;
        fixture.owner.schedulePublication();
        await tester.pump(const Duration(milliseconds: 160));
        expect(fixture.publishedAnchors, hasLength(1));
      },
    );
  }

  testWidgets("timeline navigation cancels timing and captures a new range", (
    tester,
  ) async {
    final fixture = _DimensionFixture();
    addTearDown(fixture.owner.dispose);
    fixture.owner.accept(fixture.update(0));
    fixture.owner.accept(fixture.update(4));
    fixture.owner.beginNavigation();
    await tester.pump(const Duration(seconds: 1));
    expect(fixture.publishedAnchors, isEmpty);
    fixture.anchor = fixture.position(4);
    fixture.owner.updateVisibleRange(fixture.range(4, 6));
    await tester.pump(const Duration(milliseconds: 160));
    expect(fixture.publishedAnchors.single, same(fixture.anchor));
    final updated = fixture.owner.manifestWithUpdates(fixture.manifest());
    expect(updated.hasKnownDimensionsAt(0), isFalse);
    expect(updated.hasKnownDimensionsAt(4), isTrue);
  });

  testWidgets("disposal prevents publication and late state access", (
    tester,
  ) async {
    final fixture = _DimensionFixture();
    fixture.owner.accept(fixture.update(0));
    fixture.owner.dispose();
    fixture.isRetired = true;
    fixture.owner.accept(fixture.update(1));
    fixture.owner.updateVisibleRange(fixture.range(2, 4));
    fixture.owner.setUserScrolling(true);
    fixture.owner.schedulePublication();
    await tester.pump(const Duration(seconds: 1));
    expect(fixture.publishedAnchors, isEmpty);
    expect(fixture.events, ["capture"]);
  });
}

class _DimensionFixture {
  _DimensionFixture({int end = 2}) {
    anchor = position(0);
    owner = LibraryGalleryDimensionPublication(
      readState: () {
        expectSync(isRetired, isFalse);
        return state;
      },
      capturePosition: (_) {
        events.add("capture");
        return anchor;
      },
      hasPendingTransition: () => hasPendingTransition,
      onUserScrollingChanged: (scrolling) => events.add("scroll:$scrolling"),
      onPublished: publishedAnchors.add,
    );
    owner.synchronizeContext(state.catalogRevision, state.queryId);
    owner.updateVisibleRange(range(0, end));
  }

  late final LibraryGalleryDimensionPublication owner;
  LibraryState state = LibraryState(
    catalogRevision: BigInt.one,
    queryId: "query",
  );
  late LibraryGalleryVisiblePosition anchor;
  final List<LibraryGalleryVisiblePosition?> publishedAnchors = [];
  final List<String> events = [];
  bool hasPendingTransition = false;
  bool isRetired = false;

  LibraryGalleryVisiblePosition position(int index) =>
      LibraryGalleryVisiblePosition(
        queryId: state.queryId,
        revision: state.catalogRevision ?? BigInt.zero,
        monthKey: null,
        locationId: "location-$index",
        globalItemIndex: index,
        itemFraction: 0.25,
        viewportFraction: 0.5,
      );

  LibraryGalleryVisibleRange range(int start, int end) =>
      LibraryGalleryVisibleRange(
        queryId: state.queryId,
        revision: state.catalogRevision ?? BigInt.zero,
        startGlobalItemIndex: start,
        endGlobalItemIndexExclusive: end,
      );

  LibraryGalleryLayoutDimensionUpdate update(int index) =>
      LibraryGalleryLayoutDimensionUpdate(
        revision: state.catalogRevision ?? BigInt.zero,
        queryId: state.queryId,
        globalItemIndex: index,
        locationId: "location-$index",
        width: 200,
        height: 100,
      );

  LibraryGalleryLayoutManifest manifest() {
    final revision = state.catalogRevision ?? BigInt.zero;
    final builder = LibraryGalleryLayoutManifestBuilder(
      revision: revision,
      queryId: state.queryId,
      totalItems: 8,
    );
    builder.append(
      LibraryGalleryLayoutManifestChunk(
        revision: revision,
        queryId: state.queryId,
        totalItems: 8,
        startOrdinal: 0,
        locationIds: List.generate(8, (index) => "location-$index"),
        aspectRatioMilli: Uint16List.fromList(List.filled(8, 1000)),
        dateGroupIndices: Uint16List(8),
        dateGroups: const [null],
        flags: Uint8List(8),
      ),
    );
    return builder.build();
  }
}
