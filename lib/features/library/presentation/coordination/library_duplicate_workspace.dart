import "package:flutter/foundation.dart";

import "../../application/library_duplicate_review_controller.dart";
import "../../application/library_similar_review_controller.dart";
import "../../domain/library_duplicate_review.dart";
import "../../domain/library_models.dart";
import "../../domain/library_similar_review.dart";

enum LibraryDuplicateView { exact, ignored, similar }

enum LibraryDuplicateKind { exact, similar }

sealed class LibraryReviewGroupEntry {
  const LibraryReviewGroupEntry();
  LibraryAsset get representative;
}

class LibraryExactGroupEntry extends LibraryReviewGroupEntry {
  const LibraryExactGroupEntry(this.group);
  final LibraryDuplicateReviewGroup group;
  @override
  LibraryAsset get representative => group.representative;
}

class LibrarySimilarGroupEntry extends LibraryReviewGroupEntry {
  const LibrarySimilarGroupEntry(this.group);
  final LibrarySimilarGroup group;
  @override
  LibraryAsset get representative => group.representative;
}

class LibraryDuplicateWorkspace extends ChangeNotifier {
  LibraryDuplicateWorkspace({required this.exact, required this.similar}) {
    exact.addListener(notifyListeners);
    similar.addListener(notifyListeners);
  }
  final LibraryDuplicateReviewController exact;
  final LibrarySimilarReviewController similar;
  LibraryDuplicateView view = LibraryDuplicateView.exact;
  LibraryDuplicateKind? memberKind;
  final Map<String, LibraryAsset> _comparison = {};
  List<LibraryAsset> get comparison => List.unmodifiable(_comparison.values);
  bool get hasMembers => memberKind != null;
  bool get isSimilar => memberKind == LibraryDuplicateKind.similar;
  bool get saving => exact.saving || similar.choices.saving;
  bool get saveFailed =>
      isSimilar ? similar.choices.error != null : exact.saveFailed;
  bool get loading => switch (memberKind) {
    LibraryDuplicateKind.exact => exact.loading,
    LibraryDuplicateKind.similar => similar.loading,
    null => switch (view) {
      LibraryDuplicateView.exact => exact.groupWindow.loading,
      LibraryDuplicateView.similar => similar.groupWindow.loading,
      LibraryDuplicateView.ignored =>
        exact.groupWindow.loading || similar.groupWindow.loading,
    },
  };
  bool get loadFailed => switch (memberKind) {
    LibraryDuplicateKind.exact => exact.loadFailed,
    LibraryDuplicateKind.similar => similar.loadFailed,
    null => switch (view) {
      LibraryDuplicateView.exact => exact.groupWindow.failed,
      LibraryDuplicateView.similar => similar.groupWindow.failed,
      LibraryDuplicateView.ignored =>
        exact.groupWindow.failed || similar.groupWindow.failed,
    },
  };
  int get groupCount => switch (view) {
    LibraryDuplicateView.exact => exact.groupWindow.total,
    LibraryDuplicateView.similar => similar.groupWindow.total,
    LibraryDuplicateView.ignored =>
      exact.groupWindow.total + similar.groupWindow.total,
  };
  int get itemCount => switch (memberKind) {
    LibraryDuplicateKind.exact => exact.itemCount,
    LibraryDuplicateKind.similar => similar.itemCount,
    null => groupCount,
  };
  bool get ignored =>
      isSimilar ? similar.decision.ignored : exact.decision.ignored;
  LibraryAsset? get representative => switch (memberKind) {
    LibraryDuplicateKind.exact => exact.selected?.representative,
    LibraryDuplicateKind.similar => similar.selected?.representative,
    null => null,
  };
  String get pageKey => switch (memberKind) {
    LibraryDuplicateKind.exact => exact.pageKey,
    LibraryDuplicateKind.similar => similar.pageKey,
    null => "review-${view.name}-${similar.effectiveDegree.name}",
  };

  LibraryReviewGroupEntry? groupAt(int index) {
    if (view == LibraryDuplicateView.similar ||
        (view == LibraryDuplicateView.ignored &&
            index >= exact.groupWindow.total)) {
      final offset = view == LibraryDuplicateView.ignored
          ? exact.groupWindow.total
          : 0;
      final group = similar.groupWindow.itemAt(index - offset);
      return group == null ? null : LibrarySimilarGroupEntry(group);
    }
    final group = exact.groupWindow.itemAt(index);
    return group == null ? null : LibraryExactGroupEntry(group);
  }

  LibraryAsset? assetAt(int index) => switch (memberKind) {
    LibraryDuplicateKind.exact => exact.memberWindow.itemAt(index)?.asset,
    LibraryDuplicateKind.similar => similar.memberWindow.itemAt(index)?.asset,
    null => groupAt(index)?.representative,
  };

  Future<void> showView(
    LibraryDuplicateView next, {
    LibrarySimilarityDegree? degree,
  }) async {
    view = next;
    memberKind = null;
    _comparison.clear();
    exact.memberWindow.cancelRead();
    similar.memberWindow.cancelRead();
    notifyListeners();
    await Future.wait([
      if (next != LibraryDuplicateView.similar)
        exact.showGroups(ignoredGroups: next == LibraryDuplicateView.ignored),
      if (next != LibraryDuplicateView.exact)
        similar.showGroups(
          ignoredGroups: next == LibraryDuplicateView.ignored,
          degree: degree,
        ),
    ]);
  }

  Future<void> openGroup(LibraryReviewGroupEntry entry) {
    _comparison.clear();
    switch (entry) {
      case LibraryExactGroupEntry(:final group):
        memberKind = LibraryDuplicateKind.exact;
        return exact.showGroup(group);
      case LibrarySimilarGroupEntry(:final group):
        memberKind = LibraryDuplicateKind.similar;
        return similar.showGroup(group);
    }
  }

  Future<void> ensureRange(int first, int end) async {
    if (memberKind == LibraryDuplicateKind.exact) {
      return exact.ensureRange(first, end);
    }
    if (memberKind == LibraryDuplicateKind.similar) {
      return similar.ensureRange(first, end);
    }
    final offset = view == LibraryDuplicateView.ignored
        ? exact.groupWindow.total
        : 0;
    await Future.wait([
      if (view != LibraryDuplicateView.similar &&
          first < exact.groupWindow.total)
        exact.groupWindow.ensureRange(
          first,
          end.clamp(0, exact.groupWindow.total),
        ),
      if (view != LibraryDuplicateView.exact && end > offset)
        similar.groupWindow.ensureRange(
          (first - offset).clamp(0, end),
          end - offset,
        ),
    ]);
  }

  Future<void> refresh() async {
    if (memberKind == LibraryDuplicateKind.exact) return exact.refresh();
    if (memberKind == LibraryDuplicateKind.similar) return similar.refresh();
    return showView(view);
  }

  Future<void> catalogChanged({bool keepComparison = false}) async {
    if (!keepComparison) _comparison.clear();
    if (hasMembers) {
      if (isSimilar) {
        await similar.catalogChanged();
      } else {
        await exact.catalogChanged();
      }
    } else {
      await Future.wait([
        if (view != LibraryDuplicateView.similar) exact.catalogChanged(),
        if (view != LibraryDuplicateView.exact) similar.catalogChanged(),
      ]);
    }
  }

  bool isCompared(LibraryAsset asset) =>
      _comparison.containsKey(asset.locationId);
  bool canCompare(LibraryAsset asset) =>
      isCompared(asset) || _comparison.length < 2;
  void toggleComparison(LibraryAsset asset) {
    if (isCompared(asset)) {
      _comparison.remove(asset.locationId);
    } else if (_comparison.length < 2) {
      _comparison[asset.locationId] = asset;
    }
    notifyListeners();
  }

  void removeComparison(LibraryAsset asset) {
    _comparison.remove(asset.locationId);
    notifyListeners();
  }

  @override
  void dispose() {
    exact.dispose();
    similar.dispose();
    super.dispose();
  }
}
