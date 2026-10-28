import "package:flutter/foundation.dart";

import "library_sidebar_order.dart";

class LibrarySidebarOrdering extends ChangeNotifier {
  LibrarySidebarOrdering({required List<String> ids, required this._save})
    : _ids = List.of(ids),
      _sourceIds = List.of(ids);

  final Future<void> Function(LibrarySidebarMove move) _save;
  List<String> _ids;
  List<String> _sourceIds;
  bool _hasLocalOrder = false;
  bool _saving = false;
  bool _disposed = false;

  List<String> get ids => List.unmodifiable(_ids);
  bool get saving => _saving;

  void updateItems(List<String> ids) {
    if (listEquals(_sourceIds, ids)) return;
    _sourceIds = List.of(ids);
    _ids = _hasLocalOrder ? _reconcile(_ids) : List.of(ids);
  }

  List<String> _reconcile(List<String> ordered) {
    final current = _sourceIds.toSet();
    final retained = ordered.where(current.contains).toList();
    final known = retained.toSet();
    return [...retained, ..._sourceIds.where((id) => !known.contains(id))];
  }

  Future<bool> move(int oldIndex, int newIndex) async {
    if (_saving || _disposed || oldIndex == newIndex) return false;
    if (oldIndex < 0 ||
        newIndex < 0 ||
        oldIndex >= _ids.length ||
        newIndex >= _ids.length) {
      return false;
    }
    final previous = List<String>.of(_ids);
    final previousLocalOrder = _hasLocalOrder;
    final move = LibrarySidebarMove(
      entryId: _ids[oldIndex],
      anchorId: _ids[newIndex],
      placement: oldIndex < newIndex
          ? LibrarySidebarPlacement.after
          : LibrarySidebarPlacement.before,
    );
    final entry = _ids.removeAt(oldIndex);
    _ids.insert(newIndex, entry);
    _hasLocalOrder = true;
    _saving = true;
    notifyListeners();
    try {
      await _save(move);
      return true;
    } catch (_) {
      if (!_disposed) {
        _ids = _reconcile(previous);
        _hasLocalOrder = previousLocalOrder;
      }
      rethrow;
    } finally {
      if (!_disposed) {
        _saving = false;
        notifyListeners();
      }
    }
  }

  @override
  void dispose() {
    _disposed = true;
    super.dispose();
  }
}
