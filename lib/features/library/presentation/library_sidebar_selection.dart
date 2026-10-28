class LibrarySidebarSelection {
  final Set<String> _selected = {};

  Set<String> get ids => Set.unmodifiable(_selected);
  bool get isActive => _selected.isNotEmpty;
  int get count => _selected.length;
  bool contains(String id) => _selected.contains(id);

  void toggle(String id) {
    if (!_selected.remove(id)) _selected.add(id);
  }

  void retain(Iterable<String> ids) => _selected.retainAll(ids);
  void removeAll(Iterable<String> ids) => _selected.removeAll(ids);
  void clear() => _selected.clear();

  bool allSelected(Iterable<String> ids) =>
      ids.isNotEmpty && ids.every(_selected.contains);

  void toggleAll(Iterable<String> ids) {
    if (allSelected(ids)) {
      _selected.removeAll(ids);
    } else {
      _selected.addAll(ids);
    }
  }
}
