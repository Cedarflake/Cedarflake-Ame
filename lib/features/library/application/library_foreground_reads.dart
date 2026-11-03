import "dart:async";

import "package:flutter/foundation.dart";

import "ports/library_foreground_read_activity.dart";

/// Keeps background analysis idle until all foreground media reads release ownership.
class LibraryForegroundReads extends ChangeNotifier
    implements LibraryForegroundReadActivity {
  int _active = 0;
  bool _notificationPending = false;
  bool _disposed = false;

  bool get isBusy => _active > 0;

  @override
  LibraryForegroundReadLease acquire() {
    _active++;
    if (_active == 1) {
      _notifyChanged();
    }
    return _LibraryForegroundReadLease(() {
      _active--;
      if (_active == 0) {
        _notifyChanged();
      }
    });
  }

  void _notifyChanged() {
    if (_notificationPending || _disposed) {
      return;
    }
    _notificationPending = true;
    // Image providers may start a source read while Flutter is building a frame.
    scheduleMicrotask(() {
      _notificationPending = false;
      if (!_disposed) {
        notifyListeners();
      }
    });
  }

  @override
  void dispose() {
    _disposed = true;
    super.dispose();
  }
}

class _LibraryForegroundReadLease implements LibraryForegroundReadLease {
  _LibraryForegroundReadLease(this._release);
  void Function()? _release;

  @override
  void close() {
    final release = _release;
    _release = null;
    release?.call();
  }
}

// Native previews and viewer reads share the same process-wide I/O priority.
final libraryForegroundReads = LibraryForegroundReads();
