import "dart:async";

import "package:cedarflake_ame/features/library/presentation/widgets/library_duplicate_group_thumbnail.dart";
import "package:cedarflake_ame/app/presentation/ame_dialog.dart";
import "package:cedarflake_ame/app/presentation/ame_loading_indicator.dart";

import "package:cedarflake_ame/features/library/application/library_file_operations.dart";
import "package:cedarflake_ame/features/library/application/library_controller.dart";
import "package:cedarflake_ame/features/library/application/library_organization.dart";
import "package:cedarflake_ame/features/library/application/library_duplicate_review_controller.dart";
import "package:cedarflake_ame/features/library/application/library_similar_review_controller.dart";
import "package:cedarflake_ame/features/library/application/library_file_selection.dart";
import "package:cedarflake_ame/features/library/application/library_file_selection_scope.dart";
import "package:cedarflake_ame/features/library/application/ports/library_file_operations.dart";
import "package:cedarflake_ame/features/library/domain/library_file_operations.dart";
import "package:cedarflake_ame/features/library/domain/library_models.dart";
import "package:cedarflake_ame/features/library/domain/library_organization.dart";
import "package:cedarflake_ame/features/library/domain/library_organization_batch.dart";
import "package:cedarflake_ame/features/library/domain/library_similar_review.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_file_action_dialog.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_file_history.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_photo_context_menu.dart";
import "package:cedarflake_ame/features/library/presentation/coordination/library_duplicate_workspace.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_duplicate_cleanup_action.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_duplicate_results.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:flutter_test/flutter_test.dart";
import "package:material_ui/material_ui.dart";
import "../../../support/application_test_bindings.dart";
import "../../../support/duplicate_review_fixtures.dart";
import "../../../support/similar_review_fixtures.dart";
import "../support/library_organization_fixture.dart";

void main() {
  testWidgets(
    "file selection uses the revision when the menu action is chosen",
    (tester) async {
      final backend = _Backend();
      final menu = LibraryPhotoContextMenu();
      final container = ProviderContainer(
        overrides: applicationTestOverrides([
          libraryControllerProvider.overrideWith(_RefreshingLibrary.new),
          libraryOrganizationProvider.overrideWithValue(
            OrganizationServiceFixture(),
          ),
          libraryFileOperationsProvider.overrideWithValue(backend),
        ]),
      );
      addTearDown(menu.dispose);
      addTearDown(container.dispose);
      final library =
          container.read(libraryControllerProvider.notifier)
              as _RefreshingLibrary;
      library.showMergedGallery();
      var current = true;
      await tester.pumpWidget(
        UncontrolledProviderScope(
          container: container,
          child: MaterialApp(
            home: Scaffold(
              body: Consumer(
                builder: (context, ref, _) => TextButton(
                  child: const Text("打开菜单"),
                  onPressed: () => menu.open(
                    context: context,
                    ref: ref,
                    position: const RelativeRect.fromLTRB(10, 40, 500, 0),
                    target: organizationTarget("one"),
                    isCurrent: () => current,
                    onOpen: () {},
                    onInformation: () {},
                    onCopyPath: () {},
                    onReveal: () {},
                  ),
                ),
              ),
            ),
          ),
        ),
      );
      await tester.tap(find.text("打开菜单"));
      await tester.pumpAndSettle();
      library.publishRevision(BigInt.two);
      await tester.tap(find.text("删除"));
      await tester.pumpAndSettle();
      final selected = backend.selections.single as LibraryFileGroupsInGallery;
      expect(selected.selection.catalogRevision, BigInt.two);
      expect(selected.selection.assetIds, ["asset-one"]);
      expect(backend.commands, isEmpty);
      await tester.tap(find.text("取消"));
      await tester.pumpAndSettle();
      await tester.tap(find.text("打开菜单"));
      await tester.pumpAndSettle();
      current = false;
      await tester.tap(find.text("删除"));
      await tester.pumpAndSettle();
      expect(backend.selections, hasLength(1));
      expect(backend.commands, isEmpty);
    },
  );

  testWidgets(
    "cleanup scopes and completion preserve group selection authority",
    (tester) async {
      final backend = _Backend()
        ..summary = const LibraryFileSelectionSummary(
          1,
          20,
          cleanup: LibraryDuplicateCleanupSummary(
            groups: 1,
            skippedGroups: 0,
            keptFiles: 2,
          ),
        );
      final workspace = LibraryDuplicateWorkspace(
        exact: LibraryDuplicateReviewController(ReviewFixtureService()),
        similar: LibrarySimilarReviewController(SimilarReviewFixture()),
      );
      addTearDown(workspace.dispose);
      await workspace.showView(LibraryDuplicateView.exact);
      await tester.pumpWidget(
        ProviderScope(
          overrides: applicationTestOverrides([
            libraryControllerProvider.overrideWith(ReviewFixtureLibrary.new),
            libraryFileOperationsProvider.overrideWithValue(backend),
          ]),
          child: MaterialApp(
            home: Scaffold(
              body: AnimatedBuilder(
                animation: workspace,
                builder: (context, _) => Column(
                  children: [
                    LibraryDuplicateCleanupAction(workspace: workspace),
                    Expanded(
                      child: LibraryDuplicateResults(
                        workspace: workspace,
                        analyzing: false,
                        onOpen: (_, _) {},
                        onPreferExact: (_) {},
                        onLocations: (_) {},
                      ),
                    ),
                  ],
                ),
              ),
            ),
          ),
        ),
      );
      await tester.pumpAndSettle();
      for (final view in [
        LibraryDuplicateView.exact,
        LibraryDuplicateView.similar,
      ]) {
        await workspace.showView(view);
        await tester.pumpAndSettle();
        await tester.tap(find.byKey(const Key("duplicate-cleanup")));
        await tester.pumpAndSettle();
        expect(
          (backend.selections.last as LibraryDuplicateCleanup).allGroups,
          isTrue,
        );
        expect(
          tester.widget<CheckboxListTile>(find.byType(CheckboxListTile)).value,
          isFalse,
        );
        await tester.tap(find.text("取消"));
        await tester.pumpAndSettle();
        await tester.tap(find.byType(LibraryDuplicateGroupThumbnail).first);
        await tester.pumpAndSettle();
        expect(
          find.text(view == LibraryDuplicateView.exact ? "删除重复图片" : "删除相似图片"),
          findsOneWidget,
        );
        await tester.tap(find.byKey(const Key("duplicate-cleanup")));
        await tester.pumpAndSettle();
        final selected = backend.selections.last as LibraryDuplicateCleanup;
        expect(selected.allGroups, isFalse);
        expect(selected.groups, hasLength(1));
        expect(
          selected.kind,
          view == LibraryDuplicateView.exact
              ? LibraryCleanupKind.exact
              : LibraryCleanupKind.similar,
        );
        expect(backend.appendedGroups.last, selected.groups);
        await tester.tap(find.text("取消"));
        await tester.pumpAndSettle();
        expect(workspace.cleanupSelection.isEmpty, isFalse);
        await tester.tap(
          find.text(view == LibraryDuplicateView.exact ? "查看重复图片" : "查看相似图片"),
        );
        await tester.pumpAndSettle();
        expect(workspace.cleanupSelection.groups, selected.groups);
        expect(
          find.text(view == LibraryDuplicateView.exact ? "删除重复图片" : "删除相似图片"),
          findsOneWidget,
        );
        await tester.tap(find.byKey(const Key("duplicate-cleanup")));
        await tester.pumpAndSettle();
        expect(
          (backend.selections.last as LibraryDuplicateCleanup).groups,
          selected.groups,
        );
        await tester.tap(find.text("取消"));
        await tester.pumpAndSettle();
        await workspace.showView(view);
        await tester.pumpAndSettle();
        expect(workspace.cleanupSelection.groups, selected.groups);
        expect(
          tester
              .widget<LibraryDuplicateGroupThumbnail>(
                find.byType(LibraryDuplicateGroupThumbnail).first,
              )
              .selected,
          isTrue,
        );
        if (view == LibraryDuplicateView.similar) {
          await workspace.showView(view, degree: LibrarySimilarityDegree.broad);
          await tester.pumpAndSettle();
        } else {
          await tester.tap(find.text("取消选择"));
          await tester.pumpAndSettle();
        }
        expect(workspace.cleanupSelection.isEmpty, isTrue);
        await tester.tap(
          find.text(view == LibraryDuplicateView.exact ? "查看重复图片" : "查看相似图片"),
        );
        await tester.pumpAndSettle();
        await workspace.showView(view);
        await tester.pumpAndSettle();
        expect(find.text("取消选择"), findsNothing);
      }
      backend.summary = const LibraryFileSelectionSummary(
        0,
        0,
        cleanup: LibraryDuplicateCleanupSummary(
          groups: 0,
          skippedGroups: 0,
          keptFiles: 0,
        ),
      );
      await tester.tap(find.byKey(const Key("duplicate-cleanup")));
      await tester.pumpAndSettle();
      expect(find.text("没有可删除的图片"), findsOneWidget);
      expect(
        tester
            .widget<FilledButton>(find.widgetWithText(FilledButton, "删除"))
            .onPressed,
        isNull,
      );
      await tester.tap(find.text("取消"));
      await tester.pumpAndSettle();
      expect(backend.commands, isEmpty);
      backend.summary = const LibraryFileSelectionSummary(
        1,
        20,
        cleanup: LibraryDuplicateCleanupSummary(
          groups: 1,
          skippedGroups: 0,
          keptFiles: 1,
        ),
      );
      await workspace.showView(LibraryDuplicateView.exact);
      await tester.pumpAndSettle();
      await tester.tap(find.byType(LibraryDuplicateGroupThumbnail).first);
      for (final published in [0, 1]) {
        backend.task.result = LibraryFileProgress(
          phase: LibraryFileTaskPhase.finished,
          total: 1,
          processed: 1,
          published: published,
        );
        await tester.pumpAndSettle();
        await tester.tap(find.byKey(const Key("duplicate-cleanup")));
        await tester.pumpAndSettle();
        await tester.tap(find.widgetWithText(FilledButton, "删除"));
        await tester.pumpAndSettle();
        expect(workspace.cleanupSelection.isEmpty, published > 0);
      }
      final released = Completer<void>();
      backend.task.closing = released.future;
      await tester.tap(find.byType(LibraryDuplicateGroupThumbnail).first);
      await tester.pumpAndSettle();
      await tester.tap(find.byKey(const Key("duplicate-cleanup")));
      await tester.pumpAndSettle();
      await tester.tap(find.widgetWithText(FilledButton, "删除"));
      await tester.pumpAndSettle();
      await tester.tap(find.byType(LibraryDuplicateGroupThumbnail).first);
      await tester.pump();
      await tester.tap(find.byType(LibraryDuplicateGroupThumbnail).first);
      await tester.pump();
      released.complete();
      await tester.pumpAndSettle();
      expect(workspace.cleanupSelection.isEmpty, isFalse);
      await workspace.showView(LibraryDuplicateView.ignored);
      await tester.pumpAndSettle();
      expect(workspace.cleanupSelection.isEmpty, isTrue);
      expect(find.byKey(const Key("duplicate-cleanup")), findsNothing);
    },
  );
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
    "history retry displays its new result after the worker retires",
    (tester) async {
      final backend = _Backend();
      final released = Completer<void>();
      backend.task.closing = released.future;
      LibraryFileRecord record(int id, LibraryFileRecordPhase phase) =>
          LibraryFileRecord(
            id: BigInt.from(id),
            action: LibraryFileAction.rename,
            source: r"C:\samples\before.png",
            destination: r"C:\samples\after.png",
            phase: phase,
            updatedAt: DateTime(2026, 11, 10),
            canContinueMove: false,
            canOpenRecycleBin: false,
            target: selection.locations.single,
            canRetry: phase == LibraryFileRecordPhase.failed,
          );
      final failed = record(1, LibraryFileRecordPhase.failed);
      backend.records = [failed];
      await tester.pumpWidget(
        ProviderScope(
          overrides: [libraryFileOperationsProvider.overrideWithValue(backend)],
          child: MaterialApp(
            home: Scaffold(
              body: Builder(
                builder: (context) => TextButton(
                  onPressed: () => showLibraryFileHistory(context),
                  child: const Text("打开记录"),
                ),
              ),
            ),
          ),
        ),
      );
      await tester.tap(find.text("打开记录"));
      await tester.pumpAndSettle();
      await tester.tap(find.text("重试"));
      await tester.pumpAndSettle();
      await tester.enterText(find.byType(TextField), "after.png");
      await tester.tap(find.widgetWithText(FilledButton, "重命名"));
      await tester.pumpAndSettle();
      expect(backend.commands.single.action, LibraryFileAction.rename);
      expect(find.text("已完成"), findsNothing);
      backend.records = [record(2, LibraryFileRecordPhase.published), failed];
      released.complete();
      await tester.pumpAndSettle();
      expect(find.text("已完成"), findsOneWidget);
      expect(find.text("未完成"), findsOneWidget);
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
      final opening = Completer<LibraryFileSelectionDraft>();
      backend.opening = opening.future;
      await tester.tap(find.text("打开删除确认"));
      await tester.pump();
      await tester.pump(const Duration(milliseconds: 500));
      await tester.pump(const Duration(milliseconds: 200));
      final progressFade = find
          .ancestor(
            of: find.byType(AmeLoadingIndicator),
            matching: find.byType(FadeTransition),
          )
          .first;
      final opacity = tester.widget<FadeTransition>(progressFade).opacity;
      expect(opacity.value, greaterThan(0.99));
      expect(
        tester.getCenter(find.byType(AmeLoadingIndicator)),
        tester.getCenter(find.byType(AmeDialogContent)),
      );
      expect(find.text("永久删除").hitTestable(), findsNothing);
      opening.completeError(
        const LibraryFileFailure(
          "organization_selection_changed",
          "Stale selection",
        ),
      );
      await tester.pump();
      expect(
        tester.widget<FadeTransition>(progressFade).opacity,
        same(opacity),
      );
      expect(opacity.value, greaterThan(0));
      expect(find.text("返回").hitTestable(), findsOneWidget);
      await tester.pumpAndSettle();
      expect(opacity.value, 0);
      expect(find.text("所选图片已变化，请查看当前图库后重新选择。"), findsOneWidget);
      expect(find.text("文件将移入 Windows 回收站。"), findsNothing);
      expect(find.byType(CheckboxListTile), findsNothing);
      expect(find.byType(FilledButton), findsNothing);
      await tester.tap(find.text("返回"));
      await tester.pumpAndSettle();
      expect(backend.commands, hasLength(1));
    },
  );
}

class _RefreshingLibrary extends OrganizationLibraryFixture {
  void showMergedGallery() {
    state = state.copyWith(
      query: state.query.copyWith(
        duplicateMode: LibraryGalleryDuplicateMode.merged,
      ),
    );
  }

  void publishRevision(BigInt revision) {
    state = state.copyWith(catalogRevision: revision);
  }
}

class _Backend implements LibraryFileOperations {
  final selections = <LibraryFileSelection>[];
  final appendedGroups = <List<LibraryCleanupGroup>>[];
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
  List<LibraryFileRecord> records = [];
  Future<LibraryFileSelectionDraft>? opening;
  @override
  Future<LibraryFileSelectionDraft> open(LibraryFileSelection selection) async {
    selections.add(selection);
    return opening ?? _Draft(this);
  }

  @override
  Future<LibraryFileTask> recover(
    List<BigInt> ids, {
    required bool continueMove,
  }) async => task;
  @override
  Future<List<LibraryFileRecord>> history({
    BigInt? before,
    int limit = 40,
  }) async => records;
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
  Future<void> appendGroups(List<LibraryCleanupGroup> groups) async {
    backend.appendedGroups.add(groups);
  }

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
  LibraryFileProgress result = const LibraryFileProgress(
    phase: LibraryFileTaskPhase.finished,
    total: 1,
    processed: 1,
  );
  @override
  Future<LibraryFileProgress> progress() async => result;
  @override
  Future<void> cancel() async {
    cancelled = true;
  }

  @override
  Future<void> close() async {
    await closing;
  }
}
