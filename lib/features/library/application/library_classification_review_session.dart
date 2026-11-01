import "../domain/library_classification_progress.dart";
import "../domain/library_classification_review.dart";
import "../domain/library_models.dart";
import "../domain/library_organization.dart";
import "library_classification_progress.dart";

class LibraryClassificationReviewEntry {
  LibraryClassificationReviewEntry._(this.owner, this.target);
  final LibraryClassificationReviewSession owner;
  final LibraryOrganizationTarget target;
  Future<LibraryClassificationPosition>? _checkpoint;
  LibraryClassificationPosition? _requested;
  LibraryClassificationPosition? _committedPosition;
  bool get committed => _committedPosition != null;
}

/// Keeps the active review position separate from refreshed classification drafts.
class LibraryClassificationReviewSession {
  LibraryClassificationReviewSession(this.service, this.query);
  final LibraryClassificationProgress service;
  final LibraryGalleryQuery query;
  Future<LibraryClassificationPosition>? _begin;
  Future<void>? _closing;
  BigInt _sequence = BigInt.zero;
  LibraryClassificationReviewEntry? _current;
  bool _closed = false;

  LibraryClassificationReviewEntry open(LibraryOrganizationTarget target) {
    final entry = LibraryClassificationReviewEntry._(this, target);
    _current = entry;
    return entry;
  }

  void requireCurrent(LibraryClassificationReviewEntry entry) {
    if (_closed || !identical(_current, entry)) {
      throw const LibraryClassificationFailure(
        "classification_selection_cancelled",
        "The classification image changed",
      );
    }
  }

  Future<LibraryClassificationPosition> checkpoint(
    LibraryClassificationReviewEntry entry,
  ) {
    requireCurrent(entry);
    if (entry._committedPosition case final position?) {
      return Future.value(position);
    }
    return entry._checkpoint ??= _checkpoint(entry).onError((
      Object error,
      StackTrace stack,
    ) {
      entry._checkpoint = null;
      Error.throwWithStackTrace(error, stack);
    });
  }

  Future<LibraryClassificationPosition> _checkpoint(
    LibraryClassificationReviewEntry entry,
  ) async {
    final start = await (_begin ??= service.begin(query).onError((
      Object error,
      StackTrace stack,
    ) {
      _begin = null;
      Error.throwWithStackTrace(error, stack);
    }));
    requireCurrent(entry);
    if (entry._requested == null) {
      // Reserve the intervening sequence for a command already in flight.
      _sequence += BigInt.two;
      entry._requested = LibraryClassificationPosition(
        start.sessionId,
        _sequence,
      );
    }
    final position = entry._requested!;
    await service.checkpoint(position, entry.target);
    requireCurrent(entry);
    return position;
  }

  void accept(
    LibraryClassificationReviewEntry entry,
    LibraryClassificationProgressReceipt receipt,
  ) {
    if (receipt.position.sequence > _sequence) {
      _sequence = receipt.position.sequence;
    }
    entry._committedPosition = receipt.position;
  }

  Future<void> close() => _closing ??= _close();

  Future<void> _close() async {
    _closed = true;
    _current = null;
    final pending = _begin;
    if (pending == null) return;
    LibraryClassificationPosition position;
    try {
      position = await pending;
    } on Object {
      return;
    }
    await service.end(position.sessionId);
  }
}
