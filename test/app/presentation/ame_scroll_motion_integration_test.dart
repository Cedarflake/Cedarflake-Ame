import "package:cedarflake_ame/app/presentation/ame_menu.dart";
import "package:cedarflake_ame/app/presentation/ame_scroll_motion.dart";
import "package:cedarflake_ame/app/presentation/ame_scroll_motion_theme.dart";
import "package:flutter/gestures.dart";
import "package:flutter/material.dart";
import "package:flutter/services.dart";
import "package:flutter_test/flutter_test.dart";

void main() {
  testWidgets("implicit desktop controllers stay independent across panes", (
    tester,
  ) async {
    await tester.pumpWidget(
      MaterialApp(
        theme: ThemeData(platform: TargetPlatform.windows),
        home: Scaffold(
          body: Row(
            children: [
              Expanded(child: _list("left")),
              Expanded(child: _list("right")),
            ],
          ),
        ),
      ),
    );
    final left = _position(tester, "left");
    final right = _position(tester, "right");
    await _wheel(tester, "left", const Offset(0, 120));
    expect(left.pixels, 96);
    expect(right.pixels, 0);
    await _settle(tester);
    expect(left.pixels, 120);
    await _wheel(tester, "right", const Offset(0, 80));
    expect(right.pixels, 60);
    await _settle(tester);
    expect(right.pixels, 80);
    expect(left.pixels, 120);
  });

  for (final reverse in [false, true]) {
    testWidgets(
      "horizontal wheel routing preserves direction, reverse=$reverse",
      (tester) async {
        await tester.pumpWidget(
          MaterialApp(
            home: Scaffold(
              body: _list("strip", axis: Axis.horizontal, reverse: reverse),
            ),
          ),
        );
        final position = _position(tester, "strip");
        position.jumpTo(500);
        await _wheel(tester, "strip", const Offset(0, 120));
        expect(position.pixels, reverse ? 404 : 596);
        await _settle(tester);
        expect(position.pixels, reverse ? 380 : 620);

        await _wheel(tester, "strip", const Offset(100, 0));
        await _settle(tester);
        expect(position.pixels, reverse ? 280 : 720);

        await tester.sendKeyDownEvent(LogicalKeyboardKey.shiftLeft);
        await _wheel(tester, "strip", const Offset(0, 80));
        await tester.sendKeyUpEvent(LogicalKeyboardKey.shiftLeft);
        await _settle(tester);
        expect(position.pixels, reverse ? 200 : 800);
      },
    );
  }

  for (final innerAxis in [Axis.vertical, Axis.horizontal]) {
    testWidgets("nested $innerAxis consumes once and hands off at the edge", (
      tester,
    ) async {
      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(
            body: AmeScrollMotion(
              child: ListView(
                key: const Key("outer"),
                children: [
                  SizedBox(height: 200, child: _list("inner", axis: innerAxis)),
                  const SizedBox(height: 3000),
                ],
              ),
            ),
          ),
        ),
      );
      final scrollables = tester
          .stateList<ScrollableState>(find.byType(Scrollable))
          .toList();
      final outer = scrollables.first.position;
      final inner = _position(tester, "inner");
      await _wheel(tester, "inner", const Offset(0, 120));
      await _settle(tester);
      expect(inner.pixels, 120);
      expect(outer.pixels, 0);
      inner.jumpTo(inner.maxScrollExtent);
      await _wheel(tester, "inner", const Offset(0, 120));
      expect(outer.pixels, 96);
      await _settle(tester);
      expect(outer.pixels, 120);
      expect(inner.pixels, inner.maxScrollExtent);
    });
  }

  testWidgets(
    "a captured Material popup owns its motion and retires it on dismissal",
    (tester) async {
      var selected = -1;
      await tester.pumpWidget(
        MaterialApp(
          theme: ThemeData(platform: TargetPlatform.windows),
          home: AmeScrollMotionTheme(
            child: Scaffold(
              body: Builder(
                builder: (context) => TextButton(
                  onPressed: () async {
                    final result = await showAmePopupMenu<int>(
                      context: context,
                      position: const RelativeRect.fromLTRB(100, 100, 500, 300),
                      maximumHeight: 200,
                      labels: [for (var i = 0; i < 20; i++) "Item $i"],
                      items: [
                        for (var i = 0; i < 20; i++)
                          PopupMenuItem(value: i, child: Text("Item $i")),
                      ],
                    );
                    if (result != null) selected = result;
                  },
                  child: const Text("Open menu"),
                ),
              ),
            ),
          ),
        ),
      );
      await tester.tap(find.text("Open menu"));
      await tester.pumpAndSettle();
      final menu = find.byType(SingleChildScrollView);
      final scrollable = tester.state<ScrollableState>(find.byType(Scrollable));
      await tester.sendEventToBinding(
        PointerScrollEvent(
          kind: PointerDeviceKind.mouse,
          position: tester.getCenter(menu),
          scrollDelta: const Offset(0, 120),
        ),
      );
      expect(scrollable.position.pixels, 96);
      await tester.sendKeyEvent(LogicalKeyboardKey.escape);
      await tester.pumpAndSettle();
      expect(menu, findsNothing);
      expect(selected, -1);
      expect(tester.takeException(), isNull);
      await tester.tap(find.text("Open menu"));
      await tester.pumpAndSettle();
      expect(
        tester.state<ScrollableState>(find.byType(Scrollable)).position.pixels,
        0,
      );
      await tester.tap(find.text("Item 1"));
      await tester.pumpAndSettle();
      expect(selected, 1);
    },
  );
}

Widget _list(String key, {Axis axis = Axis.vertical, bool reverse = false}) =>
    AmeScrollMotion(
      scrollDirection: axis,
      child: ListView.builder(
        key: Key(key),
        scrollDirection: axis,
        reverse: reverse,
        itemExtent: 100,
        itemCount: 50,
        itemBuilder: (_, index) => Text("Row $index"),
      ),
    );

ScrollPosition _position(WidgetTester tester, String key) => tester
    .state<ScrollableState>(
      find.descendant(
        of: find.byKey(Key(key)),
        matching: find.byType(Scrollable),
      ),
    )
    .position;

Future<void> _wheel(WidgetTester tester, String key, Offset delta) =>
    tester.sendEventToBinding(
      PointerScrollEvent(
        kind: PointerDeviceKind.mouse,
        position: tester.getCenter(find.byKey(Key(key))),
        scrollDelta: delta,
      ),
    );

Future<void> _settle(WidgetTester tester) async {
  await tester.pump();
  await tester.pump(const Duration(milliseconds: 100));
}
