import "dart:async";

import "package:cedarflake_ame/features/library/application/library_controller.dart";
import "package:cedarflake_ame/features/library/application/library_exact_analysis_controller.dart";
import "package:cedarflake_ame/features/library/application/library_exact_analysis_scheduler.dart";
import "package:cedarflake_ame/features/library/application/library_exact_duplicates.dart";
import "package:cedarflake_ame/features/library/domain/library_exact_duplicates.dart";
import "package:cedarflake_ame/features/library/domain/library_models.dart";
import "package:cedarflake_ame/features/library/domain/library_state.dart";
import "package:cedarflake_ame/features/library/domain/library_synchronization_models.dart";
import "package:cedarflake_ame/features/library/presentation/library_results_source_activity.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_analysis_activity.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_results_progress.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_source_navigation_tile.dart";
import "package:flutter/material.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:flutter_test/flutter_test.dart";

void main() {
  test("unrelated catalog publication does not update every source", () {
    final activity = LibraryResultsSourceActivity();
    addTearDown(activity.dispose);
    _update(activity, LibraryResultsProgress.ready);
    _update(activity, LibraryResultsProgress.publishing);
    expect(activity.rootIds, isEmpty);
  });

  test("new source owns waiting, failure and publication until ready", () {
    final activity = LibraryResultsSourceActivity();
    addTearDown(activity.dispose);
    _update(
      activity,
      LibraryResultsProgress.ready,
      roots: [_root("landscape")],
    );
    for (final progress in [
      LibraryResultsProgress.waiting,
      LibraryResultsProgress.analyzing,
      LibraryResultsProgress.paused,
      LibraryResultsProgress.analysisFailed,
      LibraryResultsProgress.publishing,
      LibraryResultsProgress.publicationFailed,
    ]) {
      _update(activity, progress);
      expect(activity.rootIds, {"anime"});
    }
    _update(activity, LibraryResultsProgress.ready);
    expect(activity.rootIds, isEmpty);
  });

  test("same-count source changes survive the synchronization handoff", () {
    final activity = LibraryResultsSourceActivity();
    addTearDown(activity.dispose);
    _update(activity, LibraryResultsProgress.ready);
    _update(
      activity,
      LibraryResultsProgress.ready,
      statuses: [_status("anime", updating: true)],
    );
    expect(activity.rootIds, {"anime"});
    _update(activity, LibraryResultsProgress.waiting);
    expect(activity.rootIds, {"anime"});
    _update(activity, LibraryResultsProgress.ready);
    expect(activity.rootIds, isEmpty);
  });

  test(
    "manual updates stay scoped and removed roots retire their feedback",
    () {
      final activity = LibraryResultsSourceActivity();
      addTearDown(activity.dispose);
      _update(activity, LibraryResultsProgress.ready);
      _update(activity, LibraryResultsProgress.waiting, updating: {"anime"});
      expect(activity.rootIds, {"anime"});
      _update(
        activity,
        LibraryResultsProgress.analysisFailed,
        roots: [_root("landscape")],
      );
      expect(activity.rootIds, isEmpty);
    },
  );

  test("replacement scans update their source without a count change", () {
    final activity = LibraryResultsSourceActivity();
    addTearDown(activity.dispose);
    _update(activity, LibraryResultsProgress.ready);
    _update(
      activity,
      LibraryResultsProgress.waiting,
      roots: [
        _root("landscape"),
        _root("anime", scanId: "replacement"),
      ],
    );
    expect(activity.rootIds, {"anime"});
  });

  testWidgets("adding Anime leaves the settled landscape row synchronized", (
    tester,
  ) async {
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
    _expectLabel("landscape", "已同步");

    final library =
        container.read(libraryControllerProvider.notifier) as _Library;
    library.addAnime();
    await tester.pump();
    _expectLabel("landscape", "已同步");
    _expectLabel("anime", "正在更新");
    await tester.pump(const Duration(milliseconds: 30));
    _expectLabel("landscape", "已同步");
    _expectLabel("anime", "正在更新");

    library.publication = Completer<bool>();
    await service.complete();
    await tester.pump();
    _expectLabel("landscape", "已同步");
    _expectLabel("anime", "正在更新");
    library.publication!.complete(true);
    await tester.pumpAndSettle();
    _expectLabel("landscape", "已同步");
    _expectLabel("anime", "已同步");
    expect(tester.takeException(), isNull);
    await tester.pumpWidget(const SizedBox.shrink());
  });
}

void _expectLabel(String rootId, String label) {
  expect(
    find.descendant(
      of: find.byKey(ValueKey(rootId)),
      matching: find.text(label),
    ),
    findsOneWidget,
  );
}

LibraryRoot _root(String id, {String scanId = "completed"}) => LibraryRoot(
  id: id,
  path: "C:\\Fixture\\$id",
  displayPath: id,
  activeScanId: scanId,
  createdUnixMs: 1,
  assetCount: 18,
  issueCount: 0,
  availability: LibraryRootAvailability.available,
);

LibraryRootSynchronizationStatus _status(String id, {bool updating = false}) =>
    LibraryRootSynchronizationStatus(
      rootId: id,
      rootGeneration: BigInt.one,
      availability: LibraryRootAvailability.available,
      freshness: updating
          ? LibraryCatalogFreshness.updating
          : LibraryCatalogFreshness.synchronized,
      freshnessCause: updating
          ? LibraryCatalogFreshnessCause.pendingChanges
          : LibraryCatalogFreshnessCause.noPendingChanges,
      continuity: LibraryContinuityState.current,
      phase: updating
          ? LibrarySynchronizationPhase.queuePublication
          : LibrarySynchronizationPhase.synchronized,
      phaseStartedAt: DateTime.utc(2026, 10, 23),
      sourceStatus: LibraryChangeSourceStatus.healthy,
      pendingChangeCount: updating ? BigInt.one : BigInt.zero,
      retryWaitCount: BigInt.zero,
      freshnessUnknownCount: BigInt.zero,
    );

void _update(
  LibraryResultsSourceActivity activity,
  LibraryResultsProgress progress, {
  List<LibraryRoot>? roots,
  List<LibraryRootSynchronizationStatus> statuses = const [],
  Set<String> updating = const {},
}) => activity.update(
  roots: roots ?? [_root("landscape"), _root("anime")],
  synchronization: LibrarySynchronizationSnapshot(
    isRunning: true,
    catalogRevision: BigInt.one,
    appliedMutationCount: 0,
    roots: {for (final status in statuses) status.rootId: status},
  ),
  updatingRootIds: updating,
  progress: progress,
);

class _Library extends LibraryController {
  Completer<bool>? publication;

  @override
  LibraryState build() =>
      LibraryState(roots: [_root("landscape")], catalogRevision: BigInt.one);

  void addAnime() {
    state = state.copyWith(
      roots: [_root("landscape"), _root("anime")],
      catalogRevision: BigInt.two,
    );
  }

  @override
  Future<bool> refreshCurrentQuery() async => publication?.future ?? true;
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

  Future<void> complete() async {
    _events.add(
      LibraryExactAnalysisProgress(
        runId: _runId,
        phase: LibraryExactAnalysisPhase.completed,
        checkedFiles: 18,
        fingerprintedFiles: 18,
        skippedFiles: 0,
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
