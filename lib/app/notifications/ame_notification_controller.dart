import "package:flutter_riverpod/flutter_riverpod.dart";

const ameNotificationHistoryLimit = 100;

enum AmeNotificationSeverity { info, success, warning, error }

class AmeNotificationDraft {
  const AmeNotificationDraft({
    required this.title,
    required this.message,
    required this.severity,
    this.dedupeKey,
    this.detail,
    this.elapsedStartedAt,
    this.sourcePath,
    this.technicalCode,
    this.actionId,
    this.actionLabel,
    this.actionContext,
    this.isPersistent = false,
  });

  final String title;
  final String message;
  final AmeNotificationSeverity severity;
  final String? dedupeKey;
  final String? detail;
  final DateTime? elapsedStartedAt;
  final String? sourcePath;
  final String? technicalCode;
  final String? actionId;
  final String? actionLabel;
  final String? actionContext;
  final bool isPersistent;
}

class AmeNotificationEntry {
  const AmeNotificationEntry({
    required this.id,
    required this.title,
    required this.message,
    required this.severity,
    required this.occurredAt,
    required this.isUnread,
    required this.isActive,
    required this.isPersistent,
    this.dedupeKey,
    this.detail,
    this.elapsedStartedAt,
    this.sourcePath,
    this.technicalCode,
    this.actionId,
    this.actionLabel,
    this.actionContext,
  });

  final String id;
  final String title;
  final String message;
  final AmeNotificationSeverity severity;
  final DateTime occurredAt;
  final bool isUnread;
  final bool isActive;
  final bool isPersistent;
  final String? dedupeKey;
  final String? detail;
  final DateTime? elapsedStartedAt;
  final String? sourcePath;
  final String? technicalCode;
  final String? actionId;
  final String? actionLabel;
  final String? actionContext;

  AmeNotificationEntry copyWith({
    String? title,
    String? message,
    AmeNotificationSeverity? severity,
    DateTime? occurredAt,
    bool? isUnread,
    bool? isActive,
    bool? isPersistent,
    Object? detail = _unchanged,
    Object? elapsedStartedAt = _unchanged,
    Object? sourcePath = _unchanged,
    Object? technicalCode = _unchanged,
    Object? actionId = _unchanged,
    Object? actionLabel = _unchanged,
    Object? actionContext = _unchanged,
  }) {
    return AmeNotificationEntry(
      id: id,
      title: title ?? this.title,
      message: message ?? this.message,
      severity: severity ?? this.severity,
      occurredAt: occurredAt ?? this.occurredAt,
      isUnread: isUnread ?? this.isUnread,
      isActive: isActive ?? this.isActive,
      isPersistent: isPersistent ?? this.isPersistent,
      dedupeKey: dedupeKey,
      detail: detail == _unchanged ? this.detail : detail as String?,
      elapsedStartedAt: elapsedStartedAt == _unchanged
          ? this.elapsedStartedAt
          : elapsedStartedAt as DateTime?,
      sourcePath: sourcePath == _unchanged
          ? this.sourcePath
          : sourcePath as String?,
      technicalCode: technicalCode == _unchanged
          ? this.technicalCode
          : technicalCode as String?,
      actionId: actionId == _unchanged ? this.actionId : actionId as String?,
      actionLabel: actionLabel == _unchanged
          ? this.actionLabel
          : actionLabel as String?,
      actionContext: actionContext == _unchanged
          ? this.actionContext
          : actionContext as String?,
    );
  }

  static const Object _unchanged = Object();
}

class AmeNotificationState {
  AmeNotificationState({
    List<AmeNotificationEntry> history = const [],
    List<String> pendingIds = const [],
  }) : history = List.unmodifiable(history),
       pendingIds = List.unmodifiable(pendingIds);

  final List<AmeNotificationEntry> history;
  final List<String> pendingIds;

  bool get hasUnread => history.any((notification) => notification.isUnread);

  AmeNotificationEntry? get current {
    if (pendingIds.isEmpty) {
      return null;
    }
    final currentId = pendingIds.first;
    for (final notification in history) {
      if (notification.id == currentId) {
        return notification;
      }
    }
    return null;
  }
}

final ameNotificationControllerProvider =
    NotifierProvider<AmeNotificationController, AmeNotificationState>(
      AmeNotificationController.new,
    );

class AmeNotificationController extends Notifier<AmeNotificationState> {
  final Set<String> _activeDedupeKeys = {};
  int _nextId = 1;

  @override
  AmeNotificationState build() => AmeNotificationState();

  String publish(AmeNotificationDraft draft, {DateTime? occurredAt}) {
    final timestamp = occurredAt ?? DateTime.now();
    final dedupeKey = draft.dedupeKey;
    if (dedupeKey != null && _activeDedupeKeys.contains(dedupeKey)) {
      final existingIndex = state.history.indexWhere(
        (notification) => notification.dedupeKey == dedupeKey,
      );
      if (existingIndex >= 0) {
        final existing = state.history[existingIndex];
        final history = state.history.toList();
        history[existingIndex] = existing.copyWith(
          title: draft.title,
          message: draft.message,
          severity: draft.severity,
          occurredAt: timestamp,
          isActive: true,
          isPersistent: draft.isPersistent,
          detail: draft.detail,
          elapsedStartedAt: draft.elapsedStartedAt,
          sourcePath: draft.sourcePath,
          technicalCode: draft.technicalCode,
          actionId: draft.actionId,
          actionLabel: draft.actionLabel,
          actionContext: draft.actionContext,
        );
        state = AmeNotificationState(
          history: history,
          pendingIds: state.pendingIds,
        );
        return existing.id;
      }
      return dedupeKey;
    }

    final id = "ame-notification-${_nextId++}";
    final notification = AmeNotificationEntry(
      id: id,
      title: draft.title,
      message: draft.message,
      severity: draft.severity,
      occurredAt: timestamp,
      isUnread: true,
      isActive: dedupeKey != null,
      isPersistent: draft.isPersistent,
      dedupeKey: dedupeKey,
      detail: draft.detail,
      elapsedStartedAt: draft.elapsedStartedAt,
      sourcePath: draft.sourcePath,
      technicalCode: draft.technicalCode,
      actionId: draft.actionId,
      actionLabel: draft.actionLabel,
      actionContext: draft.actionContext,
    );
    if (dedupeKey != null) {
      _activeDedupeKeys.add(dedupeKey);
    }
    final history = [notification, ...state.history];
    final retainedHistory = history.length <= ameNotificationHistoryLimit
        ? history
        : history.take(ameNotificationHistoryLimit).toList();
    final retainedIds = retainedHistory.map((entry) => entry.id).toSet();
    final retainedActiveKeys = retainedHistory
        .where((entry) => entry.isActive)
        .map((entry) => entry.dedupeKey)
        .whereType<String>()
        .toSet();
    _activeDedupeKeys.removeWhere(
      (dedupeKey) => !retainedActiveKeys.contains(dedupeKey),
    );
    state = AmeNotificationState(
      history: retainedHistory,
      pendingIds: [
        ...state.pendingIds,
        id,
      ].where(retainedIds.contains).toList(),
    );
    return id;
  }

  bool resolve(String dedupeKey) {
    if (!_activeDedupeKeys.remove(dedupeKey)) {
      return false;
    }
    final history = [
      for (final notification in state.history)
        if (notification.dedupeKey == dedupeKey)
          notification.copyWith(isActive: false)
        else
          notification,
    ];
    final resolvedIds = history
        .where(
          (notification) =>
              notification.dedupeKey == dedupeKey && !notification.isActive,
        )
        .map((notification) => notification.id)
        .toSet();
    state = AmeNotificationState(
      history: history,
      pendingIds: state.pendingIds
          .where((id) => !resolvedIds.contains(id))
          .toList(),
    );
    return true;
  }

  void dismiss(String notificationId) {
    if (!state.pendingIds.contains(notificationId)) {
      return;
    }
    state = AmeNotificationState(
      history: state.history,
      pendingIds: state.pendingIds.where((id) => id != notificationId).toList(),
    );
  }

  void markAllRead() {
    if (!state.hasUnread) {
      return;
    }
    state = AmeNotificationState(
      history: [
        for (final notification in state.history)
          notification.isUnread
              ? notification.copyWith(isUnread: false)
              : notification,
      ],
      pendingIds: state.pendingIds,
    );
  }
}
