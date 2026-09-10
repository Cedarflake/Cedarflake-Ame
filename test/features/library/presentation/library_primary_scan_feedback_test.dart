import "dart:async";

import "package:cedarflake_ame/features/library/domain/library_state.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_task_surface.dart";
import "package:flutter/material.dart";
import "package:flutter_test/flutter_test.dart";

import "../support/primary_scan_control_fixture.dart";
import "../support/retained_scan_fixture.dart";

void main() {
  testWidgets("protocol failure shows cleanup feedback before offering retry", (
    tester,
  ) async {
    final fixture = PrimaryScanControlFixture();
    final controller = fixture.controller;
    await tester.pump();
    final start = controller.scanDirectory(retainedScanRoot.path);
    await tester.pump();
    await start;
    final id = fixture.state.scanId!;
    fixture.scanner.fail(id);
    final originalError = fixture.state.errorMessage!;
    final taskState = ValueNotifier(fixture.state);
    addTearDown(taskState.dispose);
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: ValueListenableBuilder<LibraryState>(
            valueListenable: taskState,
            builder: (context, state, child) => LibraryTaskSurface(
              state: state,
              onPause: controller.pauseScan,
              onCancel: () => unawaited(controller.cancelScan()),
              onResume: () async {},
              onRetry: controller.retry,
              onDismiss: () {},
            ),
          ),
        ),
      ),
    );
    expect(find.text("正在取消…"), findsOneWidget);
    expect(find.text(originalError), findsOneWidget);
    expect(find.byType(LinearProgressIndicator), findsOneWidget);
    expect(find.byKey(const Key("library-retry-button")), findsNothing);
    final closed = fixture.scanner.close(id);
    await tester.pump();
    await closed;
    taskState.value = fixture.state;
    await tester.pump();
    expect(fixture.state.errorMessage, originalError);
    expect(find.text(originalError), findsOneWidget);
    expect(find.byType(LinearProgressIndicator), findsNothing);
    final retry = find.byKey(const Key("library-retry-button"));
    expect(retry, findsOneWidget);
    expect(tester.widget<TextButton>(retry).onPressed, isNotNull);
  });
}
