import "package:flutter/foundation.dart";

import "../domain/library_similar_review.dart";
import "ports/library_similar_review.dart";

class LibrarySimilarChoices extends ChangeNotifier {
  LibrarySimilarChoices(this._service);
  final LibrarySimilarReview _service;
  static int _nextExclusion = 0;
  bool _saving = false;
  bool _disposed = false;
  Object? _error;

  bool get saving => _saving;
  Object? get error => _error;

  void clearError() {
    if (_disposed || _error == null) return;
    _error = null;
    notifyListeners();
  }

  Future<LibrarySimilarDecision?> prefer(
    LibrarySimilarContext context,
    LibrarySimilarMember? member,
  ) => _save(() => _service.prefer(context, member));

  Future<LibrarySimilarDecision?> ignore(
    LibrarySimilarContext context,
    bool ignored,
  ) => _save(() => _service.ignore(context, ignored));

  Future<LibrarySimilarExclusionResult?> exclude(
    LibrarySimilarContext context,
    LibrarySimilarMember member,
  ) => _save(
    () => _service.exclude(
      context,
      member,
      "similar-exclusion-${DateTime.now().microsecondsSinceEpoch}-${_nextExclusion++}",
    ),
  );

  Future<LibrarySimilarUndoResult?> undo(LibrarySimilarExclusion exclusion) =>
      _save(() => _service.undo(exclusion));

  Future<T?> _save<T>(Future<T> Function() write) async {
    if (_disposed || _saving) return null;
    _saving = true;
    _error = null;
    notifyListeners();
    try {
      return await write();
    } on Object catch (error) {
      if (!_disposed) _error = error;
      return null;
    } finally {
      _saving = false;
      if (!_disposed) notifyListeners();
    }
  }

  @override
  void dispose() {
    _disposed = true;
    super.dispose();
  }
}
