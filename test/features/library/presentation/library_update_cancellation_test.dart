import "package:cedarflake_ame/features/library/application/library_scan_execution.dart";
import "package:cedarflake_ame/features/library/application/library_scanner.dart";
import "package:cedarflake_ame/features/library/application/library_update_controller.dart";
import "package:cedarflake_ame/features/library/domain/library_models.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_update_task_surface.dart";
import "package:flutter/material.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:flutter_test/flutter_test.dart";

import "../support/concurrent_library_scanner.dart";

void main() {
  testWidgets(
    "cancelling queued C preserves both active roots through completion",
    (tester) async {
      final scanner = ConcurrentLibraryScanner();
      final roots = [
        for (final name in ["A", "B", "C"]) _root(name),
      ];
      var refreshCount = 0;
      final container = ProviderContainer(
        overrides: [
          libraryScannerProvider.overrideWithValue(scanner),
          libraryUpdateConfiguredRootsProvider.overrideWithValue(roots),
          libraryUpdatePrimaryBusyProvider.overrideWithValue(false),
          libraryUpdateCatalogRefreshProvider.overrideWithValue(() async {
            refreshCount++;
          }),
        ],
      );
      addTearDown(scanner.dispose);
      addTearDown(container.dispose);
      final updates = container.read(libraryUpdateControllerProvider.notifier);
      final execution = container.read(libraryScanExecutionCoordinatorProvider);
      updates.startUpdates(roots.map((root) => root.id));
      await tester.pumpWidget(
        UncontrolledProviderScope(
          container: container,
          child: MaterialApp(
            home: Scaffold(
              body: Consumer(
                builder: (context, ref, child) => LibraryUpdateTaskSurface(
                  state: ref.watch(libraryUpdateControllerProvider),
                  onCancel: updates.cancel,
                  onRetry: updates.retry,
                  onDismiss: updates.dismiss,
                  onDismissTerminalTasks: updates.dismissTerminalTasks,
                ),
              ),
            ),
          ),
        ),
      );
      final scanA = scanner.scanIdForRootPath(r"C:\A");
      final scanB = scanner.scanIdForRootPath(r"C:\B");
      expect(scanner.activeScanIds.toSet(), {scanA, scanB});
      expect(execution.hasUpdateForRoot("C"), isTrue);
      final queuedRow = find.byKey(const ValueKey("library-update-task-C"));
      expect(
        find.descendant(of: queuedRow, matching: find.text("等待更新")),
        findsOneWidget,
      );
      await tester.tap(
        find.descendant(of: queuedRow, matching: find.text("取消")),
      );
      await tester.pump();
      expect(
        find.descendant(of: queuedRow, matching: find.text("已取消")),
        findsOneWidget,
      );
      expect(scanner.cancelledScanIds, isEmpty);
      expect(execution.hasUpdateForRoot("C"), isFalse);

      scanner.add(
        scanA,
        const LibraryScanProgress(
          visitedEntries: 17,
          acceptedItems: 11,
          issueCount: 1,
        ),
      );
      scanner.add(
        scanB,
        const LibraryScanProgress(
          visitedEntries: 29,
          acceptedItems: 23,
          issueCount: 2,
        ),
      );
      await tester.pump();
      final progressing = container.read(libraryUpdateControllerProvider);
      expect(progressing.tasksByRootId["A"]?.visitedEntries, 17);
      expect(progressing.tasksByRootId["B"]?.visitedEntries, 29);
      expect(
        progressing.tasksByRootId["C"]?.phase,
        LibraryRootUpdatePhase.cancelled,
      );

      scanner.add(scanA, _completed(11, 1));
      final closeA = scanner.close(scanA);
      await tester.pump();
      await closeA;
      await tester.pump();
      final afterA = container.read(libraryUpdateControllerProvider);
      expect(
        afterA.tasksByRootId["A"]?.phase,
        LibraryRootUpdatePhase.completed,
      );
      expect(afterA.tasksByRootId["B"]?.isActive, isTrue);
      expect(afterA.tasksByRootId["B"]?.visitedEntries, 29);
      expect(scanner.activeScanIds, [scanB]);
      expect(refreshCount, 1);
      expect(execution.hasUpdateForRoot("A"), isFalse);
      expect(execution.hasUpdateForRoot("B"), isTrue);
      expect(execution.canStartPrimary, isFalse);

      scanner.add(scanB, _completed(23, 2));
      final closeB = scanner.close(scanB);
      await tester.pump();
      await closeB;
      await tester.pump();
      final settled = container.read(libraryUpdateControllerProvider);
      expect(
        settled.tasksByRootId["A"]?.phase,
        LibraryRootUpdatePhase.completed,
      );
      expect(
        settled.tasksByRootId["B"]?.phase,
        LibraryRootUpdatePhase.completed,
      );
      expect(
        settled.tasksByRootId["C"]?.phase,
        LibraryRootUpdatePhase.cancelled,
      );
      expect(scanner.startedRootPaths, [r"C:\A", r"C:\B"]);
      expect(scanner.cancelledScanIds, isEmpty);
      expect(scanner.activeScanIds, isEmpty);
      expect(refreshCount, 2);
      expect(execution.hasUpdateScans, isFalse);
      expect(execution.canStartPrimary, isTrue);
      expect(find.text("图库更新已结束"), findsOneWidget);
      expect(tester.takeException(), isNull);
    },
  );
}

LibraryRoot _root(String name) => LibraryRoot(
  id: name,
  path: "C:\\$name",
  displayPath: "C:\\$name",
  activeScanId: "published-$name",
  createdUnixMs: 1,
  assetCount: 1,
  issueCount: 0,
  availability: LibraryRootAvailability.available,
);

LibraryScanCompleted _completed(int count, int issues) => LibraryScanCompleted(
  assetCount: count,
  issueCount: issues,
  catalogPath: "catalog.sqlite3",
  wasLimited: false,
);
