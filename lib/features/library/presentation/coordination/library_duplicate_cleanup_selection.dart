import "package:flutter/foundation.dart";

import "../../domain/library_file_operations.dart";

class LibraryDuplicateCleanupSelection extends ChangeNotifier {
  final Set<LibraryCleanupGroup> _groups = {};
  int _revision = 0;

  List<LibraryCleanupGroup> get groups => List.unmodifiable(_groups);
  int get revision => _revision;
  bool get isEmpty => _groups.isEmpty;
  bool contains(LibraryCleanupGroup group) => _groups.contains(group);

  void toggle(LibraryCleanupGroup group) {
    if (!_groups.remove(group)) _groups.add(group);
    _revision++;
    notifyListeners();
  }

  void clear() {
    if (_groups.isEmpty) return;
    _groups.clear();
    _revision++;
    notifyListeners();
  }
}
