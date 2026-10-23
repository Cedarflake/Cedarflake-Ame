import "../../../app/notifications/ame_notification_controller.dart";
import "../domain/library_models.dart";
import "../domain/library_synchronization_models.dart";
import "library_strings.dart";
import "library_synchronization_feedback.dart";

sealed class LibrarySynchronizationNotificationAction {
  const LibrarySynchronizationNotificationAction();
}

final class LibrarySynchronizationRefreshRetry
    extends LibrarySynchronizationNotificationAction {
  const LibrarySynchronizationRefreshRetry();
}

final class LibrarySynchronizationRootUpdate
    extends LibrarySynchronizationNotificationAction {
  const LibrarySynchronizationRootUpdate(this.root);

  final LibraryRoot root;
}

class LibrarySynchronizationNotifications {
  LibrarySynchronizationNotifications({
    required this._notifications,
    required this._readRoots,
  });

  static const _retrySynchronizationNotificationAction =
      "library.retrySynchronization";
  static const _refreshBlockedRootNotificationAction =
      "library.refreshBlockedRoot";
  static const _synchronizationRefreshNotificationKey =
      "library.synchronizationRefresh";
  static const _liveOnlyCapabilityNotificationKey =
      "library.liveOnlyCapability";

  final AmeNotificationController _notifications;
  final List<LibraryRoot> Function() _readRoots;
  final Map<String, String> _rootNotificationKeys = {};
  bool _hasLiveOnlyCapabilityNotification = false;

  void setRefreshFailure(bool hasFailure) {
    if (hasFailure) {
      _notifications.publish(
        const AmeNotificationDraft(
          title: LibraryStrings.synchronizationRefreshFailureTitle,
          message: LibraryStrings.synchronizationRefreshFailureMessage,
          severity: AmeNotificationSeverity.error,
          dedupeKey: _synchronizationRefreshNotificationKey,
          actionId: _retrySynchronizationNotificationAction,
          actionLabel: LibraryStrings.reload,
          isPersistent: true,
        ),
      );
    } else {
      _notifications.resolve(_synchronizationRefreshNotificationKey);
    }
  }

  void accept(LibrarySynchronizationSnapshot snapshot) {
    final roots = {for (final root in _readRoots()) root.id: root};
    final reportedRootIds = snapshot.roots.keys.toSet();
    for (final rootId in _rootNotificationKeys.keys.toList()) {
      if (!reportedRootIds.contains(rootId)) {
        final key = _rootNotificationKeys.remove(rootId);
        if (key != null) {
          _notifications.resolve(key);
        }
      }
    }
    for (final entry in snapshot.roots.entries) {
      final rootId = entry.key;
      final status = entry.value;
      final root = roots[rootId];
      final priorKey = _rootNotificationKeys[rootId];
      final feedback = LibrarySynchronizationFeedback.fromStatus(status);
      if (feedback.shouldNotify) {
        final key = _synchronizationNotificationKey(status);
        if (priorKey != null && priorKey != key) {
          _notifications.resolve(priorKey);
        }
        _rootNotificationKeys[rootId] = key;
        _notifications.publish(
          AmeNotificationDraft(
            title: feedback.titleForRoot(_notificationRootName(root)),
            message: feedback.message,
            severity: feedback.severity,
            dedupeKey: key,
            detail: feedback.detail,
            elapsedStartedAt: status.phaseStartedAt,
            sourcePath: root?.displayPath,
            technicalCode: status.lastIssueCode,
            actionId: feedback.requiresManualLibraryUpdate
                ? _refreshBlockedRootNotificationAction
                : null,
            actionLabel: feedback.requiresManualLibraryUpdate
                ? LibraryStrings.updateLibrary
                : null,
            isPersistent: true,
          ),
        );
      } else {
        if (priorKey != null) {
          _notifications.resolve(priorKey);
        }
        _rootNotificationKeys.remove(rootId);
      }
    }
    _setLiveOnlyCapabilityNotification(
      snapshot.roots.values.any(
        (status) =>
            status.availability == LibraryRootAvailability.available &&
            status.continuity == LibraryContinuityState.liveOnly &&
            !status.usesAutomaticMetadataFallback &&
            status.freshness == LibraryCatalogFreshness.synchronized &&
            status.freshnessCause ==
                LibraryCatalogFreshnessCause.noPendingChanges &&
            status.phase == LibrarySynchronizationPhase.synchronized &&
            status.sourceStatus == LibraryChangeSourceStatus.healthy &&
            status.pendingChangeCount == BigInt.zero &&
            status.retryWaitCount == BigInt.zero &&
            status.freshnessUnknownCount == BigInt.zero &&
            !status.recoveryBlocked,
      ),
    );
  }

  void _setLiveOnlyCapabilityNotification(bool hasHealthyLiveOnlyRoot) {
    if (_hasLiveOnlyCapabilityNotification == hasHealthyLiveOnlyRoot) {
      return;
    }
    _hasLiveOnlyCapabilityNotification = hasHealthyLiveOnlyRoot;
    if (hasHealthyLiveOnlyRoot) {
      _notifications.publish(
        const AmeNotificationDraft(
          title: LibraryStrings.continuityLiveOnlyNotice,
          message: LibraryStrings.continuityLiveOnlyNoticeDetail,
          severity: AmeNotificationSeverity.info,
          dedupeKey: _liveOnlyCapabilityNotificationKey,
        ),
      );
      return;
    }
    _notifications.resolve(_liveOnlyCapabilityNotificationKey);
  }

  String _synchronizationNotificationKey(
    LibraryRootSynchronizationStatus status,
  ) {
    return "library.rootReconciliation:${status.rootId}:${status.rootGeneration}";
  }

  String? _notificationRootName(LibraryRoot? root) {
    final displayPath = root?.displayPath.trim();
    if (displayPath == null || displayPath.isEmpty) {
      return null;
    }
    final normalized = displayPath.replaceAll("\\", "/");
    final segments = normalized
        .split("/")
        .where((segment) => segment.isNotEmpty)
        .toList();
    return segments.isEmpty ? displayPath : segments.last;
  }

  LibrarySynchronizationNotificationAction? admitAction(
    AmeNotificationEntry observed,
  ) {
    final action = _notifications.admitAction(observed);
    if (action == null) {
      return null;
    }
    switch (action.actionId) {
      case _retrySynchronizationNotificationAction:
        return const LibrarySynchronizationRefreshRetry();
      case _refreshBlockedRootNotificationAction:
        final rootPath = action.sourcePath;
        if (rootPath != null && rootPath.isNotEmpty) {
          for (final root in _readRoots()) {
            if (root.path == rootPath || root.displayPath == rootPath) {
              return LibrarySynchronizationRootUpdate(root);
            }
          }
        }
        return null;
      default:
        return null;
    }
  }
}
