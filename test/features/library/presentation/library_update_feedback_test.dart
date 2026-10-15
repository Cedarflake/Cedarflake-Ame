import "package:cedarflake_ame/features/library/application/library_update_controller.dart";
import "package:cedarflake_ame/features/library/domain/library_models.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_update_task_surface.dart";
import "package:flutter/material.dart";
import "package:flutter_test/flutter_test.dart";

void main() {
  testWidgets("task feedback follows queue admission and active cancellation", (
    tester,
  ) async {
    final task = ValueNotifier(_task(LibraryRootUpdatePhase.queued));
    addTearDown(task.dispose);
    final cancelled = <String>[];
    await _pumpTask(
      tester,
      task,
      onCancel: (rootId) {
        cancelled.add(rootId);
        task.value = task.value.copyWith(
          phase: LibraryRootUpdatePhase.cancelling,
        );
      },
    );

    expect(find.text("图库更新"), findsOneWidget);
    expect(find.text("Pictures"), findsOneWidget);
    expect(find.text("等待更新"), findsOneWidget);
    expect(find.text("正在更新"), findsNothing);
    task.value = task.value.copyWith(phase: LibraryRootUpdatePhase.discovering);
    await tester.pump();
    expect(find.text("正在更新"), findsOneWidget);
    expect(find.text("等待更新"), findsNothing);
    await tester.tap(find.text("取消"));
    await tester.pump();

    expect(cancelled, ["root-a"]);
    expect(find.text("正在停止"), findsOneWidget);
    expect(find.text("取消"), findsNothing);
    expect(find.byType(LinearProgressIndicator), findsOneWidget);
    task.value = task.value.copyWith(phase: LibraryRootUpdatePhase.cancelled);
    await tester.pump();

    expect(find.text("已取消"), findsOneWidget);
    expect(find.text("正在停止"), findsNothing);
    expect(find.byType(LinearProgressIndicator), findsNothing);
    expect(cancelled, ["root-a"]);
  });

  testWidgets("refresh retry announces loading and clears its old failure", (
    tester,
  ) async {
    final task = ValueNotifier(
      _task(
        LibraryRootUpdatePhase.refreshFailed,
      ).copyWith(errorMessage: "图库已更新，但刷新显示失败：Bad state: catalog busy"),
    );
    addTearDown(task.dispose);
    final retried = <String>[];
    await _pumpTask(
      tester,
      task,
      onRetry: (rootId) {
        retried.add(rootId);
        task.value = task.value.copyWith(
          phase: LibraryRootUpdatePhase.refreshing,
          errorMessage: null,
        );
      },
    );

    expect(find.text("图片加载失败"), findsOneWidget);
    expect(find.text("扫描已完成。"), findsOneWidget);
    expect(find.textContaining("catalog busy"), findsNothing);
    expect(task.value.errorMessage, contains("catalog busy"));
    expect(find.bySemanticsLabel("重新加载 Pictures"), findsOneWidget);
    expect(find.bySemanticsLabel("重试更新 Pictures"), findsNothing);
    await tester.tap(find.text("重新加载"));
    await tester.pump();

    expect(retried, ["root-a"]);
    expect(find.text("正在加载图片"), findsOneWidget);
    expect(find.textContaining("失败"), findsNothing);
    expect(find.text("重新加载"), findsNothing);
    expect(find.text("取消"), findsNothing);
    expect(find.byType(LinearProgressIndicator), findsOneWidget);

    task.value = task.value.copyWith(phase: LibraryRootUpdatePhase.completed);
    await tester.pump();
    expect(find.text("更新完成"), findsOneWidget);
    expect(find.byType(LinearProgressIndicator), findsNothing);
    expect(find.bySemanticsLabel("关闭 Pictures 更新任务"), findsOneWidget);
    expect(retried, ["root-a"]);
  });

  testWidgets("reload failure explains a root that became unavailable", (
    tester,
  ) async {
    final task = ValueNotifier(_task(LibraryRootUpdatePhase.refreshFailed));
    addTearDown(task.dispose);
    var retries = 0;
    await _pumpTask(
      tester,
      task,
      onRetry: (_) {
        retries += 1;
        task.value = task.value.copyWith(
          errorMessage: "library_update_root_changed: source unavailable",
        );
      },
    );

    await tester.tap(find.text("重新加载"));
    await tester.pump();

    expect(retries, 1);
    expect(find.text("无法更新这个目录，请检查它能否打开。"), findsOneWidget);
    expect(find.text("扫描已完成。"), findsNothing);
    expect(find.textContaining("source unavailable"), findsNothing);
    expect(task.value.errorMessage, contains("source unavailable"));
    expect(find.byType(LinearProgressIndicator), findsNothing);
  });
}

Future<void> _pumpTask(
  WidgetTester tester,
  ValueNotifier<LibraryRootUpdateTask> task, {
  ValueChanged<String>? onCancel,
  ValueChanged<String>? onRetry,
}) => tester.pumpWidget(
  MaterialApp(
    home: Scaffold(
      body: Center(
        child: ValueListenableBuilder<LibraryRootUpdateTask>(
          valueListenable: task,
          builder: (context, current, _) => LibraryUpdateTaskSurface(
            state: LibraryUpdateState(
              tasksByRootId: {current.root.id: current},
            ),
            onCancel: onCancel ?? (_) {},
            onRetry: onRetry ?? (_) {},
            onDismiss: (_) {},
            onDismissTerminalTasks: () {},
          ),
        ),
      ),
    ),
  ),
);

LibraryRootUpdateTask _task(LibraryRootUpdatePhase phase) =>
    LibraryRootUpdateTask(
      root: const LibraryRoot(
        id: "root-a",
        path: r"C:\Pictures",
        displayPath: r"C:\Pictures",
        activeScanId: "published-root-a",
        createdUnixMs: 1,
        assetCount: 2,
        issueCount: 0,
        availability: LibraryRootAvailability.available,
      ),
      scanId: "update-root-a",
      phase: phase,
      visitedEntries: 2,
      acceptedItems: 2,
    );
