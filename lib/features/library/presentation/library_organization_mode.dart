import "dart:async";

import "package:flutter/foundation.dart";

import "../application/library_review.dart";
import "../application/library_review_session.dart";
import "../application/library_organization_publication.dart";
import "../domain/library_models.dart";
import "../domain/library_organization.dart";
import "../domain/library_review.dart";
import "../domain/library_state.dart";
import "gallery_selection.dart";
import "widgets/library_gallery_position.dart";

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
  LibraryOrganizationMode(this.service, this.surface);
  final LibraryReview service;
  final LibraryReviewSurface surface;
  LibraryReviewBrowseState? _ordinary;
  LibraryReviewSession? _session;
  LibraryReviewContinuation? _continuation;
  bool _busy = false;
  bool _disposed = false;
  int _read = 0;
  Future<void>? _continuationRead;
  int _viewerIntent = 0;
  ({String message, Future<void> Function()? retry})? _problem;

  bool get isActive => _ordinary != null;
  bool get canEnter => surface.library.query.collection is LibraryAllCollection;
  LibraryGalleryQuery? get ordinaryQuery => _ordinary?.query;
  LibraryReviewBrowseState? get ordinaryBrowse => _ordinary;
  bool get busy => _busy;
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

  Future<void> enter() => _run(() async {
    if (isActive || !canEnter) return;
    final saved = surface.captureBrowse();
    final query = saved.query.copyWith(
      organizationQueue: LibraryOrganizationQueue.pending,
    );
    if (!await surface.showReviewQuery(query) || _disposed) return;
    _ordinary = saved;
    _session = LibraryReviewSession(service, query);
    await refreshContinuation();
  });

  Future<void> exit() => _run(() async {
    final saved = _ordinary;
    if (saved == null) return;
    if (!await surface.showReviewQuery(saved.query, position: saved.position) ||
        _disposed) {
      return;
    }
    await _replaceSession(null);
    _ordinary = null;
    _continuation = null;
    surface.restoreReviewBrowse(saved);
  });

  bool restoreWorkspace(LibraryReviewBrowseState ordinary) {
    final query = surface.library.query;
    if (_disposed ||
        _busy ||
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
    await _run(() async {
      if (!isActive || requested.collection is! LibraryAllCollection) return;
      final query = requested.copyWith(
        organizationQueue: requested.organizationQueue ?? queue,
      );
      final browse = surface.captureBrowse();
      final onlyOrderingChanged =
          browse.query.copyWith(
            sortKey: query.sortKey,
            sortDirection: query.sortDirection,
          ) ==
          query;
      if (!await surface.showReviewQuery(
            query,
            position: onlyOrderingChanged ? browse.position : null,
          ) ||
          _disposed) {
        return;
      }
      applied = true;
      await _replaceSession(query);
      await refreshContinuation();
    });
    return applied;
  }

  Future<void> selectQueue(LibraryOrganizationQueue queue) async {
    await changeQuery(surface.library.query.copyWith(organizationQueue: queue));
  }

  Future<void> showConflicts() => _run(() async {
    final saved = _ordinary ?? surface.captureBrowse();
    const query = LibraryGalleryQuery(
      organizationQueue: LibraryOrganizationQueue.deferred,
    );
    if (!await surface.showReviewQuery(query) || _disposed) return;
    surface.closeReviewImage();
    _ordinary = saved;
    await _replaceSession(query);
    await refreshContinuation();
  });

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
    _read++;
    return _continuationRead ??= _readContinuation();
  }

  Future<void> _readContinuation() async {
    try {
      while (isActive && !_disposed) {
        final read = _read;
        try {
          final result = await service.continuation(
            query: surface.library.query,
          );
          if (_disposed || !isActive) return;
          if (read != _read) continue;
          _continuation = result;
        } on Object {
          if (_disposed || !isActive) return;
          if (read != _read) continue;
          _problem = (message: "无法读取整理位置，请重试。", retry: refreshContinuation);
        }
        notifyListeners();
        return;
      }
    } finally {
      _continuationRead = null;
    }
  }

  Future<void> resume() => _run(() async {
    if (!isActive) return;
    final saved = await service.continuation(query: surface.library.query);
    if (_disposed) return;
    final result = saved.forScope(surface.library.query);
    _continuation = result;
    if (result.canResume) await _showContinuation(result);
  });

  Future<void> afterSave(LibraryReviewEntry entry) => _run(() async {
    final session = _session;
    if (session == null || !session.owns(entry) || !entry.committed) return;
    final saved = await service.continuation(query: surface.library.query);
    if (_disposed || !session.owns(entry)) return;
    final result = saved.forScope(surface.library.query);
    _continuation = result;
    if (result.canResume) {
      await _showContinuation(result);
    } else {
      closeCurrent();
      surface.closeReviewImage();
    }
  });

  Future<void> _showContinuation(LibraryReviewContinuation result) async {
    final intent = _viewerIntent;
    final query = result.query!;
    final target = result.target!;
    if (!await surface.showReviewQuery(query, target: target) || _disposed) {
      throw StateError("The review query could not be restored");
    }
    if (intent != _viewerIntent) return;
    if (_session?.query != query) await _replaceSession(query);
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

  Future<void> _replaceSession(LibraryGalleryQuery? query) async {
    final previous = _session;
    _session = null;
    await previous?.close();
    if (!_disposed && query != null) {
      _session = LibraryReviewSession(service, query);
    }
  }

  Future<void> _run(Future<void> Function() operation) async {
    if (_disposed || _busy) return;
    _busy = true;
    _problem = null;
    notifyListeners();
    try {
      await operation();
    } on Object {
      if (!_disposed) {
        _problem = (message: "无法恢复整理位置，请重试。", retry: () => _run(operation));
      }
    } finally {
      if (!_disposed) {
        _busy = false;
        notifyListeners();
      }
    }
  }

  @override
  void dispose() {
    _disposed = true;
    _read++;
    unawaited(
      _session?.close().catchError((Object error) {
        debugPrint("[Ame organization] review retirement failed: $error");
      }),
    );
    super.dispose();
  }
}
