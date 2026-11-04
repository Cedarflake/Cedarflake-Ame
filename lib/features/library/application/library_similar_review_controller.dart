import "package:flutter/foundation.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";

import "../domain/library_similar_review.dart";
import "library_duplicate_review_window.dart";
import "library_similar_choices.dart";
import "ports/library_similar_review.dart";

class LibrarySimilarReviewController extends ChangeNotifier {
  LibrarySimilarReviewController(this._service)
    : choices = LibrarySimilarChoices(_service) {
    groupWindow.addListener(notifyListeners);
    memberWindow.addListener(notifyListeners);
    choices.addListener(notifyListeners);
  }

  final LibrarySimilarReview _service;
  final LibrarySimilarChoices choices;
  late final groupWindow =
      LibraryDuplicateReviewWindow<LibrarySimilarGroup, String>(
        _loadGroups,
        seek: _seekGroups,
      );
  late final memberWindow =
      LibraryDuplicateReviewWindow<LibrarySimilarMember, LibrarySimilarContent>(
        _loadMembers,
        onAdmitted: (slice) {
          if (slice is _MemberSlice) _decision = slice.decision;
        },
      );
  LibrarySimilarGroup? selected;
  LibrarySimilarityDegree _degree = LibrarySimilarityDegree.standard;
  LibrarySimilarDecision _decision = LibrarySimilarDecision(
    revision: BigInt.zero,
  );
  bool _ignored = false;
  bool _disposed = false;
  bool _revalidationPending = false;
  int _viewGeneration = 0;

  LibrarySimilarityDegree get degree => _degree;
  LibrarySimilarityDegree get effectiveDegree =>
      _ignored ? LibrarySimilarityDegree.broad : _degree;
  bool get ignored => _ignored;
  LibrarySimilarDecision get decision => _decision;
  bool get loading =>
      selected == null ? groupWindow.loading : memberWindow.loading;
  bool get loadFailed =>
      selected == null ? groupWindow.failed : memberWindow.failed;
  int get itemCount =>
      selected == null ? groupWindow.total : memberWindow.total;
  String get pageKey => selected == null
      ? "similar-groups-$_ignored-${effectiveDegree.name}"
      : "similar-members-${selected!.id}-${effectiveDegree.name}";
  LibrarySimilarContext? get _context {
    final group = selected;
    if (group == null || loading || loadFailed || _disposed) return null;
    return LibrarySimilarContext(
      groupId: group.id,
      degree: effectiveDegree,
      revision: _decision.revision,
    );
  }

  Future<LibraryDuplicateReviewSlice<LibrarySimilarGroup, String>> _loadGroups(
    String? after,
  ) async => _groupSlice(
    await _service.loadGroups(
      after,
      degree: effectiveDegree,
      ignored: _ignored,
    ),
  );

  Future<LibraryDuplicateReviewSeek<LibrarySimilarGroup, String>> _seekGroups(
    int index,
  ) async {
    final range = await _service.seekGroups(
      index,
      degree: effectiveDegree,
      ignored: _ignored,
    );
    return LibraryDuplicateReviewSeek(
      startIndex: range.startIndex,
      slice: _groupSlice(range.page),
    );
  }

  LibraryDuplicateReviewSlice<LibrarySimilarGroup, String> _groupSlice(
    LibrarySimilarGroups page,
  ) => LibraryDuplicateReviewSlice(
    items: page.groups,
    total: page.total,
    next: page.next,
  );

  Future<
    LibraryDuplicateReviewSlice<LibrarySimilarMember, LibrarySimilarContent>
  >
  _loadMembers(LibrarySimilarContent? after) async => _MemberSlice(
    await _service.loadMembers(selected!.id, effectiveDegree, after),
  );

  Future<void> showGroups({
    bool? ignoredGroups,
    LibrarySimilarityDegree? degree,
  }) {
    _viewGeneration++;
    selected = null;
    memberWindow.invalidate(clear: true);
    choices.clearError();
    final nextIgnored = ignoredGroups ?? _ignored;
    final nextDegree = degree ?? _degree;
    if (nextIgnored != _ignored || nextDegree != _degree) {
      groupWindow.invalidate(clear: true);
    }
    _ignored = nextIgnored;
    _degree = nextDegree;
    notifyListeners();
    return groupWindow.revalidate();
  }

  Future<void> showGroup(LibrarySimilarGroup group) {
    _viewGeneration++;
    groupWindow.cancelRead();
    memberWindow.invalidate(clear: true);
    selected = group;
    _decision = group.decision;
    memberWindow.total = group.memberCount;
    choices.clearError();
    notifyListeners();
    return memberWindow.ensureRange(0, 1);
  }

  Future<void> refresh() =>
      selected == null ? groupWindow.refresh() : memberWindow.refresh();
  Future<void> ensureRange(int first, int end) {
    if (choices.saving) return Future.value();
    return selected == null
        ? groupWindow.ensureRange(first, end)
        : memberWindow.ensureRange(first, end);
  }

  Future<void> catalogChanged() {
    if (_disposed) return Future.value();
    if (choices.saving) {
      _revalidationPending = true;
      return Future.value();
    }
    if (selected == null) return groupWindow.revalidate();
    groupWindow.invalidate();
    return memberWindow.revalidate();
  }

  Future<bool> prefer(LibrarySimilarMember? member) =>
      _saveDecision((context) => choices.prefer(context, member));
  Future<bool> setIgnored(bool ignored) =>
      _saveDecision((context) => choices.ignore(context, ignored));

  Future<bool> _saveDecision(
    Future<LibrarySimilarDecision?> Function(LibrarySimilarContext) save,
  ) async {
    final context = _context;
    if (context == null || choices.saving) return false;
    final generation = _viewGeneration;
    memberWindow.cancelRead();
    final result = await save(context);
    if (_disposed) return result != null;
    if (result != null) {
      groupWindow.invalidate();
      if (generation == _viewGeneration) {
        memberWindow.cancelRead();
        _decision = result;
        notifyListeners();
      }
    }
    await _finishWrite(
      revalidate: result == null || generation != _viewGeneration,
    );
    return result != null;
  }

  Future<LibrarySimilarExclusion?> exclude(LibrarySimilarMember member) async {
    final context = _context;
    if (context == null || choices.saving) return null;
    final generation = _viewGeneration;
    memberWindow.cancelRead();
    final result = await choices.exclude(context, member);
    if (_disposed) return result?.undo;
    if (result != null) {
      groupWindow.invalidate();
      if (generation == _viewGeneration) _decision = result.decision;
    }
    await _finishWrite(revalidate: true);
    return result?.undo;
  }

  Future<LibrarySimilarUndoResult?> undo(
    LibrarySimilarExclusion exclusion,
  ) async {
    if (_disposed || choices.saving) return null;
    final generation = _viewGeneration;
    memberWindow.cancelRead();
    final result = await choices.undo(exclusion);
    if (_disposed) return result;
    if (result != null) {
      groupWindow.invalidate();
      if (generation == _viewGeneration && selected?.id == exclusion.groupId) {
        _decision = result.decision;
      }
    }
    await _finishWrite(revalidate: true);
    return result;
  }

  Future<void> _finishWrite({required bool revalidate}) async {
    if (revalidate || _revalidationPending) {
      _revalidationPending = false;
      await catalogChanged();
    }
  }

  @override
  void dispose() {
    _disposed = true;
    _viewGeneration++;
    choices.dispose();
    groupWindow.dispose();
    memberWindow.dispose();
    super.dispose();
  }
}

class _MemberSlice
    extends
        LibraryDuplicateReviewSlice<
          LibrarySimilarMember,
          LibrarySimilarContent
        > {
  _MemberSlice(LibrarySimilarMembers page)
    : decision = page.decision,
      super(items: page.members, total: page.total, next: page.next);
  final LibrarySimilarDecision decision;
}

final librarySimilarReviewProvider = Provider<LibrarySimilarReview>((ref) {
  throw StateError("Library similarity review requires a production binding");
});
