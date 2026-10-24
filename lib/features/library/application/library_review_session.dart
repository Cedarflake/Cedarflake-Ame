import "../domain/library_models.dart";
import "../domain/library_organization.dart";
import "../domain/library_review.dart";
import "library_review.dart";

class LibraryReviewEntry {
  LibraryReviewEntry._(this.target);
  final LibraryOrganizationTarget target;
  LibraryReviewPosition? _requested;
  Future<LibraryReviewPosition>? _checkpoint;
  bool _committed = false;
  bool get committed => _committed;
}

/// Owns one queue's save authority, independently of gallery refreshes.
class LibraryReviewSession {
  LibraryReviewSession(this.service, this.query);
  final LibraryReview service;
  final LibraryGalleryQuery query;
  Future<LibraryReviewPosition>? _begin;
  Future<void>? _closing;
  BigInt _sequence = BigInt.zero;
  LibraryReviewEntry? _current;
  bool _closed = false;

  LibraryReviewEntry? get current => _current;

  LibraryReviewEntry open(LibraryOrganizationTarget target) {
    final entry = LibraryReviewEntry._(target);
    _current = entry;
    return entry;
  }

  bool owns(LibraryReviewEntry entry) => !_closed && identical(_current, entry);

  void clearCurrent() => _current = null;

  void requireCurrent(LibraryReviewEntry entry) {
    if (!owns(entry)) {
      throw const LibraryOrganizationFailure(
        "organization_review_changed",
        "The current review image changed",
      );
    }
  }

  Future<LibraryReviewPosition> checkpoint(LibraryReviewEntry entry) {
    requireCurrent(entry);
    return entry._checkpoint ??= _checkpoint(entry).onError((
      Object error,
      StackTrace stack,
    ) {
      entry._checkpoint = null;
      Error.throwWithStackTrace(error, stack);
    });
  }

  Future<LibraryReviewPosition> _checkpoint(LibraryReviewEntry entry) async {
    final start = await (_begin ??= service.begin(query).onError((
      Object error,
      StackTrace stack,
    ) {
      _begin = null;
      Error.throwWithStackTrace(error, stack);
    }));
    requireCurrent(entry);
    if (entry._requested == null) {
      // Reserve the intervening sequence for a commit still in flight.
      _sequence += BigInt.two;
      entry._requested = LibraryReviewPosition(start.sessionId, _sequence);
    }
    final saved = await service.checkpoint(entry._requested!, entry.target);
    requireCurrent(entry);
    return saved;
  }

  void accept(LibraryReviewEntry entry, LibraryReviewReceipt receipt) {
    if (receipt.position.sequence > _sequence) {
      _sequence = receipt.position.sequence;
    }
    entry._committed = true;
  }

  Future<void> close() => _closing ??= _close();

  Future<void> _close() async {
    _closed = true;
    _current = null;
    final pending = _begin;
    if (pending == null) return;
    LibraryReviewPosition position;
    try {
      position = await pending;
    } on Object {
      return;
    }
    await service.end(position.sessionId);
  }
}
