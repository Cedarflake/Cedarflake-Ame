import "package:flutter/foundation.dart";

import "../domain/library_duplicate_review.dart";
import "../domain/library_exact_duplicates.dart";
import "../domain/library_models.dart";
import "library_duplicate_review.dart";

class LibraryDuplicateReviewController extends ChangeNotifier {
  LibraryDuplicateReviewController(this._service);
  final LibraryDuplicateReview _service;
  LibraryDuplicateReviewPage groups = const LibraryDuplicateReviewPage(
    groups: [],
    totalGroups: 0,
  );
  LibraryDuplicateReviewLocations members =
      const LibraryDuplicateReviewLocations(
        locations: [],
        decision: LibraryDuplicateDecision(),
      );
  LibraryDuplicateReviewGroup? selected;
  bool ignored = false;
  bool loading = false;
  bool saving = false;
  bool loadFailed = false;
  bool saveFailed = false;
  bool _disposed = false;
  int _read = 0;
  int _write = 0;
  LibraryDuplicateGroupCursor? _groupCursor;
  String? _locationCursor;
  final List<LibraryDuplicateGroupCursor?> _groupHistory = [];
  final List<String?> _locationHistory = [];

  bool get canPrevious =>
      selected == null ? _groupHistory.isNotEmpty : _locationHistory.isNotEmpty;
  bool get canNext =>
      !loadFailed &&
      (selected == null ? groups.next != null : members.nextLocationId != null);
  String get pageKey => selected == null
      ? "groups-$ignored-${_groupCursor?.digest}"
      : "${selected!.group.digest}-$_locationCursor";

  Future<void> refresh() async {
    final read = ++_read;
    final group = selected;
    loading = true;
    loadFailed = false;
    notifyListeners();
    try {
      if (group == null) {
        final page = await _service.loadGroups(_groupCursor, ignored: ignored);
        if (!_disposed && read == _read) groups = page;
      } else {
        final page = await _service.loadLocations(group.group, _locationCursor);
        if (!_disposed && read == _read) members = page;
      }
    } on Object {
      if (!_disposed && read == _read) loadFailed = true;
    } finally {
      if (!_disposed && read == _read) {
        loading = false;
        notifyListeners();
      }
    }
  }

  Future<void> showGroup(LibraryDuplicateReviewGroup group) {
    selected = group;
    _locationCursor = null;
    _locationHistory.clear();
    members = LibraryDuplicateReviewLocations(
      locations: const [],
      decision: group.decision,
    );
    saveFailed = false;
    return refresh();
  }

  Future<void> showGroups({bool? ignoredGroups}) {
    selected = null;
    saveFailed = false;
    if (ignoredGroups != null && ignoredGroups != ignored) {
      ignored = ignoredGroups;
      _groupCursor = null;
      _groupHistory.clear();
      groups = const LibraryDuplicateReviewPage(groups: [], totalGroups: 0);
    }
    return refresh();
  }

  Future<void> catalogChanged() {
    _groupCursor = null;
    _groupHistory.clear();
    // Location IDs remain keyset boundaries across publication; keep the reviewed page.
    return refresh();
  }

  Future<void> next() {
    if (loading || saving || !canNext) return Future.value();
    if (selected == null) {
      _groupHistory.add(_groupCursor);
      if (_groupHistory.length > 64) _groupHistory.removeAt(0);
      _groupCursor = groups.next;
    } else {
      _locationHistory.add(_locationCursor);
      if (_locationHistory.length > 64) _locationHistory.removeAt(0);
      _locationCursor = members.nextLocationId;
    }
    return refresh();
  }

  Future<void> previous() {
    if (loading || saving || !canPrevious) return Future.value();
    if (selected == null) {
      _groupCursor = _groupHistory.removeLast();
    } else {
      _locationCursor = _locationHistory.removeLast();
    }
    return refresh();
  }

  Future<bool> prefer(LibraryAsset? asset) =>
      _save((group) => _service.prefer(group, asset));
  Future<bool> setIgnored(bool value) =>
      _save((group) => _service.ignore(group, value));

  Future<bool> _save(
    Future<LibraryDuplicateDecision> Function(LibraryDuplicateGroup) write,
  ) async {
    final target = selected;
    if (target == null || saving || loading || loadFailed) return false;
    final generation = ++_write;
    saving = true;
    saveFailed = false;
    notifyListeners();
    try {
      final decision = await write(target.group);
      if (_disposed || generation != _write) return false;
      if (identical(selected, target)) {
        // Retire reads that began before this write committed; their decision may be older.
        _read++;
        loading = false;
        members = LibraryDuplicateReviewLocations(
          locations: members.locations,
          nextLocationId: members.nextLocationId,
          decision: decision,
        );
      }
      return true;
    } on Object {
      if (!_disposed && generation == _write && identical(selected, target)) {
        saveFailed = true;
      }
      return false;
    } finally {
      if (!_disposed && generation == _write) {
        saving = false;
        notifyListeners();
      }
    }
  }

  @override
  void dispose() {
    _disposed = true;
    _read++;
    _write++;
    super.dispose();
  }
}
