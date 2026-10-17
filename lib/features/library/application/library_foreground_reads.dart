import "dart:async";

import "package:flutter/foundation.dart";

/// Keeps background analysis idle until all foreground media reads release ownership.
class LibraryForegroundReads extends ChangeNotifier {
  int _active = 0;
  bool _notificationPending = false;
  bool _disposed = false;

  bool get isBusy => _active > 0;

  LibraryForegroundReadLease acquire() {
    _active++;
    if (_active == 1) {
      _notifyChanged();
    }
    return LibraryForegroundReadLease._(() {
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

class LibraryForegroundReadLease {
  LibraryForegroundReadLease._(this._release);
  void Function()? _release;

  void close() {
    final release = _release;
    _release = null;
    release?.call();
  }
}

// Native previews and viewer reads share the same process-wide I/O priority.
final libraryForegroundReads = LibraryForegroundReads();
