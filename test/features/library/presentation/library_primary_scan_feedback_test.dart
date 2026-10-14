import "dart:async";

import "package:cedarflake_ame/features/library/application/library_scan_control.dart";
import "package:cedarflake_ame/features/library/domain/library_models.dart";
import "package:cedarflake_ame/features/library/domain/library_state.dart";
import "package:cedarflake_ame/features/library/presentation/library_pending_source.dart";
import "package:cedarflake_ame/features/library/presentation/library_strings.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_task_surface.dart";
import "package:flutter/material.dart";
import "package:flutter_test/flutter_test.dart";

import "../support/primary_scan_control_fixture.dart";
import "../support/retained_scan_fixture.dart";

void main() {
  for (final publication in [
    LibraryScanPublication.confirmationPending,
    LibraryScanPublication.reloadPending,
  ]) {
    testWidgets(
      "result recovery labels follow the retry lifecycle: $publication",
      (tester) async {
        var retries = 0;
        final taskState = ValueNotifier(
          LibraryState(
            primaryScan: LibraryPrimaryScanSnapshot(
              status: LibraryStatus.failed,
              taskKind: LibraryTaskKind.import,
              scanId: "parent-import",
              displayRootPath: r"C:\Fixture\Pictures",
              publication: publication,
              errorMessage:
                  publication == LibraryScanPublication.confirmationPending
                  ? "scan_terminal_reconciliation_failed: catalog unavailable"
                  : "显示暂时不可用",
            ),
          ),
        );
        addTearDown(taskState.dispose);
        await tester.pumpWidget(
          MaterialApp(
            home: Scaffold(
              body: ValueListenableBuilder<LibraryState>(
                valueListenable: taskState,
                builder: (context, state, child) => LibraryTaskSurface(
                  state: state,
                  onPause: () {},
                  onCancel: () {},
                  onResume: () async {},
                  onRetry: () async {
                    retries += 1;
                    taskState.value = state.copyWith(
                      primaryScan: state.primaryScanSnapshot.copyWith(
                        status: LibraryStatus.refreshing,
                        errorMessage: null,
                      ),
                    );
                  },
                  onDismiss: () {},
                ),
              ),
            ),
          ),
        );
        expect(
          find.text(
            publication == LibraryScanPublication.confirmationPending
                ? "暂时无法读取扫描结果"
                : "图库已更新，刷新显示失败",
          ),
          findsOneWidget,
        );
        expect(
          find.byKey(const Key("library-task-dismiss-button")),
          findsNothing,
        );
        expect(find.byType(LinearProgressIndicator), findsNothing);
        expect(
          LibraryPendingSource.addFolderDisabledReason(
            taskState.value.primaryScanSnapshot,
          ),
          publication == LibraryScanPublication.confirmationPending
              ? LibraryStrings.scanConfirmationDetail
              : LibraryStrings.galleryDisplayReloadDetail,
        );
        expect(
          find.text(
            publication == LibraryScanPublication.confirmationPending
                ? "重新读取"
                : "重新加载",
          ),
          findsOneWidget,
        );
        expect(
          LibraryPendingSource.fromState(taskState.value)?.statusLabel,
          publication == LibraryScanPublication.confirmationPending
              ? "暂时无法读取扫描结果"
              : LibraryStrings.sourceDisplayReloadPending,
        );
        await tester.tap(find.byKey(const Key("library-retry-button")));
        await tester.pump();
        expect(retries, 1);
        expect(
          find.text(
            publication == LibraryScanPublication.confirmationPending
                ? "正在读取扫描结果…"
                : "正在刷新图库显示…",
          ),
          findsOneWidget,
        );
        expect(find.text("暂时无法读取扫描结果"), findsNothing);
        expect(find.text("图库已更新，刷新显示失败"), findsNothing);
        expect(find.byKey(const Key("library-retry-button")), findsNothing);
        expect(find.byType(LinearProgressIndicator), findsOneWidget);
        expect(
          LibraryPendingSource.addFolderDisabledReason(
            taskState.value.primaryScanSnapshot,
          ),
          publication == LibraryScanPublication.confirmationPending
              ? LibraryStrings.readingScanResult
              : "正在加载图库…",
        );
        expect(
          LibraryPendingSource.fromState(taskState.value)?.statusLabel,
          publication == LibraryScanPublication.confirmationPending
              ? "正在读取扫描结果…"
              : LibraryStrings.refreshingSourceDisplay,
        );
        taskState.value = taskState.value.copyWith(
          primaryScan: taskState.value.primaryScanSnapshot.copyWith(
            status: LibraryStatus.completed,
            publication: LibraryScanPublication.visible,
          ),
        );
        await tester.pump();
        expect(find.text("导入完成"), findsOneWidget);
        expect(
          LibraryPendingSource.addFolderDisabledReason(
            taskState.value.primaryScanSnapshot,
          ),
          isNull,
        );
        expect(find.byType(LinearProgressIndicator), findsNothing);
        expect(
          LibraryPendingSource.fromState(taskState.value)?.statusLabel,
          LibraryStrings.sourceAdded,
        );
        expect(retries, 1);
        expect(tester.takeException(), isNull);
      },
    );
  }

  testWidgets(
    "pending pause keeps cancellation available before registration",
    (tester) async {
      final fixture = PrimaryScanControlFixture();
      final controller = fixture.controller;
      await tester.pump();
      final start = controller.scanDirectory(r"C:\Fixture\New import");
      await tester.pump();
      await start;
      final id = fixture.state.scanId!;
      final baselineRoots = fixture.state.roots;
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
                onResume: controller.resumePausedScan,
                onRetry: controller.retry,
                onDismiss: () {},
              ),
            ),
          ),
        ),
      );
      await tester.tap(find.byKey(const Key("library-pause-button")));
      taskState.value = fixture.state;
      await tester.pump();
      expect(find.text("正在暂停…"), findsOneWidget);
      final cancel = find.byKey(const Key("library-cancel-button"));
      expect(cancel, findsOneWidget);
      expect(find.byKey(const Key("library-pause-button")), findsNothing);
      await tester.tap(cancel);
      taskState.value = fixture.state;
      await tester.pump();
      expect(find.text("正在取消…"), findsOneWidget);
      expect(cancel, findsNothing);
      fixture.scanner.started(id);
      expect(fixture.state.status, LibraryStatus.cancelling);
      expect(fixture.scanner.commands, [
        LibraryScanCommand.pause,
        LibraryScanCommand.cancel,
        LibraryScanCommand.cancel,
      ]);
      fixture.scanner.emit(
        id,
        const LibraryScanCancelled(acceptedItems: 0, issueCount: 0),
      );
      final closed = fixture.scanner.close(id);
      await tester.pump();
      await closed;
      expect(fixture.state.status, LibraryStatus.cancelled);
      expect(fixture.state.roots, baselineRoots);
    },
  );

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
    expect(find.text("正在停止当前任务。"), findsOneWidget);
    expect(find.text(originalError), findsNothing);
    expect(find.byType(LinearProgressIndicator), findsOneWidget);
    expect(find.byKey(const Key("library-retry-button")), findsNothing);
    final closed = fixture.scanner.close(id);
    await tester.pump();
    await closed;
    taskState.value = fixture.state;
    await tester.pump();
    expect(fixture.state.errorMessage, originalError);
    expect(find.text(originalError), findsNothing);
    expect(find.text("正在停止当前任务。"), findsNothing);
    expect(find.text("添加文件夹失败"), findsOneWidget);
    expect(find.byType(LinearProgressIndicator), findsNothing);
    final retry = find.byKey(const Key("library-retry-button"));
    expect(retry, findsOneWidget);
    expect(tester.widget<TextButton>(retry).onPressed, isNotNull);
  });
}
