import "package:cedarflake_ame/app/presentation/ame_overlay_semantics.dart";
import "package:flutter/foundation.dart";
import "package:flutter/gestures.dart";
import "package:flutter/material.dart";
import "package:flutter_test/flutter_test.dart";

void main() {
  testWidgets(
    "focus retains the accessible name without holding a visual tooltip",
    (tester) async {
      debugDefaultTargetPlatformOverride = TargetPlatform.windows;
      final focus = FocusNode();
      final mouse = await tester.createGesture(kind: PointerDeviceKind.mouse);
      final semantics = tester.ensureSemantics();
      const targetKey = Key("focused-tooltip-target");
      const message = "查看重复位置";
      try {
        await tester.pumpWidget(
          MaterialApp(
            home: Scaffold(
              body: Center(
                child: AmeTooltip(
                  message: message,
                  child: IconButton(
                    key: targetKey,
                    focusNode: focus,
                    autofocus: true,
                    onPressed: () {},
                    icon: const Icon(Icons.photo_library),
                  ),
                ),
              ),
            ),
          ),
        );
        await tester.pumpAndSettle();
        final target = find.byKey(targetKey);
        expect(focus.hasFocus, isTrue);
        expect(find.text(message), findsNothing);
        expect(
          tester
              .getSemantics(find.byType(AmeTooltip))
              .getSemanticsData()
              .tooltip,
          message,
        );

        await mouse.addPointer(location: Offset.zero);
        await mouse.moveTo(tester.getCenter(target));
        await tester.pump(const Duration(milliseconds: 500));
        await tester.pumpAndSettle();
        expect(find.text(message), findsOneWidget);
        await mouse.down(tester.getCenter(target));
        await mouse.up();
        await mouse.moveTo(Offset.zero);
        await tester.pump(const Duration(milliseconds: 100));
        await tester.pumpAndSettle();
        expect(focus.hasFocus, isTrue);
        expect(find.text(message), findsNothing);
        expect(
          tester
              .getSemantics(find.byType(AmeTooltip))
              .getSemanticsData()
              .tooltip,
          message,
        );
        expect(tester.takeException(), isNull);
      } finally {
        await mouse.removePointer();
        await tester.pumpWidget(const SizedBox.shrink());
        focus.dispose();
        semantics.dispose();
        debugDefaultTargetPlatformOverride = null;
      }
    },
  );

  testWidgets("updates a visible folder tooltip during its parent rebuild", (
    tester,
  ) async {
    debugDefaultTargetPlatformOverride = TargetPlatform.windows;
    final expanded = ValueNotifier(false);
    final mouse = await tester.createGesture(kind: PointerDeviceKind.mouse);
    try {
      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(
            body: Center(
              child: ValueListenableBuilder<bool>(
                valueListenable: expanded,
                builder: (context, value, child) => AmeTooltip(
                  message: value ? "收起目录" : "展开目录",
                  waitDuration: Duration.zero,
                  child: IconButton(
                    key: const Key("folder-toggle"),
                    onPressed: () => expanded.value = !expanded.value,
                    icon: Icon(value ? Icons.expand_less : Icons.expand_more),
                  ),
                ),
              ),
            ),
          ),
        ),
      );
      await mouse.addPointer(location: Offset.zero);
      await mouse.moveTo(
        tester.getCenter(find.byKey(const Key("folder-toggle"))),
      );
      await tester.pump();
      await tester.pump(const Duration(milliseconds: 150));
      expect(find.text("展开目录"), findsOneWidget);

      await tester.tap(find.byKey(const Key("folder-toggle")));
      await tester.pump();
      expect(tester.takeException(), isNull);
      await tester.pump();
      expect(find.text("收起目录"), findsOneWidget);
      expect(find.text("展开目录"), findsNothing);

      for (var index = 0; index < 3; index++) {
        await tester.tap(find.byKey(const Key("folder-toggle")));
        await tester.pump();
        await tester.pump();
        await mouse.moveTo(Offset.zero);
        await mouse.moveTo(
          tester.getCenter(find.byKey(const Key("folder-toggle"))),
        );
        expect(tester.takeException(), isNull);
      }
      await tester.pumpWidget(const SizedBox.shrink());
      await tester.pumpAndSettle();
      expect(tester.takeException(), isNull);
    } finally {
      await mouse.removePointer();
      await tester.pumpWidget(const SizedBox.shrink());
      expanded.dispose();
      debugDefaultTargetPlatformOverride = null;
    }
  });
}
