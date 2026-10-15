import "package:cedarflake_ame/features/library/presentation/library_strings.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_viewer_filmstrip_scrollbar.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_viewer_filmstrip_slot.dart";
import "package:flutter/foundation.dart";
import "package:flutter/gestures.dart";
import "package:flutter/material.dart";
import "package:flutter_test/flutter_test.dart";

void main() {
  testWidgets("Windows strip arrows retain hover labels across hiding", (
    tester,
  ) async {
    debugDefaultTargetPlatformOverride = TargetPlatform.windows;
    final controller = ScrollController();
    final visible = ValueNotifier(true);
    addTearDown(controller.dispose);
    addTearDown(visible.dispose);
    final semantics = tester.ensureSemantics();
    final mouse = await tester.createGesture(kind: PointerDeviceKind.mouse);
    try {
      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(
            body: Center(
              child: SizedBox(
                width: 700,
                child: ValueListenableBuilder(
                  valueListenable: visible,
                  builder: (context, value, _) => LibraryViewerFilmstripSlot(
                    visible: value,
                    child: LibraryViewerFilmstripScrollbar(
                      controller: controller,
                      step: 100,
                      child: ListView.builder(
                        controller: controller,
                        scrollDirection: Axis.horizontal,
                        itemExtent: 100,
                        itemCount: 20,
                        itemBuilder: (_, index) => Text("Image $index"),
                      ),
                    ),
                  ),
                ),
              ),
            ),
          ),
        ),
      );
      await tester.pumpAndSettle();
      await mouse.addPointer(location: Offset.zero);
      for (var cycle = 0; cycle < 2; cycle += 1) {
        for (final direction in ["right", "left"]) {
          final label = direction == "right"
              ? LibraryStrings.scrollFilmstripRight
              : LibraryStrings.scrollFilmstripLeft;
          final arrow = find.byKey(Key("viewer-filmstrip-scroll-$direction"));
          final tooltip = find.byWidgetPredicate(
            (widget) =>
                widget is Semantics && widget.properties.tooltip == label,
          );
          expect(
            tester.getSemantics(tooltip).getSemanticsData().tooltip,
            label,
          );
          // The pinned Windows engine cannot retain Tooltip's portal semantics.
          expect(find.byType(OverlayPortal, skipOffstage: false), findsNothing);
          await mouse.moveTo(tester.getCenter(arrow));
          await tester.pump(const Duration(milliseconds: 700));
          await tester.pumpAndSettle();
          expect(find.text(label), findsOneWidget);
          await tester.tap(arrow);
          await tester.pumpAndSettle();
          expect(controller.offset, direction == "right" ? 100 : 0);
        }
        visible.value = false;
        await tester.pumpAndSettle();
        expect(find.text(LibraryStrings.scrollFilmstripLeft), findsNothing);
        await mouse.moveTo(Offset.zero);
        visible.value = true;
        await tester.pumpAndSettle();
      }
      await mouse.removePointer();
      await tester.pumpWidget(const SizedBox.shrink());
      await tester.pumpAndSettle();
      expect(tester.takeException(), isNull);
    } finally {
      semantics.dispose();
      debugDefaultTargetPlatformOverride = null;
    }
  });
}
