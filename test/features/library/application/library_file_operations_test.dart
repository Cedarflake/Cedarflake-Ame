import "dart:async";

import "package:cedarflake_ame/features/library/application/library_file_operations.dart";
import "package:cedarflake_ame/features/library/application/library_file_selection.dart";
import "package:cedarflake_ame/features/library/application/library_file_selection_scope.dart";
import "package:cedarflake_ame/features/library/application/ports/library_file_operations.dart";
import "package:cedarflake_ame/features/library/domain/library_file_operations.dart";
import "package:cedarflake_ame/features/library/domain/library_models.dart";
import "package:cedarflake_ame/features/library/domain/library_organization.dart";
import "package:cedarflake_ame/features/library/domain/library_organization_batch.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_file_action_dialog.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:flutter_test/flutter_test.dart";
import "package:material_ui/material_ui.dart";

void main() {
  final selection = LibraryFilesAtLocations([
    LibraryOrganizationTarget(
      assetId: "asset",
      locationId: "location",
      sourceGeneration: BigInt.one,
    ),
  ]);

  test(
    "merged gallery deletion expands groups while file actions retain locations",
    () {
      final gallery = LibraryOrganizationSelection(
        query: const LibraryGalleryQuery(
          duplicateMode: LibraryGalleryDuplicateMode.merged,
        ),
        catalogRevision: BigInt.one,
        allMatching: false,
        assetIds: ["asset"],
      );
      for (final action in LibraryFileAction.values) {
        final result = selectGalleryFilesForAction(
          action: action,
          selection: gallery,
          location: selection.locations.single,
        );
        if (action == LibraryFileAction.recycle ||
            action == LibraryFileAction.deletePermanently) {
          expect(result, isA<LibraryFileGroupsInGallery>());
        } else {
          expect(
            (result as LibraryFilesAtLocations).locations,
            selection.locations,
          );
        }
      }
      expect(
        selectGalleryFilesForAction(
          action: LibraryFileAction.recycle,
          selection: gallery,
        ),
        isA<LibraryFileGroupsInGallery>(),
      );
    },
  );

  test(
    "shutdown joins a completed native worker before exposing completion",
    () async {
      final backend = _Backend();
      final released = Completer<void>();
      backend.task.closing = released.future;
      final container = ProviderContainer(
        overrides: [libraryFileOperationsProvider.overrideWithValue(backend)],
      );
      final actions = container.read(libraryFileActionsProvider.notifier);
      final running = actions.execute(
        LibraryFileSelectionSession(backend, selection),
        const LibraryFileCommand(LibraryFileAction.recycle),
      );
      await Future<void>.delayed(Duration.zero);
      expect(container.read(libraryFileActionsProvider).busy, isTrue);
      var closed = false;
      final closing = actions.close().then((_) => closed = true);
      await Future<void>.delayed(Duration.zero);
      expect(backend.task.cancelled, isTrue);
      expect(closed, isFalse);
      released.complete();
      final result = await running;
      expect(result?.phase, LibraryFileTaskPhase.finished);
      await closing;
      expect(
        container.read(libraryFileActionsProvider).progress!.isFinished,
        isTrue,
      );
      container.dispose();
    },
  );

  test("a worker close failure remains actionable after stopping", () async {
    final backend = _Backend();
    final released = Completer<void>();
    backend.task.closing = released.future;
    final container = ProviderContainer(
      overrides: [libraryFileOperationsProvider.overrideWithValue(backend)],
    );
    final actions = container.read(libraryFileActionsProvider.notifier);
    final running = actions.recover([BigInt.one]);
    await Future<void>.delayed(Duration.zero);
    await actions.cancel();
    released.completeError(
      const LibraryFileFailure(
        "file_operation_worker_failed",
        "Receipt unavailable",
      ),
    );
    await running;
    final progress = container.read(libraryFileActionsProvider).progress!;
    expect(progress.phase, LibraryFileTaskPhase.needsAttention);
    expect(progress.failure?.code, "file_operation_worker_failed");
    expect(progress.processed, 1);
    container.dispose();
  });

  test(
    "closing a selection retires a late bridge handle without starting files",
    () async {
      final backend = _Backend();
      final opened = Completer<LibraryFileSelectionDraft>();
      backend.opening = opened.future;
      final session = LibraryFileSelectionSession(backend, selection);
      final result = session.execute(
        "cancelled",
        const LibraryFileCommand(LibraryFileAction.recycle),
      );
      final failure = expectLater(result, throwsA(isA<LibraryFileFailure>()));
      final closing = session.close();
      final draft = _Draft(backend);
      opened.complete(draft);
      await failure;
      await closing;
      expect(draft.closed, isTrue);
      expect(backend.commands, isEmpty);
    },
  );

  testWidgets(
    "each deletion requires confirmation and resets permanent deletion",
    (tester) async {
      final backend = _Backend()
        ..summary = const LibraryFileSelectionSummary(2, 40, mergedGroups: 1);
      await tester.pumpWidget(
        ProviderScope(
          overrides: [libraryFileOperationsProvider.overrideWithValue(backend)],
          child: MaterialApp(
            home: Scaffold(
              body: Consumer(
                builder: (context, ref, _) => TextButton(
                  onPressed: () => showLibraryFileAction(
                    context: context,
                    ref: ref,
                    selection: selection,
                    action: LibraryFileAction.recycle,
                  ),
                  child: const Text("打开删除确认"),
                ),
              ),
            ),
          ),
        ),
      );
      await tester.tap(find.text("打开删除确认"));
      await tester.pumpAndSettle();
      expect(find.text("删除这组图片的全部 2 个文件？"), findsOneWidget);
      expect(
        tester.widget<CheckboxListTile>(find.byType(CheckboxListTile)).value,
        isFalse,
      );
      expect(backend.commands, isEmpty);
      expect(
        tester
            .widget<FilledButton>(find.widgetWithText(FilledButton, "删除"))
            .onPressed,
        isNotNull,
      );
      await tester.tap(find.text("永久删除"));
      await tester.pump();
      await tester.tap(find.widgetWithText(FilledButton, "永久删除"));
      await tester.pumpAndSettle();
      expect(
        backend.commands.single.action,
        LibraryFileAction.deletePermanently,
      );
      await tester.tap(find.text("打开删除确认"));
      await tester.pumpAndSettle();
      expect(
        tester.widget<CheckboxListTile>(find.byType(CheckboxListTile)).value,
        isFalse,
      );
      await tester.tap(find.text("取消"));
      await tester.pumpAndSettle();
      expect(backend.commands, hasLength(1));
    },
  );
}

class _Backend implements LibraryFileOperations {
  LibraryFileSelectionSummary summary = const LibraryFileSelectionSummary(
    1,
    20,
  );
  @override
  Future<List<LibraryFileExport>> exports({
    BigInt? before,
    int limit = 40,
  }) async => const [];
  @override
  Future<LibraryFileTask> writeReport(String batchId) =>
      throw UnimplementedError();
  final task = _Task();
  final commands = <LibraryFileCommand>[];
  Future<LibraryFileSelectionDraft>? opening;
  @override
  Future<LibraryFileSelectionDraft> open(
    LibraryFileSelection selection,
  ) async => opening ?? _Draft(this);
  @override
  Future<LibraryFileTask> recover(
    List<BigInt> ids, {
    required bool continueMove,
  }) async => task;
  @override
  Future<List<LibraryFileRecord>> history({
    BigInt? before,
    int limit = 40,
  }) async => [];
  @override
  Future<void> openRecycleBin() async => throw UnimplementedError();
}

class _Draft implements LibraryFileSelectionDraft {
  _Draft(this.backend);
  final _Backend backend;
  bool closed = false;
  @override
  Future<void> appendIds(List<String> ids) async {}
  @override
  Future<void> appendLocations(
    List<LibraryOrganizationTarget> locations,
  ) async {}
  @override
  Future<LibraryFileSelectionSummary> prepare() async => backend.summary;
  @override
  Future<LibraryFileTask> execute(
    String batchId,
    LibraryFileCommand command,
  ) async {
    backend.commands.add(command);
    return backend.task;
  }

  @override
  Future<LibraryFileProgress> progress() async =>
      const LibraryFileProgress(phase: LibraryFileTaskPhase.preparing);
  @override
  Future<void> cancel() async {}
  @override
  Future<void> close() async {
    closed = true;
  }
}

class _Task implements LibraryFileTask {
  Future<void>? closing;
  bool cancelled = false;
  @override
  Future<LibraryFileProgress> progress() async => const LibraryFileProgress(
    phase: LibraryFileTaskPhase.finished,
    total: 1,
    processed: 1,
  );
  @override
  Future<void> cancel() async {
    cancelled = true;
  }

  @override
  Future<void> close() async {
    await closing;
  }
}
