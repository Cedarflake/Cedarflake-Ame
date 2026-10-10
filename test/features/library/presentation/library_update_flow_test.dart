import "dart:ui" show SemanticsAction;

import "package:cedarflake_ame/features/library/application/library_update_controller.dart";
import "package:cedarflake_ame/features/library/domain/library_models.dart";
import "package:cedarflake_ame/features/library/domain/library_state.dart";
import "package:cedarflake_ame/features/library/presentation/library_strings.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_task_surface.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_update_dialog.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_update_task_surface.dart";
import "package:flutter/material.dart";
import "package:flutter_test/flutter_test.dart";

void main() {
  testWidgets("selects multiple configured roots in one update action", (
    tester,
  ) async {
    final roots = [
      _root("root-a", "C:\\Pictures"),
      _root("root-b", "D:\\Archive"),
      _root(
        "root-offline",
        "E:\\Offline",
        availability: LibraryRootAvailability.offline,
      ),
    ];
    List<LibraryRoot>? selectedRoots;
    await tester.pumpWidget(
      MaterialApp(
        home: Builder(
          builder: (context) => Scaffold(
            body: TextButton(
              onPressed: () async {
                selectedRoots = await showLibraryUpdateDialog(
                  context: context,
                  roots: roots,
                  activeRootIds: const {},
                  initialRootId: "root-a",
                );
              },
              child: const Text("打开"),
            ),
          ),
        ),
      ),
    );

    await tester.tap(find.text("打开"));
    await tester.pumpAndSettle();

    expect(find.byKey(const Key("library-update-dialog")), findsOneWidget);
    expect(
      tester
          .widget<CheckboxListTile>(
            find.byKey(const ValueKey("library-update-root-root-a")),
          )
          .value,
      isTrue,
    );
    expect(
      tester
          .widget<CheckboxListTile>(
            find.byKey(const ValueKey("library-update-root-root-offline")),
          )
          .enabled,
      isFalse,
    );
    expect(
      tester
          .widget<CheckboxListTile>(
            find.byKey(const ValueKey("library-update-root-root-offline")),
          )
          .onChanged,
      isNull,
    );
    await tester.tap(find.byKey(const ValueKey("library-update-root-root-b")));
    await tester.pump();
    await tester.tap(find.byKey(const Key("library-update-confirm")));
    await tester.pumpAndSettle();

    expect(selectedRoots?.map((root) => root.id), ["root-a", "root-b"]);
  });

  testWidgets("shows update wording and independent root controls", (
    tester,
  ) async {
    final semantics = tester.ensureSemantics();
    try {
      final cancelled = <String>[];
      final retried = <String>[];
      final dismissed = <String>[];
      var didDismissTerminalTasks = false;
      final state = LibraryUpdateState(
        tasksByRootId: {
          "root-a": LibraryRootUpdateTask(
            root: _root("root-a", "C:\\Pictures"),
            scanId: "update-a",
            phase: LibraryRootUpdatePhase.discovering,
            visitedEntries: 230400,
            acceptedItems: 8740,
          ),
          "root-b": LibraryRootUpdateTask(
            root: _root("root-b", "D:\\Archive"),
            scanId: "update-b",
            phase: LibraryRootUpdatePhase.failed,
            errorMessage: "archive_failed: Archive failed",
          ),
          "root-c": LibraryRootUpdateTask(
            root: _root("root-c", "E:\\Reference"),
            scanId: "update-c",
            phase: LibraryRootUpdatePhase.completed,
          ),
        },
      );
      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(
            body: Center(
              child: LibraryUpdateTaskSurface(
                state: state,
                onCancel: cancelled.add,
                onRetry: retried.add,
                onDismiss: dismissed.add,
                onDismissTerminalTasks: () => didDismissTerminalTasks = true,
              ),
            ),
          ),
        ),
      );

      expect(find.text("正在更新图库“Pictures”…"), findsOneWidget);
      expect(find.textContaining("正在添加文件夹"), findsNothing);
      expect(find.text("archive_failed: Archive failed"), findsOneWidget);
      expect(find.bySemanticsLabel("取消更新 Pictures"), findsOneWidget);
      expect(find.bySemanticsLabel("重试更新 Archive"), findsOneWidget);
      expect(find.bySemanticsLabel("清除 Reference 更新结果"), findsOneWidget);
      expect(find.bySemanticsLabel("Pictures 更新进度"), findsOneWidget);
      expect(
        tester.getSemantics(find.bySemanticsLabel("Pictures 更新进度")).value,
        "正在更新，已检查 230400 个文件，已找到 8740 张图片",
      );

      final cancelAction = find.bySemanticsLabel("取消更新 Pictures");
      final retryAction = find.bySemanticsLabel("重试更新 Archive");
      final dismissAction = find.bySemanticsLabel("清除 Reference 更新结果");
      final dismissAllAction = find.bySemanticsLabel("清除结果");
      for (final action in [
        cancelAction,
        retryAction,
        dismissAction,
        dismissAllAction,
      ]) {
        expect(
          tester
              .getSemantics(action)
              .getSemanticsData()
              .hasAction(SemanticsAction.tap),
          isTrue,
        );
      }

      for (final action in [
        cancelAction,
        retryAction,
        dismissAction,
        dismissAllAction,
      ]) {
        final node = tester.getSemantics(action);
        final semanticsOwner = node.owner;
        if (semanticsOwner == null) {
          throw StateError("Action node is detached from its semantics owner");
        }
        semanticsOwner.performAction(node.id, SemanticsAction.tap);
      }
      await tester.pump();

      expect(cancelled, ["root-a"]);
      expect(retried, ["root-b"]);
      expect(dismissed, ["root-c"]);
      expect(didDismissTerminalTasks, isTrue);
    } finally {
      semantics.dispose();
    }
  });

  testWidgets("throttles stable multi-root progress announcements", (
    tester,
  ) async {
    final semantics = tester.ensureSemantics();
    final updateState = ValueNotifier(_multiRootProgressState(validated: 10));
    addTearDown(updateState.dispose);
    try {
      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(
            body: ValueListenableBuilder<LibraryUpdateState>(
              valueListenable: updateState,
              builder: (context, state, child) => LibraryUpdateTaskSurface(
                state: state,
                onCancel: (_) {},
                onRetry: (_) {},
                onDismiss: (_) {},
                onDismissTerminalTasks: () {},
              ),
            ),
          ),
        ),
      );

      final liveRegion = find.byKey(const Key("library-task-live-region"));
      var liveSemantics = tester.getSemantics(liveRegion);
      expect(liveSemantics.flagsCollection.isLiveRegion, isTrue);
      expect(liveSemantics.label, contains("10 / 100 张图片，10%"));
      expect(
        tester.getSemantics(find.bySemanticsLabel("Pictures 更新进度")).value,
        "正在核对，10 / 100 张图片，10%",
      );

      updateState.value = _multiRootProgressState(validated: 35);
      await tester.pump();
      liveSemantics = tester.getSemantics(liveRegion);
      expect(liveSemantics.label, contains("10 / 100 张图片，10%"));
      expect(liveSemantics.label, isNot(contains("35 / 100 张图片，35%")));

      await tester.pump(const Duration(milliseconds: 799));
      expect(
        tester.getSemantics(liveRegion).label,
        contains("10 / 100 张图片，10%"),
      );
      await tester.pump(const Duration(milliseconds: 1));
      expect(
        tester.getSemantics(liveRegion).label,
        contains("35 / 100 张图片，35%"),
      );

      updateState.value = _multiRootProgressState(
        validated: 35,
        firstPhase: LibraryRootUpdatePhase.refreshing,
      );
      await tester.pump();
      expect(
        tester.getSemantics(liveRegion).label,
        contains("Pictures，正在刷新显示"),
      );
    } finally {
      semantics.dispose();
    }
  });

  testWidgets("keeps import and configured-root update wording distinct", (
    tester,
  ) async {
    final existingRoot = _root("root-a", "C:\\Pictures");

    Future<void> pumpTask(LibraryState state) {
      return tester.pumpWidget(
        MaterialApp(
          home: Scaffold(
            body: LibraryTaskSurface(
              state: state,
              onPause: () {},
              onCancel: () {},
              onResume: () async {},
              onRetry: () async {},
              onDismiss: () {},
            ),
          ),
        ),
      );
    }

    await pumpTask(
      LibraryState(
        status: LibraryStatus.scanning,
        rootPath: existingRoot.path,
        displayRootPath: existingRoot.displayPath,
        taskKind: LibraryTaskKind.update,
        roots: [existingRoot],
      ),
    );
    expect(find.text("正在更新图库“Pictures”…"), findsOneWidget);
    expect(find.textContaining("正在添加文件夹"), findsNothing);

    await pumpTask(
      const LibraryState(
        status: LibraryStatus.scanning,
        rootPath: "D:\\New",
        displayRootPath: "D:\\New",
        taskKind: LibraryTaskKind.import,
      ),
    );
    expect(find.text("正在添加文件夹“New”…"), findsOneWidget);
    expect(find.textContaining("正在更新图库"), findsNothing);
  });

  testWidgets("keeps committed removal refresh recovery visible", (
    tester,
  ) async {
    final semantics = tester.ensureSemantics();
    try {
      Future<void> pumpTask(LibraryState state) {
        return tester.pumpWidget(
          MaterialApp(
            home: Scaffold(
              body: LibraryTaskSurface(
                state: state,
                onPause: () {},
                onCancel: () {},
                onResume: () async {},
                onRetry: () async {},
                onDismiss: () {},
              ),
            ),
          ),
        );
      }

      await pumpTask(
        const LibraryState(
          status: LibraryStatus.removing,
          taskKind: LibraryTaskKind.remove,
          removingRootId: "root-a",
          removingRootDisplayPath: "C:\\Pictures",
          isRemovalCommitted: true,
        ),
      );
      expect(find.text("已从 Ame 中移除“Pictures”，正在刷新显示…"), findsOneWidget);
      expect(
        find.text(LibraryStrings.refreshingAfterRemovalDetail),
        findsOneWidget,
      );
      final liveRegion = find.byKey(const Key("library-task-live-region"));
      expect(
        tester.getSemantics(liveRegion).flagsCollection.isLiveRegion,
        isTrue,
      );
      expect(
        tester.getSemantics(liveRegion).label,
        contains("已从 Ame 中移除“Pictures”，正在刷新显示"),
      );

      await pumpTask(
        const LibraryState(
          status: LibraryStatus.failed,
          taskKind: LibraryTaskKind.remove,
          removingRootId: "root-a",
          removingRootDisplayPath: "C:\\Pictures",
          isRemovalCommitted: true,
          errorMessage: "catalog_reload_failed",
        ),
      );
      expect(
        find.text(LibraryStrings.removedFolderRefreshFailed),
        findsOneWidget,
      );
      expect(
        find.byKey(const ValueKey("library-retry-button")),
        findsOneWidget,
      );
      expect(
        find.byKey(const ValueKey("library-task-dismiss-button")),
        findsNothing,
      );
      expect(
        tester.getSemantics(liveRegion).label,
        contains(LibraryStrings.removedFolderRefreshFailed),
      );
    } finally {
      semantics.dispose();
    }
  });
}

LibraryRoot _root(
  String id,
  String path, {
  LibraryRootAvailability availability = LibraryRootAvailability.available,
}) {
  return LibraryRoot(
    id: id,
    path: path,
    displayPath: path,
    activeScanId: "published-$id",
    createdUnixMs: 1,
    assetCount: 1,
    issueCount: 0,
    availability: availability,
  );
}

LibraryUpdateState _multiRootProgressState({
  required int validated,
  LibraryRootUpdatePhase firstPhase = LibraryRootUpdatePhase.finalizing,
}) {
  return LibraryUpdateState(
    tasksByRootId: {
      "root-a": LibraryRootUpdateTask(
        root: _root("root-a", "C:\\Pictures"),
        scanId: "update-a",
        phase: firstPhase,
        validatedItems: validated,
        validationItemCount: 100,
      ),
      "root-b": LibraryRootUpdateTask(
        root: _root("root-b", "D:\\Archive"),
        scanId: "update-b",
        phase: LibraryRootUpdatePhase.discovering,
        visitedEntries: 230400,
        acceptedItems: 8740,
      ),
    },
  );
}
