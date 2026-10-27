import "dart:async";

import "package:flutter/foundation.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";

import "../../../app/notifications/ame_notification_controller.dart";
import "../domain/library_organization.dart";
import "library_controller.dart";
import "library_organization.dart";
import "library_organization_failure_feedback.dart";
import "library_organization_outcome.dart";
import "library_organization_publication.dart";
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
        unawaited(ref.read(libraryOrganizationPublicationProvider).refresh());
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
      final feedback = LibraryOrganizationFailureFeedback(error, request);
      final failure = feedback.failure;
      final code = failure.code;
      debugPrint("[Ame organization] outcome=failed code=$code");
      final retryId = request.feedbackId;
      if (failure.isStale) {
        request.refreshAuthority();
      }
      if (request.errorSurface == LibraryOrganizationErrorSurface.form) {
        return LibraryOrganizationRejected(failure);
      }
      if (feedback.canRetry) {
        _failed[retryId] = request;
      } else {
        _failed.remove(retryId);
      }
      while (_failed.length > ameNotificationHistoryLimit) {
        _failed.remove(_failed.keys.first)?.cancel();
      }
      notifications.publish(
        AmeNotificationDraft(
          title: feedback.title,
          message: feedback.message,
          severity: AmeNotificationSeverity.error,
          dedupeKey: key,
          actionId: feedback.canRetry ? "organization.retry" : null,
          actionLabel: feedback.canRetry ? "重试" : null,
          actionContext: feedback.canRetry ? retryId : null,
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
    unawaited(
      ref.read(libraryOrganizationPublicationProvider).refresh(request.change),
    );
    return LibraryOrganizationSaved(receipt);
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
