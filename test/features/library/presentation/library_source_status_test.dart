import "dart:async";

import "package:cedarflake_ame/features/library/application/library_controller.dart";
import "package:cedarflake_ame/features/library/application/library_exact_analysis_controller.dart";
import "package:cedarflake_ame/features/library/application/library_exact_analysis_scheduler.dart";
import "package:cedarflake_ame/features/library/application/library_exact_duplicates.dart";
import "package:cedarflake_ame/features/library/domain/library_exact_duplicates.dart";
import "package:cedarflake_ame/features/library/domain/library_models.dart";
import "package:cedarflake_ame/features/library/domain/library_state.dart";
import "package:cedarflake_ame/features/library/domain/library_synchronization_models.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_analysis_activity.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_duplicate_update_status.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_source_navigation_tile.dart";
import "package:flutter/material.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:flutter_test/flutter_test.dart";

void main() {
  for (final fails in [false, true]) {
    testWidgets(
      "completed import stays synchronized through analysis ${fails ? 'failure' : 'publication'}",
      (tester) async {
        final service = _AnalysisService();
        final analysis = LibraryExactAnalysisController(service);
        final scheduler = LibraryExactAnalysisScheduler(
          analysis,
          idleDelay: const Duration(milliseconds: 20),
        );
        final container = ProviderContainer(
          overrides: [
            libraryControllerProvider.overrideWith(_Library.new),
            libraryExactAnalysisControllerProvider.overrideWithValue(analysis),
            libraryExactAnalysisSchedulerProvider.overrideWithValue(scheduler),
          ],
        );
        addTearDown(() {
          container.dispose();
          scheduler.dispose();
          analysis.dispose();
        });
        await tester.pumpWidget(
          UncontrolledProviderScope(
            container: container,
            child: MaterialApp(
              home: LibraryAnalysisActivity(
                child: Scaffold(
                  body: SizedBox(
                    width: 280,
                    child: Consumer(
                      builder: (context, ref, _) => Column(
                        children: [
                          for (final root
                              in ref.watch(libraryControllerProvider).roots)
                            LibrarySourceNavigationTile(
                              key: ValueKey(root.id),
                              root: root,
                              synchronizationStatus: _status(root.id),
                              isCompact: false,
                              isSelected: false,
                              isExpanded: false,
                              isBrowseDisabled: false,
                              onSelect: () {},
                              onToggleExpansion: null,
                              onUpdate: () {},
                              onOpen: () {},
                              onRemove: () {},
                            ),
                          LibraryDuplicateUpdateStatus(
                            hasPending: true,
                            builder: (_, label) => Text(
                              label ?? "",
                              key: const Key("duplicate-progress"),
                            ),
                          ),
                        ],
                      ),
                    ),
                  ),
                ),
              ),
            ),
          ),
        );
        await tester.pump(const Duration(milliseconds: 30));
        await service.complete();
        await tester.pumpAndSettle();

        final library =
            container.read(libraryControllerProvider.notifier) as _Library;
        library.completeAnimeImport();
        await tester.pump();
        expect(scheduler.isWaiting, isTrue);
        _expectSynchronized();
        _expectDuplicateStatus(tester, "等待后台");

        scheduler.setPaused(true);
        await tester.pump();
        _expectSynchronized();
        _expectDuplicateStatus(tester, "已暂停");
        scheduler.setPaused(false);
        await tester.pump(const Duration(milliseconds: 30));
        expect(analysis.isRunning, isTrue);
        _expectSynchronized();
        _expectDuplicateStatus(tester, "正在更新");

        library.publication = Completer<bool>();
        final before = library.refreshCount;
        await service.complete(failed: fails);
        await tester.pump();
        expect(analysis.isRunning, isFalse);
        _expectSynchronized();
        expect(library.refreshCount, before + 1);
        if (fails) {
          _expectDuplicateStatus(tester, "未完成查重");
        }
        library.publication!.complete(true);
        await tester.pumpAndSettle();
        _expectSynchronized();
        expect(tester.takeException(), isNull);
        await tester.pumpWidget(const SizedBox.shrink());
      },
    );
  }
}

void _expectSynchronized() {
  for (final rootId in ["landscape", "anime"]) {
    expect(
      find.descendant(
        of: find.byKey(ValueKey(rootId)),
        matching: find.text("已同步"),
      ),
      findsOneWidget,
    );
  }
}

void _expectDuplicateStatus(WidgetTester tester, String label) {
  expect(
    tester.widget<Text>(find.byKey(const Key("duplicate-progress"))).data,
    label,
  );
}

LibraryRoot _root(String id) => LibraryRoot(
  id: id,
  path: "C:\\Fixture\\$id",
  displayPath: id,
  activeScanId: "completed",
  createdUnixMs: 1,
  assetCount: 18,
  issueCount: 0,
  availability: LibraryRootAvailability.available,
);

LibraryRootSynchronizationStatus _status(String id) =>
    LibraryRootSynchronizationStatus(
      rootId: id,
      rootGeneration: BigInt.one,
      availability: LibraryRootAvailability.available,
      freshness: LibraryCatalogFreshness.synchronized,
      freshnessCause: LibraryCatalogFreshnessCause.noPendingChanges,
      continuity: LibraryContinuityState.current,
      phase: LibrarySynchronizationPhase.synchronized,
      phaseStartedAt: DateTime.utc(2026, 10, 23),
      sourceStatus: LibraryChangeSourceStatus.healthy,
      pendingChangeCount: BigInt.zero,
      retryWaitCount: BigInt.zero,
      freshnessUnknownCount: BigInt.zero,
    );

class _Library extends LibraryController {
  Completer<bool>? publication;
  int refreshCount = 0;

  @override
  LibraryState build() =>
      LibraryState(roots: [_root("landscape")], catalogRevision: BigInt.one);

  void completeAnimeImport() {
    state = state.copyWith(
      roots: [_root("landscape"), _root("anime")],
      catalogRevision: BigInt.two,
      primaryScan: const LibraryPrimaryScanSnapshot(
        taskKind: LibraryTaskKind.import,
        status: LibraryStatus.completed,
        publication: LibraryScanPublication.visible,
        rootPath: "C:\\Fixture\\anime",
      ),
    );
  }

  @override
  Future<bool> refreshCurrentQuery() async {
    refreshCount++;
    return publication?.future ?? true;
  }
}

class _AnalysisService implements LibraryExactDuplicates {
  late StreamController<LibraryExactAnalysisProgress> _events;
  late String _runId;

  @override
  Stream<LibraryExactAnalysisProgress> analyze(String runId) {
    _runId = runId;
    _events = StreamController<LibraryExactAnalysisProgress>();
    return _events.stream;
  }

  Future<void> complete({bool failed = false}) async {
    _events.add(
      LibraryExactAnalysisProgress(
        runId: _runId,
        phase: failed
            ? LibraryExactAnalysisPhase.failed
            : LibraryExactAnalysisPhase.completed,
        checkedFiles: 18,
        fingerprintedFiles: 18,
        skippedFiles: failed ? 1 : 0,
      ),
    );
    await _events.close();
  }

  @override
  Future<void> cancel(String runId) async {}

  @override
  Future<LibraryDuplicateGroupPage> loadGroups(
    LibraryDuplicateGroupCursor? after,
  ) => throw StateError("No group reads expected");

  @override
  Future<LibraryDuplicateLocationPage> loadLocations(
    LibraryDuplicateGroup group,
    String? after,
  ) => throw StateError("No location reads expected");
}
