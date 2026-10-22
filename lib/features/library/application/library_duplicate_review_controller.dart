import "package:flutter/foundation.dart";

import "../domain/library_duplicate_review.dart";
import "../domain/library_exact_duplicates.dart";
import "../domain/library_models.dart";
import "library_duplicate_review.dart";
import "library_duplicate_review_window.dart";

class LibraryDuplicateReviewController extends ChangeNotifier {
  LibraryDuplicateReviewController(this._service) {
    groupWindow.addListener(notifyListeners);
    memberWindow.addListener(notifyListeners);
  }
  final LibraryDuplicateReview _service;
  late final groupWindow =
      LibraryDuplicateReviewWindow<
        LibraryDuplicateReviewGroup,
        LibraryDuplicateGroupCursor
      >(_loadGroups, seek: _seekGroups);
  late final memberWindow =
      LibraryDuplicateReviewWindow<LibraryDuplicateReviewLocation, String>(
        _loadMembers,
        onAdmitted: (slice) {
          if (slice.decision case final decision?) _decision = decision;
        },
      );
  LibraryDuplicateDecision _decision = const LibraryDuplicateDecision();
  LibraryDuplicateReviewGroup? selected;
  bool ignored = false;
  bool saving = false;
  bool saveFailed = false;
  bool _disposed = false;
  int _write = 0;

  bool get loading =>
      selected == null ? groupWindow.loading : memberWindow.loading;
  bool get loadFailed =>
      selected == null ? groupWindow.failed : memberWindow.failed;
  int get itemCount =>
      selected == null ? groupWindow.total : memberWindow.total;
  LibraryDuplicateDecision get decision => _decision;
  String get pageKey => selected == null
      ? "groups-$ignored"
      : "members-${selected!.group.digest}-${selected!.group.byteLength}";
  LibraryDuplicateReviewPage get groups => LibraryDuplicateReviewPage(
    groups: groupWindow.loadedItems,
    totalGroups: groupWindow.total,
  );
  LibraryDuplicateReviewLocations get members =>
      LibraryDuplicateReviewLocations(
        locations: memberWindow.loadedItems,
        decision: _decision,
      );

  Future<
    LibraryDuplicateReviewSlice<
      LibraryDuplicateReviewGroup,
      LibraryDuplicateGroupCursor
    >
  >
  _loadGroups(LibraryDuplicateGroupCursor? after) async {
    final page = await _service.loadGroups(after, ignored: ignored);
    return _groupSlice(page);
  }

  Future<
    LibraryDuplicateReviewSeek<
      LibraryDuplicateReviewGroup,
      LibraryDuplicateGroupCursor
    >
  >
  _seekGroups(int index) async {
    final range = await _service.seekGroups(index, ignored: ignored);
    return LibraryDuplicateReviewSeek(
      startIndex: range.startIndex,
      slice: _groupSlice(range.page),
    );
  }

  LibraryDuplicateReviewSlice<
    LibraryDuplicateReviewGroup,
    LibraryDuplicateGroupCursor
  >
  _groupSlice(LibraryDuplicateReviewPage page) => LibraryDuplicateReviewSlice(
    items: page.groups,
    total: page.totalGroups,
    next: page.next,
  );

  Future<LibraryDuplicateReviewSlice<LibraryDuplicateReviewLocation, String>>
  _loadMembers(String? after) async {
    final group = selected!;
    final page = await _service.loadLocations(group.group, after);
    return LibraryDuplicateReviewSlice(
      items: page.locations,
      total: group.group.locationCount,
      next: page.nextLocationId,
      decision: page.decision,
    );
  }

  Future<void> refresh() =>
      selected == null ? groupWindow.refresh() : memberWindow.refresh();

  Future<void> ensureRange(int first, int end) {
    if (saving) return Future.value();
    return selected == null
        ? groupWindow.ensureRange(first, end)
        : memberWindow.ensureRange(first, end);
  }

  Future<void> showGroup(LibraryDuplicateReviewGroup group) {
    groupWindow.cancelRead();
    selected = group;
    memberWindow.invalidate(clear: true);
    memberWindow.total = group.group.locationCount;
    _decision = group.decision;
    saveFailed = false;
    notifyListeners();
    return memberWindow.ensureRange(0, 1);
  }

  Future<void> showGroups({bool? ignoredGroups}) {
    memberWindow.cancelRead();
    selected = null;
    saveFailed = false;
    if (ignoredGroups != null && ignoredGroups != ignored) {
      ignored = ignoredGroups;
      groupWindow.invalidate(clear: true);
    }
    return groupWindow.refresh();
  }

  Future<void> catalogChanged() {
    if (selected == null) return groupWindow.revalidate();
    groupWindow.invalidate();
    return memberWindow.revalidate();
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
        memberWindow.cancelRead();
        _decision = decision;
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
    _write++;
    groupWindow.dispose();
    memberWindow.dispose();
    super.dispose();
  }
}
