import "dart:async";

import "package:flutter/foundation.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";

import "../../../app/notifications/ame_notification_controller.dart";
import "../domain/library_organization.dart";
import "library_controller.dart";
import "library_organization.dart";
import "library_organization_outcome.dart";
import "library_organization_requests.dart";

class LibraryOrganizationActivity {
  const LibraryOrganizationActivity({this.saving = false, this.sequence = 0});
  final bool saving;
  final int sequence;
}

final libraryOrganizationNoticeProvider = Provider<AmeNotificationEntry?>((
  ref,
) {
  final state = ref.watch(ameNotificationControllerProvider);
  final outcome = ref.watch(libraryOrganizationOutcomeProvider);
  final pending = state.pendingIds.toSet();
  return state.history
          .where(
            (entry) =>
                pending.contains(entry.id) &&
                entry.dedupeKey?.startsWith("organization-") == true,
          )
          .firstOrNull ??
      outcome;
});

final libraryOrganizationActionsProvider =
    NotifierProvider<LibraryOrganizationActions, LibraryOrganizationActivity>(
      LibraryOrganizationActions.new,
    );

class LibraryOrganizationActions extends Notifier<LibraryOrganizationActivity> {
  final Map<String, LibraryOrganizationRequest> _failed = {};
  LibraryOrganizationRequest? _active;
  bool _disposed = false;

  @override
  LibraryOrganizationActivity build() {
    ref.onDispose(() {
      _disposed = true;
      _active?.cancel();
      for (final request in _failed.values) {
        request.cancel();
      }
    });
    return const LibraryOrganizationActivity();
  }

  Future<void> favorite(LibraryOrganizationTarget target, bool favorite) async {
    await _run(LibraryFavoriteRequest(target, favorite));
  }

  Future<LibraryOrganizationCommandResult> submit(
    LibraryOrganizationRequest request,
  ) => _run(request);

  void retire(LibraryOrganizationRequest request) {
    request.cancel();
    _failed.removeWhere((_, pending) => identical(pending, request));
    if (!_disposed) {
      ref
          .read(ameNotificationControllerProvider.notifier)
          .resolve("organization-save-${request.feedbackId}");
    }
  }

  bool handleNotification(AmeNotificationEntry observed) {
    if (observed.actionId?.startsWith("organization.") != true) {
      return false;
    }
    final notifications = ref.read(ameNotificationControllerProvider.notifier);
    final outcomes = ref.read(libraryOrganizationOutcomeProvider.notifier);
    final current =
        outcomes.admitAction(observed) ?? notifications.admitAction(observed);
    if (current == null || state.saving) {
      return true;
    }
    final context = current.actionContext;
    switch (current.actionId) {
      case "organization.undo" when context != null:
        unawaited(_run(LibraryOrganizationUndoRequest(context)));
        outcomes.dismiss(current.id);
        notifications.dismiss(current.id);
      case "organization.retry" when context != null:
        final request = _failed[context];
        if (request != null) {
          unawaited(_run(request));
        }
      case "organization.refresh":
        unawaited(_refresh());
      default:
        break;
    }
    return true;
  }

  Future<LibraryOrganizationCommandResult> _run(
    LibraryOrganizationRequest request,
  ) async {
    if (_disposed || state.saving || request.isCancelled) {
      return const LibraryOrganizationRetired();
    }
    final notifications = ref.read(ameNotificationControllerProvider.notifier);
    final service = ref.read(libraryOrganizationProvider);
    final key = "organization-save-${request.feedbackId}";
    _active = request;
    state = LibraryOrganizationActivity(saving: true, sequence: state.sequence);
    LibraryOrganizationReceipt receipt;
    try {
      receipt = await request.execute(service);
    } on Object catch (error) {
      if (_disposed || request.isCancelled) {
        return const LibraryOrganizationRetired();
      }
      final code = error is LibraryOrganizationFailure
          ? error.code
          : "unexpected_error";
      debugPrint("[Ame organization] outcome=failed code=$code");
      final stale = error is LibraryOrganizationFailure && error.isStale;
      final undoUnavailable =
          error is LibraryOrganizationFailure &&
          error.code == "organization_undo_unavailable";
      final selectionExpired = code == "organization_selection_expired";
      final retryId = request.feedbackId;
      if (stale) {
        request.refreshAuthority();
      }
      final failure = LibraryOrganizationFailure(code, error.toString());
      if (request.errorSurface == LibraryOrganizationErrorSurface.form) {
        return LibraryOrganizationRejected(failure);
      }
      if (!undoUnavailable && !selectionExpired) {
        _failed[retryId] = request;
      }
      while (_failed.length > ameNotificationHistoryLimit) {
        _failed.remove(_failed.keys.first)?.cancel();
      }
      var message = "更改尚未确认保存，请重试。";
      if (undoUnavailable) {
        message = "图片或整理记录已更改，当前记录已保留。";
      } else if (stale) {
        message = "图片或整理信息已更新，请重试。";
      } else if (selectionExpired) {
        message = "图片或整理信息已更新，请重新选择图片。";
      }
      notifications.publish(
        AmeNotificationDraft(
          title: undoUnavailable ? "无法撤销" : "未能保存更改",
          message: message,
          severity: AmeNotificationSeverity.error,
          dedupeKey: key,
          actionId: undoUnavailable || selectionExpired
              ? null
              : "organization.retry",
          actionLabel: undoUnavailable || selectionExpired ? null : "重试",
          actionContext: undoUnavailable || selectionExpired ? null : retryId,
          isPersistent: true,
        ),
      );
      return LibraryOrganizationRejected(failure);
    } finally {
      if (identical(_active, request)) {
        _active = null;
      }
      if (!_disposed) {
        state = LibraryOrganizationActivity(sequence: state.sequence);
      }
    }
    if (_disposed) {
      return const LibraryOrganizationRetired();
    }
    _failed.removeWhere((_, pending) => identical(pending, request));
    notifications.resolve(key);
    state = LibraryOrganizationActivity(sequence: state.sequence + 1);
    ref
        .read(libraryOrganizationOutcomeProvider.notifier)
        .show(
          receipt,
          message: request.successMessage,
          canUndo: request.canUndo(receipt),
        );
    await _refresh();
    return LibraryOrganizationSaved(receipt);
  }

  Future<void> _refresh() async {
    var refreshed = false;
    try {
      refreshed = await ref
          .read(libraryControllerProvider.notifier)
          .refreshCurrentQuery();
    } on Object {
      refreshed = false;
    }
    if (_disposed) {
      return;
    }
    final notifications = ref.read(ameNotificationControllerProvider.notifier);
    if (refreshed) {
      notifications.resolve("organization-gallery-refresh");
    } else {
      notifications.publish(
        const AmeNotificationDraft(
          title: "更改已保存",
          message: "图库尚未刷新，请重试。",
          severity: AmeNotificationSeverity.warning,
          dedupeKey: "organization-gallery-refresh",
          actionId: "organization.refresh",
          actionLabel: "重试",
          isPersistent: true,
        ),
      );
    }
  }
}

final libraryOrganizationInspectionProvider = FutureProvider.autoDispose
    .family<LibraryOrganizationInspection, LibraryOrganizationTarget>((
      ref,
      target,
    ) {
      ref.watch(
        libraryOrganizationActionsProvider.select((state) => state.sequence),
      );
      ref.watch(
        libraryControllerProvider.select((state) => state.catalogRevision),
      );
      return ref.read(libraryOrganizationProvider).inspect([target]);
    });
