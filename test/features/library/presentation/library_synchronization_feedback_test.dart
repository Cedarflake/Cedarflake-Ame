import "dart:async";

import "package:cedarflake_ame/app/ame_app.dart";
import "package:cedarflake_ame/app/notifications/ame_notification_controller.dart";
import "package:cedarflake_ame/features/library/application/library_controller.dart";
import "package:cedarflake_ame/features/library/application/library_scanner.dart";
import "package:cedarflake_ame/features/library/application/library_synchronization.dart";
import "package:cedarflake_ame/features/library/domain/library_models.dart";
import "package:cedarflake_ame/features/library/domain/library_state.dart";
import "package:cedarflake_ame/features/library/domain/library_synchronization_models.dart";
import "package:cedarflake_ame/features/library/presentation/library_strings.dart";
import "package:cedarflake_ame/features/library/presentation/library_synchronization_feedback.dart";
import "package:flutter/material.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:flutter_test/flutter_test.dart";

import "../support/concurrent_library_scanner.dart";

void main() {
  test("a notification without a loaded folder name uses a readable title", () {
    final feedback = LibrarySynchronizationFeedback.fromStatus(_status());
    expect(feedback.titleForRoot(null), "图库更新受阻");
    expect(feedback.titleForRoot("Pictures"), "“Pictures”更新受阻");
  });

  test("active baseline is progress rather than an unresolved fault", () {
    final feedback = LibrarySynchronizationFeedback.fromStatus(
      _status(
        recoveryBlocked: false,
        issueCode: null,
        freshness: LibraryCatalogFreshness.updating,
        phase: LibrarySynchronizationPhase.inventoryEnumeration,
        continuity: LibraryContinuityState.baselineRequired,
        retryCount: 0,
      ),
    );
    expect(feedback.message, "正在检查文件夹");
    expect(feedback.severity, AmeNotificationSeverity.info);
    expect(feedback.detail, isNull);
    expect(feedback.requiresManualLibraryUpdate, isFalse);
    expect(feedback.shouldNotify, isFalse);
    expect(feedback.sourceLabel, LibraryStrings.synchronizing);
    expect(feedback.titleForRoot("Pictures"), "“Pictures”正在更新");
  });

  test("unresolved evidence does not claim recovery is already running", () {
    for (final issue in [
      null,
      "library_reconciliation_failed",
      "catalog_database_busy",
      "change_source_event_incomplete",
      "change_source_watch_failed",
    ]) {
      final feedback = LibrarySynchronizationFeedback.fromStatus(
        _status(recoveryBlocked: false, issueCode: issue, retryCount: 0),
      );
      expect(feedback.message, isNot(contains("正在自动")));
      expect(feedback.message, isNot(contains("将自动重试")));
      expect(feedback.detail, isNot(contains("等待重试")));
    }
  });

  test("exhausted directory work does not promise another scheduled retry", () {
    final feedback = LibrarySynchronizationFeedback.fromStatus(_status());

    expect(feedback.message, LibraryStrings.synchronizationRecoveryBlocked);
    expect(feedback.detail, contains("1 项更新尚未完成"));
    expect(feedback.detail, isNot(contains("等待重试")));
    expect(feedback.requiresManualLibraryUpdate, isFalse);
    expect(feedback.severity, AmeNotificationSeverity.warning);
  });

  test("blocked authority precedes issue and freshness fallback promises", () {
    for (final issue in [
      null,
      "unknown_directory_failure",
      "change_source_callback_capacity_exceeded",
      "change_source_watch_failed",
      "authoritative_recovery_failed",
      "library_reconciliation_failed",
    ]) {
      for (final cause in LibraryCatalogFreshnessCause.values) {
        final feedback = LibrarySynchronizationFeedback.fromStatus(
          _status(issueCode: issue, cause: cause),
        );
        expect(
          feedback.message,
          LibraryStrings.synchronizationRecoveryBlocked,
          reason: "$issue / $cause",
        );
        expect(feedback.requiresManualLibraryUpdate, isFalse);
      }
    }
  });

  test(
    "exhausted persistence errors retain severity without scan admission",
    () {
      for (final issue in [
        "catalog_database_busy",
        "change_queue_write_failed",
        "database_unavailable",
      ]) {
        final feedback = LibrarySynchronizationFeedback.fromStatus(
          _status(issueCode: issue),
        );
        expect(feedback.message, LibraryStrings.synchronizationRecoveryBlocked);
        expect(feedback.severity, AmeNotificationSeverity.error);
        expect(feedback.requiresManualLibraryUpdate, isFalse);
      }
    },
  );

  test("eligible automatic retry is quiet progress with a past failure", () {
    final feedback = LibrarySynchronizationFeedback.fromStatus(
      _status(
        recoveryBlocked: false,
        freshness: LibraryCatalogFreshness.updating,
        phase: LibrarySynchronizationPhase.retryWait,
        issueCode: "library_reconciliation_failed",
      ),
    );
    expect(feedback.message, LibraryStrings.synchronizationAutomaticRetry);
    expect(feedback.detail, contains("1 项等待重试"));
    expect(feedback.requiresManualLibraryUpdate, isFalse);
    expect(feedback.shouldNotify, isFalse);
    expect(feedback.severity, AmeNotificationSeverity.info);
    expect(feedback.sourceLabel, LibraryStrings.synchronizationRetryPending);
  });

  test("updating freshness cannot hide a blocked or failed operation", () {
    for (final phase in [
      LibrarySynchronizationPhase.inventoryEnumeration,
      LibrarySynchronizationPhase.retryWait,
      LibrarySynchronizationPhase.queuePublication,
    ]) {
      for (final source in [
        LibraryChangeSourceStatus.healthy,
        LibraryChangeSourceStatus.failed,
      ]) {
        final feedback = LibrarySynchronizationFeedback.fromStatus(
          _status(
            freshness: LibraryCatalogFreshness.updating,
            phase: phase,
            sourceStatus: source,
          ),
        );
        expect(feedback.shouldNotify, isTrue);
        expect(feedback.sourceLabel, LibraryStrings.needsReconciliation);
        expect(feedback.titleForRoot("Pictures"), "“Pictures”更新受阻");
        expect(feedback.detail, "1 项更新尚未完成");
        expect(feedback.detail, isNot(contains("等待重试")));
        expect(feedback.message, isNot(contains("正在")));
        expect(feedback.severity, isNot(AmeNotificationSeverity.info));
      }
    }
  });

  test(
    "failed monitor remains visible during an unrelated automatic attempt",
    () {
      final feedback = LibrarySynchronizationFeedback.fromStatus(
        _status(
          freshness: LibraryCatalogFreshness.updating,
          phase: LibrarySynchronizationPhase.inventoryEnumeration,
          recoveryBlocked: false,
          sourceStatus: LibraryChangeSourceStatus.failed,
          issueCode: "change_source_watch_failed",
        ),
      );
      expect(feedback.shouldNotify, isTrue);
      expect(feedback.sourceLabel, LibraryStrings.needsReconciliation);
      expect(feedback.severity, AmeNotificationSeverity.error);
      expect(feedback.message, LibraryStrings.synchronizationMonitoringFailed);
      expect(feedback.detail, "1 项更新尚未完成");
      expect(feedback.detail, isNot(contains("正在核对")));
    },
  );

  test("unconfirmed evidence names no failure or scheduled recovery", () {
    final feedback = LibrarySynchronizationFeedback.fromStatus(
      _status(recoveryBlocked: false, issueCode: null),
    );
    expect(feedback.shouldNotify, isFalse);
    expect(feedback.severity, AmeNotificationSeverity.info);
    expect(feedback.titleForRoot("Pictures"), "“Pictures”图库尚未核对");
    expect(feedback.sourceLabel, LibraryStrings.synchronizationUnconfirmed);
    expect(feedback.detail, "1 项更新尚未完成 · 还有文件变化待检查");
    expect(feedback.detail, isNot(contains("等待重试")));
    expect(feedback.requiresManualLibraryUpdate, isFalse);
  });

  test("a retry phase without deferred work does not promise a retry", () {
    final feedback = LibrarySynchronizationFeedback.fromStatus(
      _status(
        recoveryBlocked: false,
        issueCode: null,
        freshness: LibraryCatalogFreshness.updating,
        phase: LibrarySynchronizationPhase.retryWait,
        retryCount: 0,
      ),
    );
    expect(feedback.detail, isNot(contains("等待重试")));
    expect(feedback.message, isNot(contains("等待重试")));
    expect(feedback.sourceLabel, LibraryStrings.synchronizationUnconfirmed);
  });

  test(
    "healthy active recovery does not turn historical issues into faults",
    () {
      for (final issue in [
        "catalog_database_busy",
        "change_source_rescan_required",
      ]) {
        final feedback = LibrarySynchronizationFeedback.fromStatus(
          _status(
            recoveryBlocked: false,
            issueCode: issue,
            freshness: LibraryCatalogFreshness.updating,
            phase: LibrarySynchronizationPhase.inventoryEnumeration,
          ),
        );
        expect(feedback.shouldNotify, isFalse);
        expect(feedback.sourceLabel, LibraryStrings.synchronizing);
        expect(
          feedback.message,
          LibraryStrings.synchronizationPhaseInventoryEnumeration,
        );
        expect(feedback.severity, AmeNotificationSeverity.info);
      }
    },
  );

  test("synchronized text requires continuity and no unresolved work", () {
    for (final continuity in [
      LibraryContinuityState.current,
      LibraryContinuityState.liveOnly,
      LibraryContinuityState.catchingUp,
    ]) {
      final feedback = LibrarySynchronizationFeedback.fromStatus(
        _status(
          recoveryBlocked: false,
          issueCode: null,
          freshness: LibraryCatalogFreshness.synchronized,
          phase: LibrarySynchronizationPhase.synchronized,
          continuity: continuity,
          retryCount: 0,
          unknownCount: 0,
        ),
      );
      if (continuity == LibraryContinuityState.catchingUp) {
        expect(feedback.sourceLabel, LibraryStrings.synchronizationUnconfirmed);
        expect(
          feedback.message,
          isNot(LibraryStrings.synchronizationCompleted),
        );
      } else if (continuity == LibraryContinuityState.liveOnly) {
        expect(feedback.shouldNotify, isFalse);
        expect(feedback.sourceLabel, LibraryStrings.continuityLiveOnly);
        expect(feedback.message, LibraryStrings.continuityLiveOnlyDetail);
        expect(feedback.detail, isNull);
      } else {
        expect(feedback.shouldNotify, isFalse);
        expect(feedback.message, LibraryStrings.synchronizationCompleted);
      }
    }
    final unresolved = LibrarySynchronizationFeedback.fromStatus(
      _status(
        recoveryBlocked: false,
        issueCode: null,
        freshness: LibraryCatalogFreshness.synchronized,
        phase: LibrarySynchronizationPhase.synchronized,
        continuity: LibraryContinuityState.current,
        retryCount: 0,
      ),
    );
    expect(unresolved.sourceLabel, LibraryStrings.synchronizationUnconfirmed);
  });

  test("blocked phase alone does not invent exhausted recovery authority", () {
    final feedback = LibrarySynchronizationFeedback.fromStatus(
      _status(recoveryBlocked: false, issueCode: "catalog_database_busy"),
    );
    expect(feedback.message, LibraryStrings.synchronizationPersistenceFailed);
    expect(feedback.requiresManualLibraryUpdate, isFalse);
    expect(feedback.severity, AmeNotificationSeverity.error);
  });

  test("retired manual recovery codes cannot override current progress", () {
    for (final issue in [
      "legacy_recovery_authority_missing",
      "live_gap_v30_explicit_recovery_required",
    ]) {
      final progress = LibrarySynchronizationFeedback.fromStatus(
        _status(
          issueCode: issue,
          recoveryBlocked: false,
          freshness: LibraryCatalogFreshness.updating,
          phase: LibrarySynchronizationPhase.inventoryComparison,
        ),
      );
      expect(progress.sourceLabel, LibraryStrings.synchronizing);
      expect(
        progress.message,
        LibraryStrings.synchronizationPhaseInventoryComparison,
      );
      expect(progress.shouldNotify, isFalse);
      expect(progress.requiresManualLibraryUpdate, isFalse);

      final completed = LibrarySynchronizationFeedback.fromStatus(
        _status(
          issueCode: issue,
          recoveryBlocked: false,
          freshness: LibraryCatalogFreshness.synchronized,
          phase: LibrarySynchronizationPhase.synchronized,
          continuity: LibraryContinuityState.current,
          retryCount: 0,
          unknownCount: 0,
        ),
      );
      expect(completed.sourceLabel, LibraryStrings.synchronized);
      expect(completed.message, LibraryStrings.synchronizationCompleted);
      expect(completed.shouldNotify, isFalse);
      expect(completed.requiresManualLibraryUpdate, isFalse);
    }
  });

  test("unavailable roots do not expose obsolete manual recovery actions", () {
    final feedback = LibrarySynchronizationFeedback.fromStatus(
      _status(
        availability: LibraryRootAvailability.offline,
        issueCode: "legacy_recovery_authority_missing",
      ),
    );
    expect(feedback.sourceLabel, LibraryStrings.sourceUnavailable);
    expect(feedback.message, LibraryStrings.synchronizationSourceUnavailable);
    expect(feedback.detail, "1 项更新尚未完成");
    expect(feedback.requiresManualLibraryUpdate, isFalse);
    expect(feedback.detail, isNot(contains("等待重试")));
  });

  test("unavailable phase cannot claim automatic processing", () {
    final feedback = LibrarySynchronizationFeedback.fromStatus(
      _status(
        recoveryBlocked: false,
        issueCode: null,
        freshness: LibraryCatalogFreshness.updating,
        phase: LibrarySynchronizationPhase.unavailable,
      ),
    );
    expect(feedback.sourceLabel, LibraryStrings.synchronizationUnconfirmed);
    expect(feedback.message, LibraryStrings.synchronizationEvidenceGap);
    expect(feedback.shouldNotify, isFalse);
  });

  test(
    "dormant first import leaves guidance in the gallery and task panel",
    () {
      final feedback = LibrarySynchronizationFeedback.fromStatus(
        _status(
          issueCode: "library_first_import_required",
          recoveryBlocked: false,
          continuity: LibraryContinuityState.baselineRequired,
          sourceStatus: LibraryChangeSourceStatus.stopped,
          retryCount: 0,
        ),
      );
      expect(feedback.message, LibraryStrings.firstImportIncomplete);
      expect(feedback.detail, isNull);
      expect(feedback.shouldNotify, isFalse);
      expect(feedback.severity, AmeNotificationSeverity.info);
      expect(feedback.requiresManualLibraryUpdate, isFalse);
    },
  );

  test(
    "legacy and explicit manual recovery preserve their distinct contracts",
    () {
      final legacy = LibrarySynchronizationFeedback.fromStatus(
        _status(issueCode: "legacy_recovery_authority_missing"),
      );
      expect(
        legacy.message,
        LibraryStrings.synchronizationLegacyRecoveryAuthorityMissing,
      );
      expect(legacy.requiresManualLibraryUpdate, isTrue);

      final explicit = LibrarySynchronizationFeedback.fromStatus(
        _status(issueCode: "live_gap_v30_explicit_recovery_required"),
      );
      expect(
        explicit.message,
        LibraryStrings.synchronizationExplicitRecoveryRequired,
      );
      expect(explicit.requiresManualLibraryUpdate, isTrue);

      final unblocked = LibrarySynchronizationFeedback.fromStatus(
        _status(
          issueCode: "live_gap_v30_explicit_recovery_required",
          recoveryBlocked: false,
        ),
      );
      expect(unblocked.requiresManualLibraryUpdate, isFalse);
    },
  );

  testWidgets(
    "retained blocked feedback explains recovery without starting a scan",
    (tester) async {
      tester.view.physicalSize = const Size(1280, 800);
      tester.view.devicePixelRatio = 1;
      addTearDown(tester.view.resetPhysicalSize);
      addTearDown(tester.view.resetDevicePixelRatio);
      final synchronization = _FeedbackSynchronization(
        _status(recoveryBlocked: false, issueCode: null, retryCount: 0),
      );
      final scanner = ConcurrentLibraryScanner();
      addTearDown(synchronization.dispose);
      addTearDown(scanner.dispose);

      await tester.pumpWidget(
        ProviderScope(
          overrides: [
            initialLibraryStateProvider.overrideWithValue(
              LibraryState(
                status: LibraryStatus.completed,
                catalogRevision: BigInt.one,
                roots: const [
                  LibraryRoot(
                    id: "root-1",
                    path: r"C:\Generated",
                    displayPath: r"C:\Generated",
                    createdUnixMs: 1,
                    assetCount: 0,
                    issueCount: 0,
                    availability: LibraryRootAvailability.available,
                  ),
                ],
              ),
            ),
            libraryScannerProvider.overrideWithValue(scanner),
            librarySynchronizationProvider.overrideWithValue(synchronization),
          ],
          child: const AmeApp(),
        ),
      );
      await tester.pump();

      final container = ProviderScope.containerOf(
        tester.element(find.byType(AmeApp)),
      );
      expect(
        container.read(ameNotificationControllerProvider).history,
        isEmpty,
      );
      expect(
        find.text(LibraryStrings.synchronizationUnconfirmed),
        findsOneWidget,
      );
      synchronization.publish(_status());
      await tester.pump();
      expect(
        container.read(ameNotificationControllerProvider).history,
        hasLength(1),
      );
      await tester.pump();

      expect(
        find.text(LibraryStrings.synchronizationRecoveryBlocked),
        findsOneWidget,
      );
      expect(
        find.text(LibraryStrings.synchronizationEvidenceGap),
        findsNothing,
      );
      expect(find.textContaining("1 项更新尚未完成"), findsOneWidget);
      expect(find.textContaining("1 项等待重试"), findsNothing);
      expect(
        find.byKey(const Key("notification-primary-action")),
        findsNothing,
      );
      expect(scanner.startedRootPaths, isEmpty);

      synchronization.publish(_status(issueCode: "catalog_database_busy"));
      await tester.pump();
      final history = container.read(ameNotificationControllerProvider).history;
      expect(history, hasLength(1));
      expect(
        history.single.message,
        LibraryStrings.synchronizationRecoveryBlocked,
      );
      expect(history.single.technicalCode, "catalog_database_busy");
      expect(history.single.severity, AmeNotificationSeverity.error);
      expect(history.single.actionId, isNull);
      expect(scanner.startedRootPaths, isEmpty);

      synchronization.publish(
        _status(
          recoveryBlocked: true,
          freshness: LibraryCatalogFreshness.updating,
          phase: LibrarySynchronizationPhase.retryWait,
        ),
      );
      await tester.pump();
      await tester.pump();
      expect(find.text("“Generated”更新受阻"), findsOneWidget);
      expect(
        find.text(LibraryStrings.synchronizationRetryPending),
        findsNothing,
      );
      expect(
        find.text(LibraryStrings.synchronizationPhaseInventoryEnumeration),
        findsNothing,
      );

      synchronization.publish(
        _status(
          recoveryBlocked: false,
          freshness: LibraryCatalogFreshness.updating,
          phase: LibrarySynchronizationPhase.retryWait,
          issueCode: "library_reconciliation_failed",
        ),
      );
      await tester.pump();
      await tester.pump(const Duration(milliseconds: 300));
      expect(find.text("“Generated”更新受阻"), findsNothing);
      expect(
        find.text(LibraryStrings.synchronizationRetryPending),
        findsOneWidget,
      );
      expect(
        container
            .read(ameNotificationControllerProvider)
            .history
            .single
            .isActive,
        isFalse,
      );
      expect(scanner.startedRootPaths, isEmpty);

      synchronization.publish(
        _status(
          recoveryBlocked: false,
          issueCode: null,
          freshness: LibraryCatalogFreshness.updating,
          phase: LibrarySynchronizationPhase.inventoryEnumeration,
          continuity: LibraryContinuityState.baselineRequired,
          retryCount: 0,
        ),
      );
      await tester.pump();
      await tester.pump(const Duration(milliseconds: 300));
      expect(
        find.text(LibraryStrings.synchronizationRecoveryBlocked),
        findsNothing,
      );
      expect(find.textContaining("更新受阻"), findsNothing);
      expect(find.text(LibraryStrings.synchronizing), findsOneWidget);
      expect(find.text(LibraryStrings.synchronized), findsNothing);
      expect(scanner.startedRootPaths, isEmpty);

      synchronization.publish(
        _status(
          recoveryBlocked: false,
          issueCode: null,
          freshness: LibraryCatalogFreshness.synchronized,
          phase: LibrarySynchronizationPhase.synchronized,
          continuity: LibraryContinuityState.current,
          retryCount: 0,
          unknownCount: 0,
        ),
      );
      await tester.pump();
      await tester.pump();
      expect(find.text(LibraryStrings.synchronized), findsOneWidget);
      expect(find.text(LibraryStrings.synchronizing), findsNothing);
      expect(
        container.read(ameNotificationControllerProvider).history,
        hasLength(1),
      );
      expect(scanner.startedRootPaths, isEmpty);
    },
  );
}

LibraryRootSynchronizationStatus _status({
  LibraryRootAvailability availability = LibraryRootAvailability.available,
  String? issueCode = "directory_unreadable",
  bool recoveryBlocked = true,
  LibraryCatalogFreshnessCause cause = LibraryCatalogFreshnessCause.evidenceGap,
  LibraryCatalogFreshness freshness =
      LibraryCatalogFreshness.needsReconciliation,
  LibrarySynchronizationPhase phase = LibrarySynchronizationPhase.blocked,
  LibraryContinuityState continuity = LibraryContinuityState.recoveryRequired,
  LibraryChangeSourceStatus sourceStatus = LibraryChangeSourceStatus.healthy,
  int retryCount = 1,
  int unknownCount = 1,
}) => LibraryRootSynchronizationStatus(
  rootId: "root-1",
  rootGeneration: BigInt.one,
  availability: availability,
  freshness: freshness,
  freshnessCause: cause,
  continuity: continuity,
  phase: phase,
  phaseStartedAt: DateTime.utc(2026, 9, 23),
  sourceStatus: sourceStatus,
  pendingChangeCount: BigInt.zero,
  retryWaitCount: BigInt.from(retryCount),
  freshnessUnknownCount: BigInt.from(unknownCount),
  recoveryBlocked: recoveryBlocked,
  lastIssueCode: issueCode,
);

class _FeedbackSynchronization extends InertLibrarySynchronization {
  _FeedbackSynchronization(LibraryRootSynchronizationStatus status)
    : _current = _snapshot(status);

  final _changes = StreamController<LibrarySynchronizationSnapshot>.broadcast();
  LibrarySynchronizationSnapshot _current;

  @override
  LibrarySynchronizationSnapshot get current => _current;

  void publish(LibraryRootSynchronizationStatus status) {
    _current = _snapshot(status);
    _changes.add(_current);
  }

  @override
  Stream<LibrarySynchronizationSnapshot> watch() => _changes.stream;

  @override
  Future<void> dispose() => _changes.close();

  static LibrarySynchronizationSnapshot _snapshot(
    LibraryRootSynchronizationStatus status,
  ) => LibrarySynchronizationSnapshot(
    isRunning: true,
    catalogRevision: BigInt.one,
    appliedMutationCount: 0,
    roots: {status.rootId: status},
  );
}
