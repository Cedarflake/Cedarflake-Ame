import "package:cedarflake_ame/features/library/domain/library_models.dart";
import "package:cedarflake_ame/features/library/presentation/coordination/library_gallery_motion_publication.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_gallery_layout_motion.dart";
import "package:material_ui/material_ui.dart";
import "package:flutter/rendering.dart";
import "package:flutter_test/flutter_test.dart";

void main() {
  for (final background in [false, true]) {
    testWidgets(
      "duplicate reflow retires actions and reverses, background=$background",
      (tester) async {
        final query = LibraryGalleryQuery(
          duplicateMode: background
              ? LibraryGalleryDuplicateMode.merged
              : LibraryGalleryDuplicateMode.all,
        );
        final publication = ValueNotifier<_DuplicateView>((
          query: query,
          ids: [0, 1, 2, 3, 4, 5],
          revision: 1,
        ));
        final scroll = ScrollController();
        final taps = <int>[];
        addTearDown(publication.dispose);
        addTearDown(scroll.dispose);
        await tester.pumpWidget(_duplicateFixture(publication, scroll, taps));
        await tester.pumpAndSettle();
        await tester.tapAt(const Offset(150, 50));
        expect(taps, [1]);
        taps.clear();
        final retained = find.byKey(const Key("merge-photo-2"));
        final before = tester.getRect(retained);
        publication.value = (
          query: query.copyWith(
            duplicateMode: LibraryGalleryDuplicateMode.merged,
          ),
          ids: [0, 2, 3, 4, 5],
          revision: 2,
        );
        await tester.pump();
        await tester.pump();
        expect(tester.getRect(retained), before);
        expect(find.byKey(const Key("merge-photo-1")), findsNothing);
        final exit = find.byType(RawImage);
        expect(exit, findsOneWidget);
        final pixels = tester.widget<RawImage>(exit).image!;
        expect(pixels.debugDisposed, isFalse);
        await tester.tapAt(const Offset(150, 50));
        expect(taps, isNot(contains(1)));
        await tester.pump(const Duration(milliseconds: 60));
        final midway = tester.getRect(retained);
        expect(midway.left, lessThan(before.left));
        expect(midway.left, greaterThan(100));
        expect(scroll.offset, 0);

        publication.value = (
          query: query.copyWith(duplicateMode: LibraryGalleryDuplicateMode.all),
          ids: [0, 1, 2, 3, 4, 5],
          revision: 3,
        );
        await tester.pump();
        expect(
          tester.getRect(retained),
          rectMoreOrLessEquals(midway, epsilon: 0.000001),
        );
        await tester.pumpAndSettle();
        expect(tester.getRect(retained), before);
        expect(find.byType(RawImage), findsNothing);
        expect(pixels.debugDisposed, isTrue);

        publication.value = (
          query: query.copyWith(
            duplicateMode: LibraryGalleryDuplicateMode.merged,
            rootId: "another-folder",
          ),
          ids: [0, 2, 3, 4, 5],
          revision: 4,
        );
        await tester.pump();
        expect(tester.getTopLeft(retained), const Offset(100, 0));
        expect(find.byType(RawImage), findsNothing);
        expect(tester.binding.transientCallbackCount, 0);
        expect(tester.takeException(), isNull);
        await tester.pumpWidget(const SizedBox.shrink());
      },
    );
  }

  testWidgets("resize preserves a tile across row changes and rapid reversal", (
    tester,
  ) async {
    final width = ValueNotifier(600.0);
    final scroll = ScrollController();
    addTearDown(width.dispose);
    addTearDown(scroll.dispose);
    await tester.pumpWidget(_fixture(width, scroll));
    await tester.pumpAndSettle();
    final tile = find.byKey(const Key("photo-2"));
    final before = tester.getRect(tile);
    expect(before.left, 400);
    expect(before.top, 0);
    width.value = 360;
    await tester.pump();
    expect(tester.getRect(tile), before);
    await tester.pump(const Duration(milliseconds: 90));
    final middle = tester.getRect(tile);
    expect(middle.left, greaterThan(0));
    expect(middle.left, lessThan(before.left));
    expect(middle.top, greaterThan(0));
    expect(middle.width, lessThan(before.width));
    width.value = 600;
    await tester.pump();
    expect(
      tester.getRect(tile),
      rectMoreOrLessEquals(middle, epsilon: 0.000001),
    );
    await tester.pumpAndSettle();
    expect(tester.getRect(tile), before);
    expect(tester.takeException(), isNull);
    await tester.pumpWidget(const SizedBox.shrink());
  });

  testWidgets("query changes do not animate unrelated images", (tester) async {
    final width = ValueNotifier(600.0);
    final scroll = ScrollController();
    addTearDown(width.dispose);
    addTearDown(scroll.dispose);
    await tester.pumpWidget(_fixture(width, scroll));
    await tester.pumpAndSettle();
    width.value = 360;
    await tester.pumpWidget(_fixture(width, scroll, query: "different-query"));
    expect(
      tester.getTopLeft(find.byKey(const Key("photo-2"))),
      const Offset(0, 180),
    );
    expect(tester.binding.transientCallbackCount, 0);
    await tester.pumpWidget(const SizedBox.shrink());
  });

  testWidgets("scroll end preserves reflow until a new user scroll", (
    tester,
  ) async {
    final width = ValueNotifier(600.0);
    final scroll = ScrollController();
    addTearDown(width.dispose);
    addTearDown(scroll.dispose);
    await tester.pumpWidget(_fixture(width, scroll));
    await tester.pumpAndSettle();
    width.value = 360;
    await tester.pump();
    await tester.pump(const Duration(milliseconds: 60));
    final tile = find.byKey(const Key("photo-2"));
    final middle = tester.getRect(tile);
    final context = scroll.position.context.notificationContext!;
    UserScrollNotification(
      metrics: scroll.position,
      context: context,
      direction: ScrollDirection.idle,
    ).dispatch(context);
    await tester.pump();
    expect(tester.getRect(tile), middle);
    UserScrollNotification(
      metrics: scroll.position,
      context: context,
      direction: ScrollDirection.reverse,
    ).dispatch(context);
    await tester.pump();
    expect(tester.getTopLeft(tile), const Offset(0, 180));
    expect(tester.binding.transientCallbackCount, 0);
    expect(tester.takeException(), isNull);
    await tester.pumpWidget(const SizedBox.shrink());
  });

  testWidgets("reduced motion reaches final geometry without a ticker", (
    tester,
  ) async {
    final width = ValueNotifier(600.0);
    final scroll = ScrollController();
    addTearDown(width.dispose);
    addTearDown(scroll.dispose);
    await tester.pumpWidget(_fixture(width, scroll, reducedMotion: true));
    await tester.pumpAndSettle();
    width.value = 360;
    await tester.pump();
    expect(
      tester.getTopLeft(find.byKey(const Key("photo-2"))),
      const Offset(0, 180),
    );
    expect(tester.binding.transientCallbackCount, 0);
    await tester.pumpWidget(const SizedBox.shrink());
  });
}

typedef _DuplicateView = ({
  LibraryGalleryQuery query,
  List<int> ids,
  int revision,
});

Widget _duplicateFixture(
  ValueNotifier<_DuplicateView> publication,
  ScrollController scroll,
  List<int> taps,
) => MaterialApp(
  home: Align(
    alignment: Alignment.topLeft,
    child: ValueListenableBuilder(
      valueListenable: publication,
      builder: (context, value, _) => SizedBox(
        width: 300,
        height: 400,
        child: LibraryGalleryLayoutMotion(
          queryIdentity: value.revision,
          geometryIdentity: 300,
          scrollController: scroll,
          duplicatePublication: LibraryGalleryMotionPublication(
            query: value.query,
            layoutIdentity: "grid",
            displayedItems: value.ids.length,
            sourceItems: 6,
          ),
          child: ListView(
            controller: scroll,
            children: [
              for (var start = 0; start < value.ids.length; start += 3)
                SizedBox(
                  key: ValueKey(start),
                  height: 100,
                  child: Row(
                    children: [
                      for (
                        var column = 0;
                        column < 3 && start + column < value.ids.length;
                        column++
                      )
                        LibraryGalleryMotionItem(
                          key: ValueKey(value.ids[start + column]),
                          identity: value.ids[start + column],
                          retainExit: true,
                          rect: Rect.fromLTWH(
                            column * 100,
                            (start ~/ 3) * 100,
                            100,
                            100,
                          ),
                          builder: (_, size) => SizedBox.fromSize(
                            key: Key(
                              "merge-photo-${value.ids[start + column]}",
                            ),
                            size: size,
                            child: GestureDetector(
                              onTap: () => taps.add(value.ids[start + column]),
                              child: const ColoredBox(color: Colors.blue),
                            ),
                          ),
                        ),
                    ],
                  ),
                ),
              const SizedBox(height: 600),
            ],
          ),
        ),
      ),
    ),
  ),
);

Widget _fixture(
  ValueNotifier<double> width,
  ScrollController scroll, {
  bool reducedMotion = false,
  String query = "query",
}) => MaterialApp(
  home: MediaQuery(
    data: MediaQueryData(disableAnimations: reducedMotion),
    child: Align(
      alignment: Alignment.topLeft,
      child: ValueListenableBuilder(
        valueListenable: width,
        builder: (context, value, _) {
          final columns = value == 600 ? 3 : 2;
          final cellWidth = value / columns;
          return SizedBox(
            width: value,
            height: 500,
            child: LibraryGalleryLayoutMotion(
              queryIdentity: query,
              geometryIdentity: value,
              scrollController: scroll,
              child: ListView(
                controller: scroll,
                clipBehavior: Clip.none,
                children: [
                  for (var start = 0; start < 6; start += columns)
                    SizedBox(
                      key: ValueKey("row-$start-$columns"),
                      height: cellWidth,
                      child: Row(
                        children: [
                          for (var column = 0; column < columns; column++)
                            LibraryGalleryMotionItem(
                              identity: start + column,
                              rect: Rect.fromLTWH(
                                column * cellWidth,
                                (start ~/ columns) * cellWidth,
                                cellWidth,
                                cellWidth,
                              ),
                              builder: (context, size) => SizedBox.fromSize(
                                key: Key("photo-${start + column}"),
                                size: size,
                                child: const ColoredBox(color: Colors.blue),
                              ),
                            ),
                        ],
                      ),
                    ),
                ],
              ),
            ),
          );
        },
      ),
    ),
  ),
);
