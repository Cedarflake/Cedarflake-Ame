import "dart:math" as math;

import "package:flutter/foundation.dart";

import "../domain/library_duplicate_review.dart";

class LibraryDuplicateReviewSlice<T, C> {
  const LibraryDuplicateReviewSlice({
    required this.items,
    required this.total,
    this.next,
    this.decision,
  });
  final List<T> items;
  final int total;
  final C? next;
  final LibraryDuplicateDecision? decision;
}

class LibraryDuplicateReviewSeek<T, C> {
  const LibraryDuplicateReviewSeek({
    required this.startIndex,
    required this.slice,
  });
  final int startIndex;
  final LibraryDuplicateReviewSlice<T, C> slice;
}

/// Keeps bounded payloads and keyset checkpoints behind a continuous viewport.
class LibraryDuplicateReviewWindow<T, C> extends ChangeNotifier {
  LibraryDuplicateReviewWindow(this._load, {this._seek, this.onAdmitted});
  final Future<LibraryDuplicateReviewSlice<T, C>> Function(C?) _load;
  final Future<LibraryDuplicateReviewSeek<T, C>> Function(int)? _seek;
  final void Function(LibraryDuplicateReviewSlice<T, C>)? onAdmitted;
  static const maxCachedPages = 6;
  static const maxCheckpoints = 64;
  var _data = _ReviewWindowData<T, C>();
  int get total => _data.total;
  set total(int value) => _data.total = value;
  bool loading = false;
  bool failed = false;
  bool _disposed = false;
  int _generation = 0;
  int _first = 0;
  int _end = 1;
  Future<void>? _pending;

  T? itemAt(int index) => _data.itemAt(index);

  List<T> get loadedItems {
    final starts = _data.pages.keys.toList()..sort();
    return [for (final start in starts) ..._data.pages[start]!];
  }

  int get cachedItemCount =>
      _data.pages.values.fold(0, (count, items) => count + items.length);
  int get checkpointCount => _data.checkpoints.length;

  void invalidate({bool clear = false}) {
    cancelRead();
    _data = _ReviewWindowData<T, C>()..total = clear ? 0 : total;
    failed = false;
    if (clear) {
      total = 0;
      _first = 0;
      _end = 1;
    }
  }

  void cancelRead() {
    _generation++;
    _pending = null;
    loading = false;
  }

  Future<void> refresh() {
    invalidate();
    return ensureRange(_first, _end);
  }

  Future<void> revalidate() {
    if (_disposed) return Future.value();
    if (_data.pages.isEmpty) return refresh();
    cancelRead();
    failed = false;
    // Keep the published viewport until its replacement range is ready.
    final replacement = _ReviewWindowData<T, C>()..total = total;
    return _pending = _drain(_generation, replacement);
  }

  Future<void> retryRange(int first, int end) {
    failed = false;
    return ensureRange(first, end);
  }

  Future<void> ensureRange(int first, int end) {
    if (_disposed) return Future.value();
    _first = math.max(0, first);
    _end = math.max(_first + 1, math.min(end, _first + 128));
    if (failed) return Future.value();
    final wasLoading = loading;
    loading = _missing(_data) != null;
    if (_pending != null && loading != wasLoading) notifyListeners();
    return _pending ??= _drain(_generation, _data);
  }

  int? _missing(_ReviewWindowData<T, C> data) {
    if (data.pages.isEmpty) {
      return math.min(_first, math.max(0, data.total - 1));
    }
    for (var index = _first; index < math.min(_end, data.total); index++) {
      if (data.itemAt(index) == null) return index;
    }
    return null;
  }

  Future<void> _drain(int generation, _ReviewWindowData<T, C> data) async {
    // Defer notifications until the caller has recorded the pending request.
    await Future<void>.value();
    if (_disposed || generation != _generation) return;
    if (_missing(data) == null) {
      _pending = null;
      return;
    }
    final replacing = !identical(data, _data);
    if (!replacing) {
      loading = true;
      notifyListeners();
    }
    LibraryDuplicateReviewSlice<T, C>? latest;
    try {
      while (!_disposed && generation == _generation) {
        final missing = _missing(data);
        if (missing == null) break;
        var start = data.checkpoints.keys
            .where((offset) => offset <= missing)
            .reduce(math.max);
        final seek = _seek;
        final LibraryDuplicateReviewSlice<T, C> slice;
        if (seek != null && start < missing) {
          final range = await seek(missing);
          start = range.startIndex;
          slice = range.slice;
          if (start < 0 ||
              start > missing ||
              (start + slice.items.length <= missing &&
                  slice.total > missing)) {
            throw StateError(
              "Duplicate review seek missed its requested range",
            );
          }
        } else {
          slice = await _load(data.checkpoints[start]);
        }
        if (_disposed || generation != _generation) return;
        if (slice.items.isEmpty && slice.next != null) {
          throw StateError("Duplicate review cursor did not advance");
        }
        data.pages[start] = slice.items;
        final nextStart = start + slice.items.length;
        data.total = slice.next == null
            ? nextStart
            : math.max(slice.total, nextStart + 1);
        if (slice.next case final next?) data.checkpoints[nextStart] = next;
        latest = slice;
        _trim(data);
        if (!replacing) {
          onAdmitted?.call(slice);
          notifyListeners();
        }
        if (data.total == 0) break;
      }
      if (!_disposed && generation == _generation && replacing) {
        _data = data;
        if (latest != null) onAdmitted?.call(latest);
      }
    } on Object {
      if (!_disposed && generation == _generation) failed = true;
    } finally {
      if (!_disposed && generation == _generation) {
        loading = false;
        _pending = null;
        notifyListeners();
      }
    }
  }

  void _trim(_ReviewWindowData<T, C> data) {
    final center = (_first + _end) / 2;
    while (data.pages.length > maxCachedPages) {
      final farthest = data.pages.keys.reduce(
        (a, b) => (a - center).abs() > (b - center).abs() ? a : b,
      );
      data.pages.remove(farthest);
    }
    while (data.checkpoints.length > maxCheckpoints) {
      final farthest = data.checkpoints.keys
          .where((offset) => offset != 0)
          .reduce((a, b) => (a - center).abs() > (b - center).abs() ? a : b);
      data.checkpoints.remove(farthest);
    }
  }

  @override
  void dispose() {
    _disposed = true;
    cancelRead();
    super.dispose();
  }
}

class _ReviewWindowData<T, C> {
  final pages = <int, List<T>>{};
  final checkpoints = <int, C?>{0: null};
  int total = 0;

  T? itemAt(int index) {
    for (final entry in pages.entries) {
      final offset = index - entry.key;
      if (offset >= 0 && offset < entry.value.length) {
        return entry.value[offset];
      }
    }
    return null;
  }
}
