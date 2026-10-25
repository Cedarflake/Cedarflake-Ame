import "package:cedarflake_ame/features/library/domain/library_models.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/annotated_time_rail.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_gallery_layout.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_sort_rail_projection.dart";
import "package:flutter/material.dart";
import "package:flutter_test/flutter_test.dart";

void main() {
  LibrarySortRailProjection projection(List<LibraryScrollSection> sections) =>
      LibrarySortRailProjection(
        timeline: LibraryTimeline(
          revision: BigInt.one,
          queryId: "query",
          totalItems: 4,
          buckets: const [],
          scrollSections: sections,
        ),
        metrics: LibraryGalleryLayoutMetrics(
          contentExtent: 1000,
          photoRowHeight: 200,
          dateAnchors: const [],
          locationOffsets: const {},
          itemOffsets: const [0, 200, 200, 700],
          isQueryWide: true,
        ),
      );

  test("no arrivals keeps the unannotated shared rail", () {
    final buckets = projection(const [
      LibraryScrollSection(
        kind: LibraryScrollSectionKind.earlier,
        key: "earlier",
        startOrdinal: 0,
        itemCount: 4,
      ),
    ]).buckets(800);
    expect(buckets, isEmpty);
  });

  testWidgets(
    "name labels use real row offsets and stay on the continuous slider",
    (tester) async {
      final buckets = projection(const [
        LibraryScrollSection(
          kind: LibraryScrollSectionKind.name,
          key: "0-9",
          startOrdinal: 0,
          itemCount: 1,
        ),
        LibraryScrollSection(
          kind: LibraryScrollSectionKind.name,
          key: "A",
          startOrdinal: 1,
          itemCount: 2,
        ),
        LibraryScrollSection(
          kind: LibraryScrollSectionKind.name,
          key: "Z",
          startOrdinal: 3,
          itemCount: 1,
        ),
      ]).buckets(800);
      expect(buckets.map((b) => b.scrollOffset), [0, 200, 700]);
      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(
            body: AnnotatedTimeRail(
              value: .3,
              buckets: buckets,
              maximumScrollOffset: 800,
              onChanged: (_) {},
            ),
          ),
        ),
      );
      await tester.pumpAndSettle();
      expect(find.text("0–9"), findsOneWidget);
      expect(find.text("A"), findsOneWidget);
      expect(find.text("Z"), findsOneWidget);
      expect(tester.widget<Slider>(find.byType(Slider)).divisions, isNull);
      expect(tester.takeException(), isNull);
    },
  );

  testWidgets("new-arrival highlight ends at the first earlier row", (
    tester,
  ) async {
    final buckets = projection(const [
      LibraryScrollSection(
        kind: LibraryScrollSectionKind.newArrivals,
        key: "new",
        startOrdinal: 0,
        itemCount: 3,
      ),
      LibraryScrollSection(
        kind: LibraryScrollSectionKind.earlier,
        key: "earlier",
        startOrdinal: 3,
        itemCount: 1,
      ),
    ]).buckets(800);
    expect(buckets[0].contentExtent, 700);
    expect(buckets[0].highlighted, isTrue);
    expect(buckets[1].highlighted, isFalse);
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: AnnotatedTimeRail(
            value: 0,
            buckets: buckets,
            maximumScrollOffset: 800,
            onChanged: (_) {},
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();
    expect(find.text("新加入"), findsOneWidget);
    expect(find.text("此前"), findsOneWidget);
    final range = tester.getRect(
      find.byKey(const ValueKey("rail-range-sort-new")),
    );
    final next = tester.getCenter(
      find.byKey(const ValueKey("time-marker-sort-earlier")),
    );
    expect(range.bottom, closeTo(next.dy, 1));
    expect(tester.takeException(), isNull);
  });
}
