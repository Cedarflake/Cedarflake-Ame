import "package:cedarflake_ame/app/notifications/ame_notification_controller.dart";
import "package:cedarflake_ame/features/library/domain/library_models.dart";
import "package:cedarflake_ame/features/library/domain/library_synchronization_models.dart";
import "package:cedarflake_ame/features/library/presentation/library_strings.dart";
import "package:cedarflake_ame/features/library/presentation/library_synchronization_notifications.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:flutter_test/flutter_test.dart";

void main() {
  late ProviderContainer container;
  late AmeNotificationController notifications;
  late LibrarySynchronizationNotifications synchronizationNotifications;
  late List<LibraryRoot> roots;
  late int rootReads;

  AmeNotificationState state() =>
      container.read(ameNotificationControllerProvider);

  void accept(List<LibraryRootSynchronizationStatus> statuses) {
    synchronizationNotifications.accept(
      LibrarySynchronizationSnapshot(
        isRunning: true,
        catalogRevision: BigInt.one,
        appliedMutationCount: 0,
        roots: {for (final status in statuses) status.rootId: status},
      ),
    );
  }

  setUp(() {
    container = ProviderContainer();
    notifications = container.read(ameNotificationControllerProvider.notifier);
    roots = [_root()];
    rootReads = 0;
    synchronizationNotifications = LibrarySynchronizationNotifications(
      notifications: notifications,
      readRoots: () {
        rootReads += 1;
        return roots;
      },
    );
  });
  tearDown(() => container.dispose());

  test(
    "repeated blocked conditions update details without repeating alerts",
    () {
      accept([_blockedStatus()]);
      final observed = state().history.single;
      expect(observed.title, "“测试图库”更新受阻");
      expect(
        observed.message,
        LibraryStrings.synchronizationExplicitRecoveryRequired,
      );
      expect(observed.sourcePath, roots.single.displayPath);
      expect(observed.actionLabel, LibraryStrings.updateLibrary);
      expect(observed.isPersistent, isTrue);
      expect(observed.elapsedStartedAt, DateTime.utc(2026, 10, 18));
      notifications.dismiss(observed.id);
      notifications.markAllRead();

      accept([_blockedStatus(pendingCount: 3)]);
      final updated = state().history.single;
      expect(updated.id, observed.id);
      expect(updated.detail, "3 项等待处理");
      expect(state().pendingIds, isEmpty);
      expect(state().hasUnread, isFalse);
      expect(
        synchronizationNotifications.admitAction(observed),
        isA<LibrarySynchronizationRootUpdate>().having(
          (action) => action.root,
          "current root",
          same(roots.single),
        ),
      );
    },
  );

  test("a new root generation retires the prior action and timer", () {
    accept([_blockedStatus()]);
    final observed = state().history.single;
    accept([_blockedStatus(generation: 2)]);

    expect(state().history, hasLength(2));
    final retired = state().history.singleWhere(
      (entry) => entry.id == observed.id,
    );
    final current = state().history.singleWhere((entry) => entry.isActive);
    expect(retired.isResolvedCondition, isTrue);
    expect(retired.elapsedStartedAt, isNull);
    expect(retired.actionId, isNull);
    expect(state().pendingIds, [current.id]);
    final readsBeforeAction = rootReads;
    expect(synchronizationNotifications.admitAction(observed), isNull);
    expect(rootReads, readsBeforeAction);
    expect(
      synchronizationNotifications.admitAction(current),
      isA<LibrarySynchronizationRootUpdate>(),
    );
  });

  test(
    "removed roots resolve their notices without disturbing other roots",
    () {
      roots = [_root(), _root(id: "other", displayPath: r"C:\Other")];
      accept([_blockedStatus(), _blockedStatus(rootId: "other")]);
      final observed = state().history.singleWhere(
        (entry) => entry.sourcePath == roots.first.displayPath,
      );

      roots = [roots.last];
      accept([_blockedStatus(rootId: "other")]);
      expect(state().history, hasLength(2));
      expect(state().history.where((entry) => entry.isActive), hasLength(1));
      expect(state().pendingIds, hasLength(1));
      expect(synchronizationNotifications.admitAction(observed), isNull);
      accept([]);
      expect(
        state().history.every((entry) => entry.isResolvedCondition),
        isTrue,
      );
      expect(state().pendingIds, isEmpty);
    },
  );

  test(
    "recovery resolves history and a later recurrence creates a new alert",
    () {
      accept([_blockedStatus()]);
      final observed = state().history.single;
      accept([_healthyStatus(continuity: LibraryContinuityState.current)]);
      expect(state().history.single.isResolvedCondition, isTrue);
      expect(state().pendingIds, isEmpty);
      expect(synchronizationNotifications.admitAction(observed), isNull);

      accept([_blockedStatus()]);
      expect(state().history, hasLength(2));
      expect(state().history.first.id, isNot(observed.id));
      expect(state().pendingIds, [state().history.first.id]);
    },
  );

  test(
    "root actions use current roots and retain path matching precedence",
    () {
      accept([_blockedStatus()]);
      final observed = state().history.single;
      final replacement = _root(
        path: roots.single.displayPath,
        displayPath: "Renamed",
      );
      final secondMatch = _root(id: "other");
      roots = [replacement, secondMatch];

      expect(
        synchronizationNotifications.admitAction(observed),
        isA<LibrarySynchronizationRootUpdate>().having(
          (action) => action.root,
          "first current path match",
          same(replacement),
        ),
      );
      roots = [secondMatch];
      expect(
        synchronizationNotifications.admitAction(observed),
        isA<LibrarySynchronizationRootUpdate>().having(
          (action) => action.root,
          "current display path match",
          same(secondMatch),
        ),
      );
      roots = [];
      expect(synchronizationNotifications.admitAction(observed), isNull);
    },
  );

  test(
    "changed notification paths reject old actions before reading roots",
    () {
      accept([_blockedStatus()]);
      final observed = state().history.single;
      roots = [_root(displayPath: r"C:\Replacement")];
      accept([_blockedStatus()]);
      expect(state().history.single.id, observed.id);
      final readsBeforeAction = rootReads;
      expect(synchronizationNotifications.admitAction(observed), isNull);
      expect(rootReads, readsBeforeAction);
      expect(
        synchronizationNotifications.admitAction(state().history.single),
        isA<LibrarySynchronizationRootUpdate>(),
      );
    },
  );

  test(
    "refresh retry does not read roots and ends when the failure resolves",
    () {
      synchronizationNotifications.setRefreshFailure(true);
      final observed = state().history.single;
      expect(observed.title, LibraryStrings.synchronizationRefreshFailureTitle);
      expect(observed.actionLabel, LibraryStrings.reload);
      expect(observed.severity, AmeNotificationSeverity.error);
      expect(observed.isPersistent, isTrue);
      notifications.dismiss(observed.id);
      synchronizationNotifications.setRefreshFailure(true);
      expect(state().history, hasLength(1));
      expect(state().pendingIds, isEmpty);
      expect(
        synchronizationNotifications.admitAction(observed),
        isA<LibrarySynchronizationRefreshRetry>(),
      );
      synchronizationNotifications.setRefreshFailure(false);
      expect(state().history.single.isResolvedCondition, isTrue);
      expect(synchronizationNotifications.admitAction(observed), isNull);
      expect(rootReads, 0);
    },
  );

  test("unrecognized actions stay with their existing owner", () {
    notifications.publish(
      const AmeNotificationDraft(
        title: "Other action",
        message: "",
        severity: AmeNotificationSeverity.info,
        actionId: "other.action",
        actionLabel: "Open",
      ),
    );
    final unrelatedState = state();
    expect(
      synchronizationNotifications.admitAction(unrelatedState.history.single),
      isNull,
    );
    expect(rootReads, 0);
    expect(state(), same(unrelatedState));
  });

  test("automatic verification clears the manual-update capability notice", () {
    accept([_healthyStatus()]);
    expect(state().history.single.isResolvedCondition, isFalse);
    accept([_healthyStatus(issueCode: "persistent_journal_metadata_fallback")]);
    expect(state().history.single.isResolvedCondition, isTrue);
    accept([_healthyStatus(issueCode: "persistent_journal_metadata_fallback")]);
    expect(state().history, hasLength(1));
  });

  test("live-only notice follows healthy capability, not each snapshot", () {
    accept([_healthyStatus()]);
    final firstState = state();
    final observed = firstState.history.single;
    expect(observed.title, LibraryStrings.continuityLiveOnlyNotice);
    expect(observed.isPersistent, isFalse);
    expect(observed.actionId, isNull);
    accept([_healthyStatus(), _blockedStatus(rootId: "other")]);
    final withOtherRoot = state();
    expect(
      withOtherRoot.history.where((entry) => entry.id == observed.id).single,
      same(observed),
    );
    accept([_healthyStatus()]);
    final stableState = state();
    accept([_healthyStatus()]);
    expect(state(), same(stableState));

    accept([_healthyStatus(pendingCount: 1)]);
    expect(
      state().history
          .singleWhere((entry) => entry.id == observed.id)
          .isResolvedCondition,
      isTrue,
    );
    accept([_healthyStatus()]);
    final replacement = state().history.first;
    expect(replacement.id, isNot(observed.id));
    expect(replacement.title, observed.title);
    accept([]);
    expect(state().history.every((entry) => entry.isResolvedCondition), isTrue);
  });

  test(
    "missing root names preserve the generic notice without an action target",
    () {
      roots = [];
      accept([_blockedStatus()]);
      final observed = state().history.single;
      expect(observed.title, "图库更新受阻");
      expect(observed.sourcePath, isNull);
      final readsBeforeAction = rootReads;
      expect(synchronizationNotifications.admitAction(observed), isNull);
      expect(rootReads, readsBeforeAction);
    },
  );
}

LibraryRoot _root({
  String id = "root",
  String path = r"C:\Generated",
  String displayPath = " C:\\测试图库\\ ",
}) => LibraryRoot(
  id: id,
  path: path,
  displayPath: displayPath,
  createdUnixMs: 1,
  assetCount: 0,
  issueCount: 0,
);

LibraryRootSynchronizationStatus _blockedStatus({
  String rootId = "root",
  int generation = 1,
  int pendingCount = 0,
}) => LibraryRootSynchronizationStatus(
  rootId: rootId,
  rootGeneration: BigInt.from(generation),
  availability: LibraryRootAvailability.available,
  freshness: LibraryCatalogFreshness.needsReconciliation,
  freshnessCause: LibraryCatalogFreshnessCause.evidenceGap,
  continuity: LibraryContinuityState.recoveryRequired,
  phase: LibrarySynchronizationPhase.blocked,
  phaseStartedAt: DateTime.utc(2026, 10, 18),
  sourceStatus: LibraryChangeSourceStatus.healthy,
  pendingChangeCount: BigInt.from(pendingCount),
  retryWaitCount: BigInt.zero,
  freshnessUnknownCount: BigInt.zero,
  recoveryBlocked: true,
  lastIssueCode: "live_gap_v30_explicit_recovery_required",
);

LibraryRootSynchronizationStatus _healthyStatus({
  LibraryContinuityState continuity = LibraryContinuityState.liveOnly,
  int pendingCount = 0,
  String? issueCode,
}) => LibraryRootSynchronizationStatus(
  rootId: "root",
  rootGeneration: BigInt.one,
  availability: LibraryRootAvailability.available,
  freshness: LibraryCatalogFreshness.synchronized,
  freshnessCause: LibraryCatalogFreshnessCause.noPendingChanges,
  continuity: continuity,
  phase: LibrarySynchronizationPhase.synchronized,
  phaseStartedAt: DateTime.utc(2026, 10, 18),
  sourceStatus: LibraryChangeSourceStatus.healthy,
  pendingChangeCount: BigInt.from(pendingCount),
  retryWaitCount: BigInt.zero,
  freshnessUnknownCount: BigInt.zero,
  lastIssueCode: issueCode,
);
