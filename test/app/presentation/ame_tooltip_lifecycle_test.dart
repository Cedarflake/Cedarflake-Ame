import "package:cedarflake_ame/app/presentation/ame_overlay_semantics.dart";
import "package:flutter/foundation.dart";
import "package:flutter/gestures.dart";
import "package:flutter/material.dart";
import "package:flutter_test/flutter_test.dart";

void main() {
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
                  message: value ? "收起文件夹" : "展开文件夹",
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
      expect(find.text("展开文件夹"), findsOneWidget);

      await tester.tap(find.byKey(const Key("folder-toggle")));
      await tester.pump();
      expect(tester.takeException(), isNull);
      await tester.pump();
      expect(find.text("收起文件夹"), findsOneWidget);
      expect(find.text("展开文件夹"), findsNothing);

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
