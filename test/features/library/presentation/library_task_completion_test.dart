import "package:cedarflake_ame/app/ame_app.dart";
import "package:cedarflake_ame/features/library/application/library_update_controller.dart";
import "package:cedarflake_ame/features/library/domain/library_state.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_task_surface.dart";
import "package:flutter/gestures.dart";
import "package:flutter/material.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:flutter_test/flutter_test.dart";

import "../support/retained_scan_fixture.dart";

void main() {
  testWidgets("completion leaves the gallery after a short acknowledgement", (
    tester,
  ) async {
    final fixture = await _show(tester);
    await _completePrimary(tester, fixture);
    final assets = fixture.state.assets;
    final roots = fixture.state.roots;
    expect(find.text("添加完成"), findsOneWidget);
    await tester.pump(const Duration(seconds: 4));
    expect(find.byType(LibraryTaskSurface), findsOneWidget);
    await tester.pump(const Duration(seconds: 2));
    expect(find.byType(LibraryTaskSurface), findsNothing);
    expect(fixture.state.assets, assets);
    expect(fixture.state.roots, roots);
    expect(fixture.scanner.cancelActiveIds, isEmpty);
  });

  testWidgets("an older completion cannot dismiss a newer task result", (
    tester,
  ) async {
    final fixture = await _show(tester);
    await _completePrimary(tester, fixture);
    final oldScanId = fixture.state.scanId;
    await tester.pump(const Duration(seconds: 4));
    await fixture.controller.scanDirectory(retainedScanRoot.path);
    final newScanId = fixture.state.scanId!;
    expect(newScanId, isNot(oldScanId));
    fixture.scanner.completeScan(newScanId);
    await tester.pump();
    await tester.pump();
    await tester.pump(const Duration(seconds: 2));
    expect(fixture.state.scanId, newScanId);
    expect(find.byType(LibraryTaskSurface), findsOneWidget);
    await tester.pump(const Duration(seconds: 4));
    expect(find.byType(LibraryTaskSurface), findsNothing);
  });

  testWidgets("paused and failed tasks retain their recovery controls", (
    tester,
  ) async {
    final fixture = await _show(tester);
    await tester.pump(const Duration(seconds: 8));
    expect(find.byKey(const Key("library-resume-button")), findsOneWidget);
    await fixture.controller.cancelScan();
    await fixture.controller.scanDirectory(retainedScanRoot.path);
    fixture.scanner.failUpdates();
    await tester.pump();
    await tester.pump();
    await tester.pump(const Duration(seconds: 8));
    expect(fixture.state.status, LibraryStatus.failed);
    expect(find.byKey(const Key("library-retry-button")), findsOneWidget);
  });

  testWidgets("completed update leaves an independent paused import intact", (
    tester,
  ) async {
    final fixture = await _show(tester);
    fixture.container
        .read(libraryUpdateControllerProvider.notifier)
        .startUpdates([otherPublishedRoot.id]);
    await tester.pump();
    final update = fixture.container
        .read(libraryUpdateControllerProvider)
        .tasksByRootId[otherPublishedRoot.id]!;
    fixture.scanner.completeScan(update.scanId);
    await tester.pump();
    await tester.pump();
    expect(find.text("更新完成"), findsOneWidget);
    await tester.pump(const Duration(seconds: 6));
    expect(find.byKey(const Key("library-update-task-surface")), findsNothing);
    expect(fixture.state.status, LibraryStatus.paused);
    expect(fixture.state.scanId, retainedScanCheckpoint.scanId);
    expect(find.byKey(const Key("library-resume-button")), findsOneWidget);
    expect(fixture.scanner.cancelActiveIds, isEmpty);
    expect(fixture.scanner.cancelRetainedIds, isEmpty);
  });

  testWidgets("hover keeps completion visible until the pointer leaves", (
    tester,
  ) async {
    final fixture = await _show(tester);
    await _completePrimary(tester, fixture);
    final pointer = await tester.createGesture(kind: PointerDeviceKind.mouse);
    await pointer.addPointer();
    await pointer.moveTo(tester.getCenter(find.byType(LibraryTaskSurface)));
    await tester.pump();
    await tester.pump(const Duration(seconds: 8));
    expect(find.byType(LibraryTaskSurface), findsOneWidget);
    await pointer.moveTo(const Offset(10, 10));
    await tester.pump();
    await tester.pump(const Duration(seconds: 6));
    expect(find.byType(LibraryTaskSurface), findsNothing);
    await pointer.removePointer();
  });

  testWidgets("keyboard focus keeps the completion available", (tester) async {
    final fixture = await _show(tester);
    await _completePrimary(tester, fixture);
    final button = find.byKey(const Key("library-task-dismiss-button"));
    Focus.of(
      tester.element(find.descendant(of: button, matching: find.text("关闭"))),
    ).requestFocus();
    await tester.pump();
    await tester.pump(const Duration(seconds: 8));
    expect(button, findsOneWidget);
    await tester.tap(button);
    await tester.pump();
    expect(find.byType(LibraryTaskSurface), findsNothing);
  });

  testWidgets("accessible navigation retains completion until dismissed", (
    tester,
  ) async {
    tester.platformDispatcher.accessibilityFeaturesTestValue =
        const FakeAccessibilityFeatures(accessibleNavigation: true);
    addTearDown(tester.platformDispatcher.clearAccessibilityFeaturesTestValue);
    final fixture = await _show(tester);
    await _completePrimary(tester, fixture);
    await tester.pump(const Duration(seconds: 8));
    expect(find.byType(LibraryTaskSurface), findsOneWidget);
    await tester.tap(find.byKey(const Key("library-task-dismiss-button")));
    await tester.pump();
    expect(find.byType(LibraryTaskSurface), findsNothing);
  });
}

Future<RetainedScanFixture> _show(WidgetTester tester) async {
  tester.view.physicalSize = const Size(1400, 900);
  tester.view.devicePixelRatio = 1;
  addTearDown(tester.view.resetPhysicalSize);
  addTearDown(tester.view.resetDevicePixelRatio);
  final fixture = RetainedScanFixture();
  await tester.pumpWidget(
    UncontrolledProviderScope(
      container: fixture.container,
      child: const AmeApp(),
    ),
  );
  await tester.pump();
  await tester.pump();
  return fixture;
}

Future<void> _completePrimary(
  WidgetTester tester,
  RetainedScanFixture fixture,
) async {
  await fixture.controller.cancelScan();
  await fixture.controller.scanDirectory(retainedScanRoot.path);
  fixture.scanner.completeScan(fixture.state.scanId!);
  await tester.pump();
  await tester.pump();
  expect(fixture.state.status, LibraryStatus.completed);
}
