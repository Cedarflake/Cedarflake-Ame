import "dart:async";

import "library_catalog_publication.dart";
import "library_query_projection.dart";
import "library_query_update.dart";

export "library_query_update.dart";

class _ActiveQueryAttempt {
  _ActiveQueryAttempt(this.generation);

  final int generation;
  final Completer<LibraryQueryUpdateOutcome> completion = Completer();
}

/// User queries may replace a projection; committed refresh obligations survive
/// that replacement and resume against the user's latest published query.
class LibraryQueryRefreshCoordinator {
  LibraryQueryRefreshCoordinator(
    this._publications, [
    LibraryQueryProjections? projections,
  ]) : _projections = projections ?? LibraryQueryProjections();

  final LibraryCatalogPublicationCoordinator _publications;
  final LibraryQueryProjections _projections;
  _ActiveQueryAttempt? _active;
  final Set<_ActiveQueryAttempt> _inFlight = {};
  Future<void> _committedTail = Future<void>.value();
  int _committedRequests = 0;
  int _generation = 0;
  bool _isDisposed = false;

  Future<LibraryQueryUpdateOutcome> runUser(LibraryQueryAttempt attempt) =>
      _start(attempt).completion.future;

  Future<LibraryQueryUpdateOutcome> runPassive(LibraryQueryAttempt attempt) {
    if (_isDisposed) {
      return Future.value(LibraryQueryUpdateOutcome.superseded);
    }
    if (_active != null || _committedRequests != 0) {
      return Future.value(LibraryQueryUpdateOutcome.busy);
    }
    return _start(attempt).completion.future;
  }

  Future<bool> refreshCommitted(LibraryQueryAttempt attempt) async {
    _committedRequests += 1;
    final previous = _committedTail;
    final released = Completer<void>();
    _committedTail = released.future;
    try {
      await previous;
      while (!_isDisposed) {
        final active = _active;
        if (active != null) {
          await active.completion.future;
          continue;
        }
        if (!await _publications.waitUntilAvailable() || _isDisposed) {
          return false;
        }
        if (_active != null || _publications.isReserved) {
          continue;
        }
        final publicationGeneration = _publications.generation;
        final positionGeneration = _projections.generation;
        final request = _start(attempt);
        final outcome = await request.completion.future;
        if (outcome == LibraryQueryUpdateOutcome.applied) {
          return true;
        }
        final wasReplaced =
            request.generation != _generation ||
            publicationGeneration != _publications.generation ||
            positionGeneration != _projections.generation;
        if (wasReplaced &&
            (outcome == LibraryQueryUpdateOutcome.superseded ||
                outcome == LibraryQueryUpdateOutcome.busy)) {
          continue;
        }
        return false;
      }
      return false;
    } finally {
      _committedRequests -= 1;
      released.complete();
    }
  }

  void invalidate() => _generation += 1;

  void dispose() {
    _isDisposed = true;
    invalidate();
    _active = null;
    for (final request in _inFlight) {
      if (!request.completion.isCompleted) {
        request.completion.complete(LibraryQueryUpdateOutcome.superseded);
      }
    }
    _inFlight.clear();
  }

  _ActiveQueryAttempt _start(LibraryQueryAttempt attempt) {
    final request = _ActiveQueryAttempt(++_generation);
    if (_isDisposed) {
      request.completion.complete(LibraryQueryUpdateOutcome.superseded);
      return request;
    }
    _active = request;
    _inFlight.add(request);
    unawaited(_execute(request, attempt));
    return request;
  }

  Future<void> _execute(
    _ActiveQueryAttempt request,
    LibraryQueryAttempt attempt,
  ) async {
    try {
      final outcome = await attempt();
      if (!request.completion.isCompleted) {
        request.completion.complete(outcome);
      }
    } on Object catch (error, stackTrace) {
      if (!request.completion.isCompleted) {
        request.completion.completeError(error, stackTrace);
      }
    } finally {
      _inFlight.remove(request);
      if (identical(_active, request)) {
        _active = null;
      }
    }
  }
}
