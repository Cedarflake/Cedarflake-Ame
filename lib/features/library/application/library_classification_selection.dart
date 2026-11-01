import "dart:async";

import "package:flutter/foundation.dart";

import "../domain/library_classification_review.dart";
import "library_classification_review.dart";

class LibraryClassificationSelection {
  LibraryClassificationSelection(this.service, this.scope);
  final LibraryClassificationReviewService service;
  final LibraryClassificationScope scope;
  LibraryClassificationDraft? _draft;
  Future<LibraryClassificationSummary>? _preparing;
  bool _closed = false;
  LibraryClassificationSummary? summary;

  void _requireActive() {
    if (_closed) {
      throw const LibraryClassificationFailure(
        "classification_selection_cancelled",
        "The selection has been retired",
      );
    }
  }

  Future<LibraryClassificationSummary> prepare() {
    _requireActive();
    return _preparing ??= _prepare();
  }

  Future<LibraryClassificationSummary> _prepare() async {
    LibraryClassificationDraft? opened;
    try {
      opened = await service.open(scope);
      _requireActive();
      _draft = opened;
      if (scope.targets.isNotEmpty) await opened.appendTargets(scope.targets);
      final ids = scope.assetIds;
      for (var offset = 0; offset < ids.length; offset += 256) {
        _requireActive();
        await opened.append(
          ids.sublist(offset, (offset + 256).clamp(0, ids.length)),
        );
      }
      _requireActive();
      final result = await opened.prepare();
      _requireActive();
      return summary = result;
    } on Object {
      if (identical(_draft, opened)) _draft = null;
      if (opened != null) await _close(opened);
      _preparing = null;
      rethrow;
    }
  }

  Future<LibraryClassificationReceipt> apply(
    String operationId,
    LibraryClassificationCommand command,
  ) async {
    await prepare();
    _requireActive();
    final draft = _draft;
    if (draft == null) {
      throw StateError("Classification selection has not been prepared");
    }
    return draft.apply(operationId, command);
  }

  void dispose() {
    if (_closed) return;
    _closed = true;
    final draft = _draft;
    _draft = null;
    if (draft != null) unawaited(_close(draft));
  }

  Future<void> _close(LibraryClassificationDraft draft) async {
    try {
      await draft.close();
    } on Object catch (error) {
      debugPrint("[Ame classification] selection close failed: $error");
    }
  }
}
