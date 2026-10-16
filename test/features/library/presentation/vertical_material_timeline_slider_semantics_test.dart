import "package:cedarflake_ame/features/library/presentation/widgets/vertical_material_timeline_slider.dart";
import "package:flutter/gestures.dart";
import "package:flutter/material.dart";
import "package:flutter/semantics.dart";
import "package:flutter_test/flutter_test.dart";

import "../../../support/semantics/retained_semantics_update_harness.dart";

void main() {
  RetainedSemanticsUpdateBinding();
  setUp(RetainedSemanticsUpdateValidator.instance.reset);

  testWidgets(
    "timeline keeps its portal parent through input and geometry changes",
    (tester) async {
      final semantics = tester.ensureSemantics();
      final mouse = await tester.createGesture(kind: PointerDeviceKind.mouse);
      await mouse.addPointer(location: const Offset(10, 10));
      var value = 0.8;
      var enabled = true;
      var generation = 0;
      late StateSetter update;
      try {
        await tester.pumpWidget(
          MaterialApp(
            home: Scaffold(
              body: StatefulBuilder(
                builder: (context, setState) {
                  update = setState;
                  return Align(
                    alignment: Alignment.centerRight,
                    child: SizedBox(
                      width: 48,
                      child: VerticalMaterialTimelineSlider(
                        key: ValueKey(generation),
                        value: value,
                        endpointInset: 24,
                        onChanged: enabled
                            ? (next) => setState(() => value = next)
                            : null,
                        onChangeStart: enabled ? (_) {} : null,
                        onChangeEnd: enabled ? (_) {} : null,
                        semanticLabelFor: (value) =>
                            "Month ${value.toStringAsFixed(2)}",
                      ),
                    ),
                  );
                },
              ),
            ),
          ),
        );
        await tester.pumpAndSettle();
        for (var cycle = 0; cycle < 3; cycle++) {
          await mouse.moveTo(tester.getCenter(find.byType(Slider)));
          await mouse.down(tester.getCenter(find.byType(Slider)));
          await tester.pump();
          update(() {
            enabled = false;
            generation++;
          });
          await tester.pump();
          update(() {
            enabled = true;
            value = 0.2;
          });
          await tester.pump();
          await mouse.up();
          await tester.pumpAndSettle();
          await tester.binding.setSurfaceSize(const Size(1280, 800));
          await tester.pump();
          await tester.binding.setSurfaceSize(const Size(800, 600));
          await tester.pump();
          final sliderNode = tester.semantics
              .simulatedAccessibilityTraversal()
              .singleWhere((node) => node.flagsCollection.isSlider);
          final data = sliderNode.getSemanticsData();
          expect(data.value, "Month 0.20");
          expect(data.hasAction(SemanticsAction.increase), isTrue);
          expect(data.hasAction(SemanticsAction.decrease), isTrue);
          final previousValue = value;
          final owner = sliderNode.owner;
          if (owner == null) {
            throw StateError("Timeline slider is detached from semantics");
          }
          owner.performAction(sliderNode.id, SemanticsAction.increase);
          await tester.pump();
          expect(value, lessThan(previousValue));
          RetainedSemanticsUpdateValidator.instance.verifyLatestUpdate(
            trace: "timeline-$cycle",
          );
          expect(tester.takeException(), isNull);
        }
        await tester.pumpWidget(const SizedBox.shrink());
      } finally {
        await mouse.removePointer();
        semantics.dispose();
        await tester.binding.setSurfaceSize(null);
      }
    },
    variant: TargetPlatformVariant.only(TargetPlatform.windows),
  );
}
