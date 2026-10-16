import "package:flutter/foundation.dart";

import "../domain/library_exact_duplicates.dart";
import "library_exact_duplicates.dart";

class LibraryDuplicateResultsController extends ChangeNotifier {
  LibraryDuplicateResultsController(this._service);

  final LibraryExactDuplicates _service;
  LibraryDuplicateGroupPage _groups = const LibraryDuplicateGroupPage(
    groups: [],
  );
  LibraryDuplicateLocationPage _locations = const LibraryDuplicateLocationPage(
    locations: [],
  );
  LibraryDuplicateGroup? _selected;
  LibraryDuplicateGroupCursor? _groupCursor;
  String? _locationCursor;
  final List<LibraryDuplicateGroupCursor?> _groupHistory = [];
  final List<String?> _locationHistory = [];
  int _request = 0;
  bool _disposed = false;
  bool _loading = false;
  bool _hasError = false;

  LibraryDuplicateGroupPage get groups => _groups;
  LibraryDuplicateLocationPage get locations => _locations;
  LibraryDuplicateGroup? get selected => _selected;
  bool get loading => _loading;
  bool get hasError => _hasError;
  bool get canPrevious => _selected == null
      ? _groupHistory.isNotEmpty
      : _locationHistory.isNotEmpty;
  bool get canNext {
    if (_hasError) {
      return false;
    }
    return _selected == null
        ? _groups.next != null
        : _locations.nextLocationId != null;
  }

  Future<void> refresh() async {
    final request = ++_request;
    final selected = _selected;
    _loading = true;
    _hasError = false;
    notifyListeners();
    try {
      if (selected == null) {
        final page = await _service.loadGroups(_groupCursor);
        if (_disposed || request != _request) {
          return;
        }
        _groups = page;
      } else {
        final page = await _service.loadLocations(selected, _locationCursor);
        if (_disposed || request != _request) {
          return;
        }
        _locations = page;
      }
    } catch (_) {
      if (_disposed || request != _request) {
        return;
      }
      _hasError = true;
    } finally {
      if (!_disposed && request == _request) {
        _loading = false;
        notifyListeners();
      }
    }
  }

  Future<void> showLocations(LibraryDuplicateGroup group) {
    _selected = group;
    _locationCursor = null;
    _locationHistory.clear();
    _locations = const LibraryDuplicateLocationPage(locations: []);
    return refresh();
  }

  Future<void> showGroups() {
    _selected = null;
    return refresh();
  }

  Future<void> catalogChanged() {
    _groupCursor = null;
    _locationCursor = null;
    _groupHistory.clear();
    _locationHistory.clear();
    _groups = const LibraryDuplicateGroupPage(groups: []);
    _locations = const LibraryDuplicateLocationPage(locations: []);
    return refresh();
  }

  Future<void> next() {
    if (loading || !canNext) {
      return Future.value();
    }
    if (_selected == null) {
      _groupHistory.add(_groupCursor);
      if (_groupHistory.length > 64) {
        _groupHistory.removeAt(0);
      }
      _groupCursor = _groups.next;
    } else {
      _locationHistory.add(_locationCursor);
      if (_locationHistory.length > 64) {
        _locationHistory.removeAt(0);
      }
      _locationCursor = _locations.nextLocationId;
    }
    return refresh();
  }

  Future<void> previous() {
    if (loading || !canPrevious) {
      return Future.value();
    }
    if (_selected == null) {
      _groupCursor = _groupHistory.removeLast();
    } else {
      _locationCursor = _locationHistory.removeLast();
    }
    return refresh();
  }

  @override
  void dispose() {
    _disposed = true;
    _request++;
    super.dispose();
  }
}
