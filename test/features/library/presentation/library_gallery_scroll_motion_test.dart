import "package:cedarflake_ame/features/library/presentation/widgets/library_gallery_scroll_motion.dart";
import "package:flutter/gestures.dart";
import "package:flutter/material.dart";
import "package:flutter_test/flutter_test.dart";

void main() {
  testWidgets(
    "wheel input moves immediately and settles at its exact distance",
    (tester) async {
      final controller = await _mount(tester);
      await _wheel(tester, 120);
      expect(controller.offset, 96);
      expect(controller.position.isScrollingNotifier.value, isTrue);
      await tester.pump();
      await tester.pump(const Duration(milliseconds: 45));
      expect(controller.offset, greaterThan(96));
      expect(controller.offset, lessThan(120));
      await tester.pump(const Duration(milliseconds: 55));
      expect(controller.offset, closeTo(120, 0.001));
      expect(controller.position.isScrollingNotifier.value, isFalse);
      await tester.pump(const Duration(seconds: 1));
      expect(controller.offset, closeTo(120, 0.001));
    },
  );

  testWidgets(
    "a wheel burst keeps its distance with a bounded remaining tail",
    (tester) async {
      final controller = await _mount(tester);
      for (var step = 1; step <= 12; step += 1) {
        await _wheel(tester, 120);
        expect(controller.offset, closeTo(step * 120 - 24, 0.001));
      }
      await _finishTail(tester);
      expect(controller.offset, closeTo(1440, 0.001));
      expect(controller.position.isScrollingNotifier.value, isFalse);
    },
  );

  testWidgets("reversal immediately retires the previous direction", (
    tester,
  ) async {
    final controller = await _mount(tester);
    controller.jumpTo(500);
    await _wheel(tester, 120);
    await tester.pump();
    await tester.pump(const Duration(milliseconds: 30));
    final beforeReverse = controller.offset;
    await _wheel(tester, -120);
    expect(controller.offset, closeTo(beforeReverse - 96, 0.001));
    await _finishTail(tester);
    expect(controller.offset, closeTo(beforeReverse - 120, 0.001));
  });

  testWidgets("a direct rail position cannot be overwritten by an old tail", (
    tester,
  ) async {
    final controller = await _mount(tester);
    await _wheel(tester, 120);
    controller.jumpTo(427);
    expect(controller.offset, 427);
    await _finishTail(tester);
    expect(controller.offset, 427);
    await _wheel(tester, 120);
    await _finishTail(tester);
    expect(controller.offset, closeTo(547, 0.001));
  });

  testWidgets("photos remain clickable and pointer down stops the tail", (
    tester,
  ) async {
    var taps = 0;
    final controller = await _mount(tester, onTap: () => taps += 1);
    await _wheel(tester, 120);
    final stoppedAt = controller.offset;
    await tester.tapAt(tester.getCenter(find.byType(ListView)));
    expect(taps, 1);
    expect(controller.position.isScrollingNotifier.value, isFalse);
    await _finishTail(tester);
    expect(controller.offset, stoppedAt);
  });

  for (final scenario in [
    (name: "reduced motion", reduced: true, kind: PointerDeviceKind.mouse),
    (name: "trackpad signal", reduced: false, kind: PointerDeviceKind.trackpad),
  ]) {
    testWidgets("${scenario.name} retains immediate native scrolling", (
      tester,
    ) async {
      final controller = await _mount(tester, reducedMotion: scenario.reduced);
      await _wheel(tester, 120, kind: scenario.kind);
      expect(controller.offset, 120);
      expect(controller.position.isScrollingNotifier.value, isFalse);
      await _finishTail(tester);
      expect(controller.offset, 120);
    });
  }

  testWidgets("platform inertia cancellation stops a pending tail", (
    tester,
  ) async {
    final controller = await _mount(tester);
    await _wheel(tester, 120);
    final stoppedAt = controller.offset;
    await tester.sendEventToBinding(
      PointerScrollInertiaCancelEvent(
        position: tester.getCenter(find.byType(ListView)),
      ),
    );
    await _finishTail(tester);
    expect(controller.offset, stoppedAt);
    expect(controller.position.isScrollingNotifier.value, isFalse);
  });

  testWidgets(
    "precision trackpad movement matches the native scroll position",
    (tester) async {
      final trajectories = <List<double>>[];
      for (final controller in [
        ScrollController(),
        LibraryGalleryScrollController(),
      ]) {
        await _mount(tester, scrollController: controller);
        controller.jumpTo(500);
        final center = tester.getCenter(find.byType(ListView));
        final gesture = await tester.createGesture(
          kind: PointerDeviceKind.trackpad,
        );
        final samples = <double>[];
        await gesture.panZoomStart(center);
        for (var step = 1; step <= 5; step += 1) {
          await gesture.panZoomUpdate(
            center,
            pan: Offset(0, -30.0 * step),
            timeStamp: Duration(milliseconds: step * 16),
          );
          await tester.pump(const Duration(milliseconds: 16));
          samples.add(controller.offset);
        }
        await gesture.panZoomEnd(timeStamp: const Duration(milliseconds: 96));
        await tester.pump(const Duration(milliseconds: 100));
        samples.add(controller.offset);
        expect(samples.last, greaterThan(500));
        trajectories.add(samples);
        await tester.pumpWidget(const SizedBox.shrink());
      }
      expect(trajectories.last, orderedEquals(trajectories.first));
    },
  );

  testWidgets("wheel motion stops at either gallery boundary", (tester) async {
    final controller = await _mount(tester);
    await _wheel(tester, -120);
    expect(controller.offset, 0);
    final maximum = controller.position.maxScrollExtent;
    controller.jumpTo(maximum - 10);
    await _wheel(tester, 120);
    expect(controller.offset, inInclusiveRange(maximum - 10, maximum));
    await _finishTail(tester);
    expect(controller.offset, closeTo(maximum, 0.001));
    await _wheel(tester, 120);
    await _finishTail(tester);
    expect(controller.offset, closeTo(maximum, 0.001));
  });

  testWidgets("relayout keeps its corrected position after retiring the tail", (
    tester,
  ) async {
    final controller = await _mount(tester);
    await _wheel(tester, 120);
    controller.position.correctBy(200);
    final correctedPosition = controller.offset;
    await tester.pump();
    expect(controller.position.isScrollingNotifier.value, isFalse);
    await _finishTail(tester);
    expect(controller.offset, correctedPosition);
  });

  testWidgets("detaching the gallery retires an unfinished wheel activity", (
    tester,
  ) async {
    final controller = await _mount(tester);
    await _wheel(tester, 120);
    await tester.pumpWidget(const SizedBox.shrink());
    await _finishTail(tester);
    expect(controller.hasClients, isFalse);
    expect(tester.takeException(), isNull);
  });
}

Future<ScrollController> _mount(
  WidgetTester tester, {
  bool reducedMotion = false,
  VoidCallback? onTap,
  ScrollController? scrollController,
}) async {
  final controller = scrollController ?? LibraryGalleryScrollController();
  addTearDown(controller.dispose);
  await tester.pumpWidget(
    MaterialApp(
      home: MediaQuery(
        data: MediaQueryData(disableAnimations: reducedMotion),
        child: Scaffold(
          body: LibraryGalleryScrollMotion(
            controller: controller,
            child: ListView.builder(
              controller: controller,
              itemExtent: 100,
              itemCount: 100,
              itemBuilder: (context, index) => GestureDetector(
                behavior: HitTestBehavior.opaque,
                onTap: onTap,
                child: Text("Photo $index"),
              ),
            ),
          ),
        ),
      ),
    ),
  );
  return controller;
}

Future<void> _wheel(
  WidgetTester tester,
  double delta, {
  PointerDeviceKind kind = PointerDeviceKind.mouse,
}) => tester.sendEventToBinding(
  PointerScrollEvent(
    kind: kind,
    position: tester.getCenter(find.byType(ListView)),
    scrollDelta: Offset(0, delta),
  ),
);

Future<void> _finishTail(WidgetTester tester) async {
  await tester.pump();
  await tester.pump(const Duration(milliseconds: 100));
}
