import "dart:async";

import "package:flutter/foundation.dart";

import "../domain/library_organization.dart";
import "../domain/library_organization_batch.dart";
import "library_organization_batch.dart";

class LibrarySelectionSession {
  LibrarySelectionSession(this.service, this.selection);
  final LibraryOrganizationBatches service;
  final LibraryOrganizationSelection selection;
  LibraryOrganizationBatch? _batch;
  Future<LibraryBatchSummary>? _preparing;
  bool _closed = false;
  LibraryBatchSummary? summary;
  bool hasCommitted = false;

  void _requireActive() {
    if (_closed) {
      throw const LibraryOrganizationFailure(
        "organization_selection_cancelled",
        "The selection has been retired",
      );
    }
  }

  Future<LibraryBatchSummary> prepare() {
    _requireActive();
    return _preparing ??= _prepare();
  }

  Future<LibraryBatchSummary> _prepare() async {
    try {
      final batch = await service.open(selection);
      if (_closed) {
        await batch.close();
        _requireActive();
      }
      _batch = batch;
      final ids = selection.assetIds;
      for (var start = 0; start < ids.length; start += 256) {
        _requireActive();
        await batch.append(
          ids.sublist(start, (start + 256).clamp(0, ids.length)),
        );
      }
      _requireActive();
      final result = await batch.prepare();
      _requireActive();
      return summary = result;
    } on Object {
      final batch = _batch;
      _batch = null;
      if (batch != null) await _close(batch);
      _preparing = null;
      rethrow;
    }
  }

  Future<void> refresh() async {
    _requireActive();
    final batch = _batch;
    if (batch == null) throw StateError("The selection has not been prepared");
    final result = await batch.refresh();
    _requireActive();
    summary = result;
  }

  Future<LibraryOrganizationReceipt> apply(
    String operationId,
    LibraryBatchAction action,
  ) async {
    await prepare();
    _requireActive();
    final receipt = await _batch!.apply(operationId, action);
    hasCommitted = true;
    return receipt;
  }

  Future<LibraryBatchProgress?> progress() async {
    if (_closed) return null;
    return _batch?.progress();
  }

  void dispose() {
    if (_closed) return;
    _closed = true;
    final batch = _batch;
    _batch = null;
    if (batch != null) unawaited(_close(batch));
  }

  Future<void> _close(LibraryOrganizationBatch batch) async {
    try {
      await batch.close();
    } on Object catch (error) {
      debugPrint("[Ame organization] selection close failed: $error");
    }
  }
}
