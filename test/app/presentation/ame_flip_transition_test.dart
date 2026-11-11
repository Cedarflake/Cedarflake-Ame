import "package:cedarflake_ame/app/presentation/ame_flip_transition.dart";
import "package:material_ui/material_ui.dart";
import "package:flutter_test/flutter_test.dart";

void main() {
  testWidgets(
    "flip keeps its target and content through reversal and reduced motion",
    (tester) async {
      var selected = false;
      var reducedMotion = false;
      late StateSetter update;
      const frontKey = ValueKey("front");
      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(
            body: StatefulBuilder(
              builder: (context, setState) {
                update = setState;
                return MediaQuery(
                  data: MediaQueryData(disableAnimations: reducedMotion),
                  child: Center(
                    child: IconButton(
                      isSelected: selected,
                      onPressed: () => setState(() => selected = !selected),
                      icon: SizedBox.square(
                        dimension: 80,
                        child: AmeFlipTransition(
                          showBack: selected,
                          front: const Text("Picture", key: frontKey),
                          back: const Text("Selected"),
                        ),
                      ),
                    ),
                  ),
                );
              },
            ),
          ),
        ),
      );
      final button = find.byType(IconButton);
      final target = tester.getRect(button);
      final front = tester.element(find.byKey(frontKey));
      final transform = find.descendant(
        of: find.byType(AmeFlipTransition),
        matching: find.byType(Transform),
      );
      final original = tester.widget<Transform>(transform).transform.clone();

      await tester.tap(button);
      expect(selected, isTrue);
      await tester.pump();
      await tester.pump(const Duration(milliseconds: 40));
      final halfway = tester.widget<Transform>(transform).transform.clone();
      expect(halfway.entry(0, 0).abs(), inExclusiveRange(0, 1));
      expect(halfway.entry(1, 1), 1);
      expect(tester.getRect(button), target);

      await tester.tap(button);
      expect(selected, isFalse);
      await tester.pump();
      expect(tester.widget<Transform>(transform).transform, halfway);
      await tester.pumpAndSettle();
      expect(find.text("Picture"), findsOneWidget);
      expect(find.text("Selected"), findsNothing);
      expect(tester.element(find.byKey(frontKey)), same(front));

      await tester.tap(button);
      await tester.pumpAndSettle();
      expect(find.text("Selected"), findsOneWidget);
      expect(find.text("Picture"), findsNothing);
      expect(tester.getRect(button), target);

      update(() {
        reducedMotion = true;
        selected = false;
      });
      await tester.pump();
      expect(find.text("Picture"), findsOneWidget);
      expect(tester.element(find.byKey(frontKey)), same(front));
      expect(tester.widget<Transform>(transform).transform, original);
      expect(tester.takeException(), isNull);
    },
  );
}
