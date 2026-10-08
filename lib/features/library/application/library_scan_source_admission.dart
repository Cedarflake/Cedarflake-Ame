import "../domain/library_models.dart";
import "../domain/library_state.dart";
import "library_scan_target.dart";

/// Resolves source-binding feedback against the published roots before a run.
class LibraryScanSourceAdmission {
  LibraryScanSourceAdmission({
    required this._scan,
    required List<LibraryRoot> roots,
    required this._target,
    required this._isResuming,
  }) : _roots = List.unmodifiable(roots);

  final RecoverableLibraryScan _scan;
  final List<LibraryRoot> _roots;
  final LibraryScanTarget _target;
  final bool _isResuming;
  Future<bool>? _refresh;

  LibraryTaskKind get initialTaskKind {
    final explicitKind = _target.taskKind;
    if (explicitKind != null) {
      return explicitKind;
    }
    if (_roots.any(
      (root) => root.activeScanId != null && _matchesRequestedPath(root),
    )) {
      return LibraryTaskKind.update;
    }
    return LibraryTaskKind.import;
  }

  bool get needsRefresh {
    if (_target is LibraryRootRelocation) {
      return true;
    }
    if (_isResuming || _roots.every((root) => root.activeScanId == null)) {
      return false;
    }
    return !_roots.any(_matchesRequestedPath);
  }

  Future<bool> refresh(Future<bool> Function() reloadCatalog) =>
      _refresh ??= reloadCatalog();

  bool hasAdmittedExistingRoot(
    List<LibraryRoot> roots, {
    required String? displayRootPath,
  }) {
    final Set<String> eligibleIds;
    if (_target case LibraryRootRelocation(:final root)) {
      eligibleIds = {root.id};
    } else {
      eligibleIds = {
        for (final root in _roots)
          if (root.activeScanId != null) root.id,
      };
    }
    return roots.any(
      (root) =>
          eligibleIds.contains(root.id) &&
          (_matchesRequestedPath(root) ||
              root.path == displayRootPath ||
              root.displayPath == displayRootPath),
    );
  }

  bool _matchesRequestedPath(LibraryRoot root) =>
      root.path == _scan.rootPath ||
      root.path == _scan.displayRootPath ||
      root.displayPath == _scan.rootPath ||
      root.displayPath == _scan.displayRootPath;
}
