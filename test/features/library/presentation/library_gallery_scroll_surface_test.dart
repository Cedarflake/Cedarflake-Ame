import "package:cedarflake_ame/features/library/presentation/widgets/annotated_time_rail.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_gallery_scroll_surface.dart";
import "package:flutter/material.dart";
import "package:flutter_test/flutter_test.dart";

void main() {
  testWidgets(
    "position rail follows scroll metrics and retires old query gestures",
    (tester) async {
      await tester.binding.setSurfaceSize(const Size(900, 600));
      addTearDown(() => tester.binding.setSurfaceSize(null));
      final controller = ScrollController();
      final query = ValueNotifier(1);
      addTearDown(controller.dispose);
      addTearDown(query.dispose);
      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(
            body: ValueListenableBuilder(
              valueListenable: query,
              builder: (_, identity, _) => LibraryGalleryScrollSurface(
                controller: controller,
                usePositionRail: true,
                scrollIdentity: identity,
                gallery: ListView.builder(
                  controller: controller,
                  itemExtent: 40,
                  itemCount: 100,
                  itemBuilder: (_, index) => Text("Photo $index"),
                ),
              ),
            ),
          ),
        ),
      );
      await tester.pumpAndSettle();
      AnnotatedTimeRail rail() => tester.widget(find.byType(AnnotatedTimeRail));
      expect(rail().buckets, isEmpty);
      expect(rail().onChanged, isNotNull);
      final slider = tester.widget<Slider>(find.byType(Slider));
      expect(slider.semanticFormatterCallback!(.5), "滚动位置 50%");
      rail().onChanged!(.5);
      await tester.pumpAndSettle();
      expect(controller.offset, controller.position.maxScrollExtent / 2);
      final previousEnd = rail().onChangeEnd!;
      query.value = 2;
      controller.jumpTo(0);
      await tester.pumpAndSettle();
      previousEnd(1);
      expect(controller.offset, 0);
      await tester.tap(find.byKey(const Key("timeline-next")));
      await tester.pumpAndSettle();
      expect(controller.offset, 50);
      await tester.binding.setSurfaceSize(const Size(900, 400));
      await tester.pumpAndSettle();
      expect(rail().maximumScrollOffset, controller.position.maxScrollExtent);
      expect(
        rail().value,
        closeTo(50 / controller.position.maxScrollExtent, .0001),
      );
      rail().onChangeEnd!(1);
      await tester.pumpAndSettle();
      expect(controller.offset, controller.position.maxScrollExtent);
      expect(tester.takeException(), isNull);
    },
  );

  testWidgets("a short organization queue keeps the shared rail disabled", (
    tester,
  ) async {
    final controller = ScrollController();
    addTearDown(controller.dispose);
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: LibraryGalleryScrollSurface(
            controller: controller,
            usePositionRail: true,
            gallery: ListView(
              controller: controller,
              children: const [Text("Photo")],
            ),
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();
    final rail = tester.widget<AnnotatedTimeRail>(
      find.byType(AnnotatedTimeRail),
    );
    expect(rail.value, 0);
    expect(rail.onChanged, isNull);
    expect(rail.onStep, isNull);
    expect(tester.widget<Slider>(find.byType(Slider)).onChanged, isNull);
    expect(find.byKey(const Key("timeline-previous")), findsOneWidget);
    expect(tester.takeException(), isNull);
  });
}
