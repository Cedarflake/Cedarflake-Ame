import "package:cedarflake_ame/features/library/presentation/widgets/library_gallery_layout_motion.dart";
import "package:flutter/material.dart";
import "package:flutter/rendering.dart";
import "package:flutter_test/flutter_test.dart";

void main() {
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
    expect(tester.getRect(tile), middle);
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
