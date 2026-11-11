import "dart:async";

import "package:flutter/foundation.dart";

import "../../application/ports/library_review.dart";
import "../../application/library_review_session.dart";
import "../../application/library_review_continuation_reader.dart";
import "../../application/library_organization_publication.dart";
import "../../domain/library_models.dart";
import "../../domain/library_organization.dart";
import "../../domain/library_review.dart";
import "../../domain/library_state.dart";
import "gallery_selection.dart";
import "library_organization_transition.dart";
import "../widgets/library_gallery_position.dart";

class LibraryReviewBrowseState {
  const LibraryReviewBrowseState({
    required this.query,
    required this.selection,
    required this.isSelecting,
    this.position,
  });
  final LibraryGalleryQuery query;
  final GallerySelection selection;
  final bool isSelecting;
  final LibraryGalleryVisiblePosition? position;
}

abstract interface class LibraryReviewSurface {
  LibraryState get library;
  LibraryReviewBrowseState captureBrowse();
  Future<bool> showReviewQuery(
    LibraryGalleryQuery query, {
    LibraryOrganizationTarget? target,
    LibraryGalleryVisiblePosition? position,
  });
  void restoreReviewBrowse(LibraryReviewBrowseState saved);
  void openReviewImage(LibraryAsset asset);
  void closeReviewImage();
}

/// Retains ordinary browsing while the shared gallery visits review queues.
class LibraryOrganizationMode extends ChangeNotifier
    implements LibraryOrganizationProjection {
  LibraryOrganizationMode(this.service, this.surface) {
    _continuationReader = LibraryReviewContinuationReader(
      (query) => service.continuation(query: query),
      _acceptContinuation,
    );
  }
  final LibraryReview service;
  final LibraryReviewSurface surface;
  LibraryReviewBrowseState? _ordinary;
  LibraryReviewSession? _session;
  LibraryReviewContinuation? _continuation;
  final _transition = LibraryOrganizationTransition();
  bool _disposed = false;
  late final LibraryReviewContinuationReader _continuationReader;
  int _viewerIntent = 0;
  ({String message, Future<void> Function()? retry})? _problem;

  bool get isActive => _ordinary != null;
  bool get canEnter => surface.library.query.collection is LibraryAllCollection;
  LibraryGalleryQuery? get ordinaryQuery => _ordinary?.query;
  LibraryReviewBrowseState? get ordinaryBrowse => _ordinary;
  bool get busy => _transition.busy;
  String? get problem => _problem?.message;
  LibraryReviewSession? get session => _session;
  LibraryReviewEntry? get current => _session?.current;
  LibraryReviewContinuation? get continuation =>
      _continuation?.forScope(surface.library.query);
  LibraryOrganizationQueue get queue =>
      surface.library.query.organizationQueue ??
      LibraryOrganizationQueue.pending;

  Future<void> toggle() => isActive ? exit() : enter();

  @override
  Future<bool> retireAlbum(String albumId) async {
    if (_disposed) return false;
    final collection = LibraryGalleryCollection.album(albumId);
    LibraryGalleryQuery withoutAlbum(LibraryGalleryQuery query) =>
        query.copyWith(
          collection: const LibraryGalleryCollection.library(),
          rootId: null,
          folderRelativePath: null,
        );
    final saved = _ordinary;
    if (saved != null && saved.query.collection == collection) {
      _ordinary = LibraryReviewBrowseState(
        query: withoutAlbum(saved.query),
        selection: saved.selection.clear(),
        isSelecting: false,
      );
    }
    final current = surface.library.query;
    if (current.collection != collection) return true;
    final query = withoutAlbum(current);
    final applied = await (isActive
        ? changeQuery(query)
        : surface.showReviewQuery(query));
    return !_disposed &&
        (applied || surface.library.query.collection != collection);
  }

  Future<void> enter() => _run((intent) async {
    if (isActive || !canEnter) return;
    final saved = surface.captureBrowse();
    final boundary = await service.admissionBoundary();
    if (!_owns(intent) || surface.library.query != saved.query) return;
    final query = saved.query.copyWith(
      organizationQueue: LibraryOrganizationQueue.pending,
      newSinceAdmission: boundary,
    );
    unawaited(_continuationReader.refresh(query));
    if (!await surface.showReviewQuery(query) || !_owns(intent)) return;
    _ordinary = saved;
    _session = LibraryReviewSession(service, query);
    _acceptContinuation();
  }, failureMessage: "无法进入整理模式，请重试。");

  Future<void> exit() =>
      _run(_exit, failureMessage: "无法返回图库，请重试。", supersede: true);

  Future<void> _exit(int intent) async {
    final saved = _ordinary;
    if (saved == null) return;
    _viewerIntent++;
    _session?.clearCurrent();
    _continuationReader.invalidate();
    if (!await surface.showReviewQuery(saved.query, position: saved.position) ||
        !_owns(intent)) {
      return;
    }
    _replaceSession(null);
    _ordinary = null;
    _continuation = null;
    _continuationReader.invalidate();
    surface.restoreReviewBrowse(saved);
  }

  Future<void> switchWorkspace(Future<void> Function() navigate) async {
    if (_disposed) return;
    await _transition.navigate(() async {
      if (_disposed) return;
      notifyListeners();
      await navigate();
    });
    if (!_disposed) notifyListeners();
  }

  Future<void> leaveWorkspace() {
    assert(_transition.isNavigating && !_transition.isRunning);
    return _guard(_exit, "无法返回图库，请重试。");
  }

  bool restoreWorkspace(LibraryReviewBrowseState ordinary) {
    final query = surface.library.query;
    if (_disposed ||
        _transition.isRunning ||
        isActive ||
        !canEnter ||
        query.organizationQueue == null) {
      return false;
    }
    _ordinary = ordinary;
    _session = LibraryReviewSession(service, query);
    _problem = null;
    unawaited(refreshContinuation());
    notifyListeners();
    return true;
  }

  Future<bool> changeQuery(LibraryGalleryQuery requested) async {
    var applied = false;
    await _run((intent) async {
      if (!isActive || requested.collection is! LibraryAllCollection) return;
      final query = requested.copyWith(
        organizationQueue: requested.organizationQueue ?? queue,
        newSinceAdmission: surface.library.query.newSinceAdmission,
      );
      final browse = surface.captureBrowse();
      final onlyOrderingChanged =
          browse.query.copyWith(
            sortKey: query.sortKey,
            sortDirection: query.sortDirection,
          ) ==
          query;
      unawaited(_continuationReader.refresh(query));
      if (!await surface.showReviewQuery(
            query,
            position: onlyOrderingChanged ? browse.position : null,
          ) ||
          !_owns(intent)) {
        return;
      }
      applied = true;
      _replaceSession(query);
      _acceptContinuation();
    }, failureMessage: "无法切换整理范围，请重试。");
    return applied;
  }

  Future<void> selectQueue(LibraryOrganizationQueue queue) async {
    await changeQuery(surface.library.query.copyWith(organizationQueue: queue));
  }

  Future<void> showConflicts() => _run((intent) async {
    final saved = _ordinary ?? surface.captureBrowse();
    final boundary = isActive
        ? surface.library.query.newSinceAdmission
        : await service.admissionBoundary();
    if (!_owns(intent)) return;
    final query = LibraryGalleryQuery(
      organizationQueue: LibraryOrganizationQueue.deferred,
      newSinceAdmission: boundary,
    );
    unawaited(_continuationReader.refresh(query));
    if (!await surface.showReviewQuery(query) || !_owns(intent)) return;
    surface.closeReviewImage();
    _ordinary = saved;
    _replaceSession(query);
    _acceptContinuation();
  }, failureMessage: "无法打开归属冲突的图片，请重试。");

  void open(LibraryAsset asset) {
    final session = _session;
    if (!isActive || _disposed || session == null) return;
    _viewerIntent++;
    final entry = session.open(LibraryOrganizationTarget.fromAsset(asset));
    _problem = null;
    unawaited(_checkpoint(session, entry));
  }

  void adoptDisplayedAsset(LibraryAsset asset) {
    if (current?.target != LibraryOrganizationTarget.fromAsset(asset)) {
      open(asset);
    }
  }

  void reconcileDisplayedAsset(LibraryAsset asset) {
    final entry = current;
    if (entry != null && !entry.committed) adoptDisplayedAsset(asset);
  }

  void closeCurrent() {
    _viewerIntent++;
    _session?.clearCurrent();
    _problem = null;
  }

  Future<void> _checkpoint(
    LibraryReviewSession session,
    LibraryReviewEntry entry,
  ) async {
    try {
      await session.checkpoint(entry);
      if (!_disposed && session.owns(entry)) await refreshContinuation();
    } on Object {
      if (!_disposed && session.owns(entry)) {
        _problem = (message: "整理位置尚未保存，请重试。", retry: null);
        notifyListeners();
      }
    }
  }

  Future<void> refreshContinuation() {
    if (!isActive || _disposed) return Future.value();
    return _continuationReader.refresh(surface.library.query);
  }

  void _acceptContinuation() {
    final snapshot = _continuationReader.latest;
    if (_disposed || !isActive || snapshot?.query != surface.library.query) {
      return;
    }
    if (snapshot!.failed) {
      _problem = (message: "无法读取整理位置，请重试。", retry: refreshContinuation);
    } else {
      _continuation = snapshot.value;
    }
    notifyListeners();
  }

  Future<void> resume() => _run((intent) async {
    if (!isActive) return;
    final saved = await service.continuation(query: surface.library.query);
    if (!_owns(intent)) return;
    final result = saved.forScope(surface.library.query);
    _continuation = result;
    if (result.canResume) await _showContinuation(result, intent);
  }, failureMessage: "无法恢复整理位置，请重试。");

  Future<void> afterSave(LibraryReviewEntry entry) => _run((intent) async {
    final session = _session;
    if (session == null || !session.owns(entry) || !entry.committed) return;
    final saved = await service.continuation(query: surface.library.query);
    if (!_owns(intent) || !session.owns(entry)) return;
    final result = saved.forScope(surface.library.query);
    _continuation = result;
    if (result.canResume) {
      await _showContinuation(result, intent);
    } else {
      closeCurrent();
      surface.closeReviewImage();
    }
  }, failureMessage: "更改已保存，无法打开下一张图片，请重试。");

  Future<void> _showContinuation(
    LibraryReviewContinuation result,
    int command,
  ) async {
    final intent = _viewerIntent;
    final query = result.query!.copyWith(
      newSinceAdmission: surface.library.query.newSinceAdmission,
    );
    final target = result.target!;
    final applied = await surface.showReviewQuery(query, target: target);
    if (!_owns(command)) return;
    if (!applied) {
      throw StateError("The review query could not be restored");
    }
    if (intent != _viewerIntent) return;
    if (_session?.query != query) _replaceSession(query);
    if (_disposed || intent != _viewerIntent) return;
    final asset = surface.library.assets
        .where((asset) => LibraryOrganizationTarget.fromAsset(asset) == target)
        .firstOrNull;
    if (asset == null) {
      throw const LibraryOrganizationFailure(
        "organization_review_changed",
        "The continuation image changed",
      );
    }
    open(asset);
    surface.openReviewImage(asset);
  }

  Future<void> retry() async {
    final retryCommand = _problem?.retry;
    _problem = null;
    if (retryCommand != null) {
      await retryCommand();
      if (!_disposed) notifyListeners();
      return;
    }
    final session = _session;
    final entry = session?.current;
    if (session != null && entry != null) {
      if (entry.committed) {
        await afterSave(entry);
      } else {
        await _checkpoint(session, entry);
      }
    } else {
      await refreshContinuation();
    }
    if (!_disposed) notifyListeners();
  }

  void reportSaveFailure(
    LibraryReviewEntry entry,
    String message,
    Future<void> Function() retry,
  ) {
    if (_disposed || _session?.owns(entry) != true) return;
    _problem = (message: message, retry: retry);
    notifyListeners();
  }

  void _replaceSession(LibraryGalleryQuery? query) {
    final previous = _session;
    _session = !_disposed && query != null
        ? LibraryReviewSession(service, query)
        : null;
    unawaited(
      previous?.close().catchError((Object error) {
        debugPrint("[Ame organization] review retirement failed: $error");
      }),
    );
  }

  bool _owns(int intent) => !_disposed && _transition.owns(intent);

  Future<void> _run(
    Future<void> Function(int) operation, {
    required String failureMessage,
    bool supersede = false,
  }) async {
    if (_disposed) return;
    await _transition.run(
      () => _guard(operation, failureMessage),
      supersede: supersede,
    );
    if (!_disposed) notifyListeners();
  }

  Future<void> _guard(
    Future<void> Function(int) operation,
    String failureMessage,
  ) async {
    final intent = _transition.generation;
    _problem = null;
    notifyListeners();
    try {
      await operation(intent);
    } on Object {
      if (_owns(intent)) {
        _problem = (
          message: failureMessage,
          retry: () => _run(operation, failureMessage: failureMessage),
        );
      }
    } finally {
      if (!_disposed) {
        notifyListeners();
      }
    }
  }

  @override
  void dispose() {
    _disposed = true;
    _continuationReader.close();
    unawaited(
      _session?.close().catchError((Object error) {
        debugPrint("[Ame organization] review retirement failed: $error");
      }),
    );
    super.dispose();
  }
}
