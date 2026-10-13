import "dart:async";

import "../domain/library_models.dart";

enum LibraryPageDirection { next, previous }

class _PageFlight {
  _PageFlight(this.generation, this.direction, this.cursor);

  final int generation;
  final LibraryPageDirection direction;
  final LibraryCatalogCursor? cursor;
  final Completer<bool> completion = Completer<bool>();
  final Completer<void> settled = Completer<void>();
}

/// Joins cursor reads and serializes opposite directions within one query epoch.
class LibraryPageOperation {
  final Map<LibraryPageDirection, _PageFlight> _pending = {};
  _PageFlight? _tail;
  int _generation = -1;

  Future<bool> run(
    int generation,
    LibraryCatalogCursor? cursor,
    Future<bool> Function() load, {
    LibraryPageDirection direction = LibraryPageDirection.next,
  }) {
    if (generation < _generation) {
      return Future.value(false);
    }
    if (generation != _generation) {
      _generation = generation;
      _pending.clear();
      _tail = null;
    }
    final active = _pending[direction];
    if (active != null && identical(active.cursor, cursor)) {
      return active.completion.future;
    }
    // Dependencies point backward in registration order, so waits cannot cycle.
    final predecessor = _tail;
    final request = _PageFlight(generation, direction, cursor);
    _pending[direction] = request;
    _tail = request;
    unawaited(_finish(request, predecessor, load));
    return request.completion.future;
  }

  Future<void> _finish(
    _PageFlight request,
    _PageFlight? predecessor,
    Future<bool> Function() load,
  ) async {
    try {
      if (predecessor != null) {
        await predecessor.settled.future;
      }
      if (request.generation != _generation ||
          !identical(_pending[request.direction], request)) {
        request.completion.complete(false);
        return;
      }
      request.completion.complete(await load());
    } on Object catch (error, stackTrace) {
      request.completion.completeError(error, stackTrace);
    } finally {
      if (identical(_pending[request.direction], request)) {
        _pending.remove(request.direction);
      }
      if (identical(_tail, request)) {
        _tail = null;
      }
      request.settled.complete();
    }
  }
}
