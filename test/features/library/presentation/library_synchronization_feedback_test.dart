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

  test("eligible automatic retry keeps its existing explanation", () {
    final feedback = LibrarySynchronizationFeedback.fromStatus(
      _status(
        recoveryBlocked: false,
        freshness: LibraryCatalogFreshness.updating,
        phase: LibrarySynchronizationPhase.retryWait,
        issueCode: "library_reconciliation_failed",
      ),
    );
    expect(feedback.message, LibraryStrings.synchronizationRecoveryFailed);
    expect(feedback.detail, contains("1 项等待重试"));
    expect(feedback.requiresManualLibraryUpdate, isFalse);
  });

  test("blocked phase alone does not invent exhausted recovery authority", () {
    final feedback = LibrarySynchronizationFeedback.fromStatus(
      _status(recoveryBlocked: false, issueCode: "catalog_database_busy"),
    );
    expect(feedback.message, LibraryStrings.synchronizationPersistenceFailed);
    expect(feedback.requiresManualLibraryUpdate, isFalse);
    expect(feedback.severity, AmeNotificationSeverity.error);
  });

  test("dormant first import retains its separate user-intent explanation", () {
    final feedback = LibrarySynchronizationFeedback.fromStatus(
      _status(
        issueCode: "library_first_import_required",
        recoveryBlocked: false,
        continuity: LibraryContinuityState.baselineRequired,
        sourceStatus: LibraryChangeSourceStatus.stopped,
        retryCount: 0,
      ),
    );
    expect(feedback.message, LibraryStrings.firstImportRequiredDetail);
    expect(feedback.detail, LibraryStrings.firstImportAwaitingUser);
    expect(feedback.severity, AmeNotificationSeverity.info);
    expect(feedback.requiresManualLibraryUpdate, isFalse);
  });

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
      final synchronization = _FeedbackSynchronization(_status());
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
      final container = ProviderScope.containerOf(
        tester.element(find.byType(AmeApp)),
      );
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
    },
  );
}

LibraryRootSynchronizationStatus _status({
  String? issueCode = "directory_unreadable",
  bool recoveryBlocked = true,
  LibraryCatalogFreshnessCause cause = LibraryCatalogFreshnessCause.evidenceGap,
  LibraryCatalogFreshness freshness =
      LibraryCatalogFreshness.needsReconciliation,
  LibrarySynchronizationPhase phase = LibrarySynchronizationPhase.blocked,
  LibraryContinuityState continuity = LibraryContinuityState.recoveryRequired,
  LibraryChangeSourceStatus sourceStatus = LibraryChangeSourceStatus.healthy,
  int retryCount = 1,
}) => LibraryRootSynchronizationStatus(
  rootId: "root-1",
  rootGeneration: BigInt.one,
  availability: LibraryRootAvailability.available,
  freshness: freshness,
  freshnessCause: cause,
  continuity: continuity,
  phase: phase,
  phaseStartedAt: DateTime.utc(2026, 9, 23),
  sourceStatus: sourceStatus,
  pendingChangeCount: BigInt.zero,
  retryWaitCount: BigInt.from(retryCount),
  freshnessUnknownCount: BigInt.one,
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
