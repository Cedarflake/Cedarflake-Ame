import "package:flutter/foundation.dart";

import "../domain/library_synchronization_models.dart";
import "ports/library_synchronization_gateway.dart";

class LibrarySynchronizationDiagnostics {
  LibrarySynchronizationDiagnostics({
    required this._enabled,
    required this._now,
  });

  final bool _enabled;
  final DateTime Function() _now;
  DateTime? _lastDebugSnapshotAt;
  Map<String, String> _lastDebugRootSignatures = const {};

  void failure({
    required String phase,
    required String errorCode,
    required Object error,
    required Duration elapsed,
    required bool isTransient,
    required int attempt,
  }) {
    if (!_enabled) {
      return;
    }
    final detail = switch (error) {
      LibrarySynchronizationFailure(:final message) => message.replaceAll(
        RegExp(r"[\r\n]+"),
        " ",
      ),
      _ => error.toString(),
    };
    debugPrint(
      "[Ame sync] phase=$phase result=${isTransient ? "retrying" : "failed"} "
      "elapsed_ms=${elapsed.inMilliseconds} code=$errorCode "
      "attempt=$attempt message=$detail",
    );
  }

  void snapshot(
    String phase,
    LibrarySynchronizationSnapshot snapshot,
    Duration elapsed,
  ) {
    if (!_enabled) {
      return;
    }
    final now = _now();
    final signatures = {
      for (final entry in snapshot.roots.entries)
        entry.key:
            "${entry.value.freshness.name}:${entry.value.continuity.name}:"
            "${entry.value.sourceStatus.name}:"
            "${entry.value.phase.name}:${entry.value.lastIssueCode ?? "-"}",
    };
    final hasTransition =
        signatures.length != _lastDebugRootSignatures.length ||
        signatures.entries.any(
          (entry) => _lastDebugRootSignatures[entry.key] != entry.value,
        );
    final shouldReportHeartbeat =
        _lastDebugSnapshotAt == null ||
        now.difference(_lastDebugSnapshotAt!) >= const Duration(seconds: 5);
    if (!hasTransition &&
        !shouldReportHeartbeat &&
        elapsed.inMilliseconds < 500) {
      return;
    }
    _lastDebugRootSignatures = signatures;
    _lastDebugSnapshotAt = now;
    if (snapshot.roots.isEmpty) {
      debugPrint(
        "[Ame sync] phase=$phase elapsed_ms=${elapsed.inMilliseconds} "
        "running=${snapshot.isRunning} roots=0 code=${snapshot.lastErrorCode ?? "-"}",
      );
      return;
    }
    for (final status in snapshot.roots.values) {
      final root = status.rootId.length <= 8
          ? status.rootId
          : status.rootId.substring(0, 8);
      final phaseElapsed = now.difference(status.phaseStartedAt);
      debugPrint(
        "[Ame sync] phase=$phase elapsed_ms=${elapsed.inMilliseconds} "
        "root=$root freshness=${status.freshness.name} "
        "continuity=${status.continuity.name} "
        "root_phase=${status.phase.name} "
        "root_phase_elapsed_ms=${phaseElapsed.isNegative ? 0 : phaseElapsed.inMilliseconds} "
        "source=${status.sourceStatus.name} pending=${status.pendingChangeCount} "
        "retry=${status.retryWaitCount} gaps=${status.freshnessUnknownCount} "
        "code=${status.lastIssueCode ?? snapshot.lastErrorCode ?? "-"}",
      );
    }
  }
}
