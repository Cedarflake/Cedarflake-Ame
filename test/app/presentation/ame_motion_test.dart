import "package:cedarflake_ame/app/presentation/ame_motion.dart";
import "package:material_ui/material_ui.dart";
import "package:flutter_test/flutter_test.dart";

void main() {
  testWidgets("reversal retains velocity and retires at the exact endpoint", (
    tester,
  ) async {
    for (final motion in AmeMotion.values) {
      final controller = AnimationController(vsync: tester);
      motion.animate(controller, forward: true);
      await tester.pump();
      await tester.pump(const Duration(milliseconds: 40));
      final value = controller.value;
      final velocity = controller.velocity;
      expect(value, inExclusiveRange(0, 1));
      expect(velocity, greaterThan(0));

      motion.animate(controller, forward: false);
      expect(controller.value, value);
      expect(controller.velocity, closeTo(velocity, 0.0001));
      await tester.pumpAndSettle();
      expect(controller.value, 0);
      expect(controller.status, AnimationStatus.dismissed);
      expect(controller.isAnimating, isFalse);

      motion.animate(controller, forward: true);
      await tester.pumpAndSettle();
      expect(controller.value, 1);
      expect(controller.status, AnimationStatus.completed);
      controller.dispose();
    }
  });

  testWidgets(
    "reduced motion and inactive surfaces finish an interrupted state",
    (tester) async {
      var active = false;
      var reducedMotion = false;
      var enabled = true;
      late StateSetter update;
      late Animation<double> animation;
      await tester.pumpWidget(
        MaterialApp(
          home: StatefulBuilder(
            builder: (context, setState) {
              update = setState;
              return MediaQuery(
                data: MediaQueryData(disableAnimations: reducedMotion),
                child: TickerMode(
                  enabled: enabled,
                  child: AmeStateTransition(
                    active: active,
                    motion: AmeMotion.fastSpatial,
                    builder: (context, progress, _) {
                      animation = progress;
                      return RotationTransition(
                        turns: progress,
                        child: const Icon(Icons.arrow_downward),
                      );
                    },
                  ),
                ),
              );
            },
          ),
        ),
      );
      expect(animation.value, 0);
      update(() => active = true);
      await tester.pump();
      await tester.pump(const Duration(milliseconds: 40));
      expect(animation.value, inExclusiveRange(0, 1));
      update(() => reducedMotion = true);
      await tester.pump();
      expect(animation.value, 1);
      expect(tester.binding.transientCallbackCount, 0);

      update(() {
        reducedMotion = false;
        active = false;
      });
      await tester.pump();
      await tester.pump(const Duration(milliseconds: 40));
      expect(animation.value, inExclusiveRange(0, 1));
      update(() => enabled = false);
      await tester.pump();
      expect(animation.value, 0);
      await tester.pumpWidget(const SizedBox.shrink());
      expect(tester.binding.transientCallbackCount, 0);
      expect(tester.takeException(), isNull);
    },
  );
}
