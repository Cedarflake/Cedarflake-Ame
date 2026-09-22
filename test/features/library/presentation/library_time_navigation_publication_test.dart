import "package:cedarflake_ame/features/library/domain/library_models.dart";
import "package:cedarflake_ame/features/library/presentation/gallery_view_options.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_gallery_layout.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_time_navigation.dart";
import "package:flutter/material.dart";
import "package:flutter_test/flutter_test.dart";

void main() {
  testWidgets(
    "a retired pointer cannot commit after replacement geometry arrives",
    (tester) async {
      final fixture = _PublicationFixture(tester);
      await fixture.show();
      final slider = find.byKey(const Key("timeline-slider"));
      final bounds = tester.getRect(slider);
      final gesture = await tester.startGesture(
        Offset(bounds.center.dx, bounds.top + bounds.height * 0.65),
      );
      await tester.pump();
      await fixture.show(revision: 2, hasGeometry: false);
      fixture.controller.jumpTo(1000);
      await fixture.show(revision: 2);
      await gesture.up();
      await tester.pump();
      await tester.pump(const Duration(milliseconds: 200));
      expect(
        fixture.controller.offset,
        1000,
        reason: "PointerUp belongs to the retired catalog revision",
      );
      expect(fixture.seeks, 0);
      await tester.tapAt(
        Offset(bounds.center.dx, bounds.top + bounds.height * 0.45),
      );
      await tester.pump();
      expect(
        fixture.controller.offset,
        isNot(1000),
        reason: "The replacement rail accepts a new pointer",
      );
    },
  );

  testWidgets("keeps the rail visible without granting stale geometry input", (
    tester,
  ) async {
    final fixture = _PublicationFixture(tester);
    await fixture.show();
    fixture.controller.jumpTo(1800);
    await tester.pump();
    final originalLine = tester.getRect(
      find.byKey(const Key("timeline-current-line")),
    );
    final oldSlider = tester.widget<Slider>(
      find.byKey(const Key("timeline-slider")),
    );

    await fixture.show(revision: 2, hasGeometry: false);
    expect(find.byKey(const Key("library-time-rail")), findsOneWidget);
    expect(
      tester.getRect(find.byKey(const Key("timeline-current-line"))),
      originalLine,
    );
    expect(
      tester.widget<Slider>(find.byKey(const Key("timeline-slider"))).onChanged,
      isNull,
    );
    expect(
      tester
          .widget<IconButton>(find.byKey(const Key("timeline-next")))
          .onPressed,
      isNull,
    );
    oldSlider.onChanged?.call(0.1);
    oldSlider.onChangeEnd?.call(0.1);
    await tester.pump(const Duration(milliseconds: 200));
    expect(fixture.controller.offset, 1800);
    expect(fixture.seeks, 0);

    await fixture.show(revision: 2);
    final currentSlider = tester.widget<Slider>(
      find.byKey(const Key("timeline-slider")),
    );
    expect(currentSlider.onChanged, isNotNull);
    currentSlider.onChanged!.call(0.3);
    await tester.pump();
    expect(fixture.controller.offset, isNot(1800));
  });

  for (final change in ["query", "empty", "layout", "controller"]) {
    testWidgets("does not retain a rail across a $change change", (
      tester,
    ) async {
      final fixture = _PublicationFixture(tester);
      await fixture.show();
      expect(find.byKey(const Key("library-time-rail")), findsOneWidget);
      if (change == "controller") {
        fixture.controller = ScrollController();
        addTearDown(fixture.controller.dispose);
      }
      await fixture.show(
        revision: 2,
        hasGeometry: false,
        queryId: change == "query" ? "other-query" : "query-1",
        isEmpty: change == "empty",
        shape: change == "layout"
            ? GalleryLayoutShape.equalHeight
            : GalleryLayoutShape.square,
      );
      expect(find.byKey(const Key("library-time-rail")), findsNothing);
    });
  }
}

class _PublicationFixture {
  _PublicationFixture(this.tester) {
    addTearDown(controller.dispose);
  }

  final WidgetTester tester;
  ScrollController controller = ScrollController();
  int seeks = 0;

  Future<void> show({
    int revision = 1,
    bool hasGeometry = true,
    bool isEmpty = false,
    String queryId = "query-1",
    GalleryLayoutShape shape = GalleryLayoutShape.square,
  }) async {
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: Row(
            children: [
              Expanded(
                child: ListView(
                  controller: controller,
                  children: const [SizedBox(height: 5000)],
                ),
              ),
              LibraryTimeNavigation(
                isLoading: false,
                scrollController: controller,
                layoutMetrics: hasGeometry
                    ? LibraryGalleryLayoutMetrics(
                        contentExtent: 5000,
                        photoRowHeight: 100,
                        dateAnchors: const [
                          LibraryGalleryDateAnchor(
                            id: "2026-09-01",
                            label: "2026",
                            scrollOffset: 0,
                            year: 2026,
                            isUnknown: false,
                          ),
                        ],
                        locationOffsets: const {},
                        itemOffsets: List.generate(
                          100,
                          (index) => index * 50.0,
                        ),
                      )
                    : null,
                timeline: LibraryTimeline(
                  revision: BigInt.from(revision),
                  queryId: queryId,
                  totalItems: isEmpty ? 0 : 100,
                  buckets: isEmpty
                      ? const []
                      : const [
                          LibraryTimeBucket(
                            monthKey: "2026-09",
                            itemCount: 50,
                            aspectRatioSum: 50,
                          ),
                          LibraryTimeBucket(
                            monthKey: "2012-03",
                            itemCount: 50,
                            aspectRatioSum: 50,
                          ),
                        ],
                ),
                layoutShape: shape,
                windowStartItemOffset: 0,
                loadedItemCount: 100,
                onSeek: (_, _) async {
                  seeks += 1;
                  return true;
                },
              ),
            ],
          ),
        ),
      ),
    );
  }
}
