import "dart:async";
import "dart:collection";

import "../domain/library_models.dart";

class LibraryPreviewSourceIdentity {
  LibraryPreviewSourceIdentity.fromAsset(LibraryAsset asset)
    : assetId = asset.assetId,
      locationId = asset.locationId,
      rootId = asset.rootId,
      sourcePath = asset.fileIdentity == null ? asset.sourcePath : null,
      fileSize = asset.fileSize,
      modifiedUnixMs = asset.modifiedUnixMs,
      fileIdentityScheme = asset.fileIdentity?.scheme,
      fileIdentityValue = asset.fileIdentity?.value;

  final String assetId;
  final String locationId;
  final String rootId;
  final String? sourcePath;
  final BigInt fileSize;
  final int modifiedUnixMs;
  final String? fileIdentityScheme;
  final String? fileIdentityValue;

  bool isCompatibleWith(LibraryAsset asset) {
    return this == LibraryPreviewSourceIdentity.fromAsset(asset);
  }

  @override
  bool operator ==(Object other) {
    return other is LibraryPreviewSourceIdentity &&
        other.assetId == assetId &&
        other.locationId == locationId &&
        other.rootId == rootId &&
        other.sourcePath == sourcePath &&
        other.fileSize == fileSize &&
        other.modifiedUnixMs == modifiedUnixMs &&
        other.fileIdentityScheme == fileIdentityScheme &&
        other.fileIdentityValue == fileIdentityValue;
  }

  @override
  int get hashCode => Object.hash(
    assetId,
    locationId,
    rootId,
    sourcePath,
    fileSize,
    modifiedUnixMs,
    fileIdentityScheme,
    fileIdentityValue,
  );
}

bool libraryPreviewSourcesAreCompatible(
  LibraryAsset first,
  LibraryAsset second,
) {
  return LibraryPreviewSourceIdentity.fromAsset(first).isCompatibleWith(second);
}

class LibraryPreviewStore {
  LibraryPreviewStore({int maxRetainedFailures = 512})
    : assert(maxRetainedFailures > 0),
      _maxRetainedFailures = maxRetainedFailures;

  final int _maxRetainedFailures;
  final Map<String, _StoredLibraryPreview> _entries = {};
  final LinkedHashMap<String, _StoredLibraryPreview> _failures =
      LinkedHashMap();
  final Map<String, StreamController<void>> _channels = {};
  bool _isDisposed = false;

  LibraryAsset resolve(LibraryAsset fallback) {
    final stored =
        _entries[fallback.locationId] ?? _failures[fallback.locationId];
    if (stored == null || !stored.source.isCompatibleWith(fallback)) {
      return fallback;
    }
    final preview = stored.asset;
    return fallback.withPreview(
      previewPath: preview.previewPath,
      width: preview.width,
      height: preview.height,
      previewStatus: preview.previewStatus,
      previewIssueCode: preview.previewIssueCode,
      previewIssueMessage: preview.previewIssueMessage,
    );
  }

  Stream<void> changesFor(String locationId) {
    if (_isDisposed) {
      return const Stream<void>.empty();
    }
    return (_channels[locationId] ??= _createChannel(locationId)).stream;
  }

  StreamController<void> _createChannel(String locationId) {
    late final StreamController<void> channel;
    channel = StreamController<void>.broadcast(
      sync: true,
      onCancel: () {
        scheduleMicrotask(() {
          if (channel.hasListener ||
              !identical(_channels[locationId], channel)) {
            return;
          }
          _channels.remove(locationId);
          unawaited(channel.close());
        });
      },
    );
    return channel;
  }

  void publish(LibraryAsset asset) {
    if (_isDisposed) {
      return;
    }
    final stored = _StoredLibraryPreview(
      asset: asset,
      source: LibraryPreviewSourceIdentity.fromAsset(asset),
    );
    _entries[asset.locationId] = stored;
    if (asset.previewStatus == LibraryPreviewStatus.failed) {
      _failures.remove(asset.locationId);
      _failures[asset.locationId] = stored;
      while (_failures.length > _maxRetainedFailures) {
        final evictedId = _failures.keys.first;
        _failures.remove(evictedId);
        if (!_entries.containsKey(evictedId)) {
          _channels[evictedId]?.add(null);
        }
      }
    } else {
      _failures.remove(asset.locationId);
    }
    _channels[asset.locationId]?.add(null);
  }

  void retain(Iterable<String> locationIds) {
    if (_isDisposed) {
      return;
    }
    final retainedIds = locationIds.toSet();
    final removedIds = <String>[];
    _entries.removeWhere((locationId, entry) {
      final shouldRemove = !retainedIds.contains(locationId);
      if (shouldRemove && !_failures.containsKey(locationId)) {
        removedIds.add(locationId);
      }
      return shouldRemove;
    });
    for (final locationId in removedIds) {
      _channels[locationId]?.add(null);
    }
  }

  void clear() {
    if (_isDisposed || (_entries.isEmpty && _failures.isEmpty)) {
      return;
    }
    final removedIds = {..._entries.keys, ..._failures.keys};
    _entries.clear();
    _failures.clear();
    for (final locationId in removedIds) {
      _channels[locationId]?.add(null);
    }
  }

  void dispose() {
    if (_isDisposed) {
      return;
    }
    _isDisposed = true;
    _entries.clear();
    _failures.clear();
    final channels = _channels.values.toList(growable: false);
    _channels.clear();
    for (final channel in channels) {
      unawaited(channel.close());
    }
  }
}

class _StoredLibraryPreview {
  const _StoredLibraryPreview({required this.asset, required this.source});

  final LibraryAsset asset;
  final LibraryPreviewSourceIdentity source;
}
