import "package:flutter/foundation.dart";

class LibrarySidebarRemovalEntry {
  const LibrarySidebarRemovalEntry(this.id, this.name);
  final String id;
  final String name;
}

class LibrarySidebarRemovalOutcome {
  const LibrarySidebarRemovalOutcome({required this.committed, this.error});
  final bool committed;
  final String? error;
}

/// Stops between guarded commands; a committed item is never submitted again.
class LibrarySidebarRemoval extends ChangeNotifier {
  LibrarySidebarRemoval({
    required List<LibrarySidebarRemovalEntry> entries,
    required this.remove,
  }) : entries = List.unmodifiable(entries);

  final List<LibrarySidebarRemovalEntry> entries;
  final Future<LibrarySidebarRemovalOutcome> Function(String id) remove;
  final Set<String> _completed = {};
  bool _disposed = false;
  bool _stopping = false;
  bool _running = false;
  String? _error;

  Set<String> get completed => Set.unmodifiable(_completed);
  bool get isRunning => _running;
  bool get isStopping => _stopping;
  bool get isComplete => _completed.length == entries.length;
  String? get error => _error;

  Future<void> run() async {
    if (_disposed || _running || isComplete) return;
    _running = true;
    _stopping = false;
    _error = null;
    notifyListeners();
    try {
      for (final entry in entries) {
        if (_disposed || _stopping) break;
        if (_completed.contains(entry.id)) continue;
        final outcome = await remove(entry.id);
        if (outcome.committed) _completed.add(entry.id);
        _error = outcome.error;
        if (_disposed) break;
        notifyListeners();
        if (!outcome.committed || _error != null) break;
      }
    } on Object {
      _error = "未能完成操作，请重试。";
    } finally {
      _running = false;
      if (!_disposed) notifyListeners();
    }
  }

  void stop() {
    if (_disposed || !_running) return;
    _stopping = true;
    notifyListeners();
  }

  @override
  void dispose() {
    if (_disposed) return;
    _disposed = true;
    super.dispose();
  }
}
