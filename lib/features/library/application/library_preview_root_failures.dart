import "../domain/library_models.dart";
import "library_previewer.dart";

sealed class LibraryPreviewRootAdmission {
  const LibraryPreviewRootAdmission();
}

final class LibraryPreviewRootAllowed extends LibraryPreviewRootAdmission {
  const LibraryPreviewRootAllowed();
}

final class LibraryPreviewRootBlocked extends LibraryPreviewRootAdmission {
  const LibraryPreviewRootBlocked(this.failure);

  final LibraryPreviewFailure failure;
}

final class LibraryPreviewRootRecovering extends LibraryPreviewRootAdmission {
  const LibraryPreviewRootRecovering(this.asset);

  final LibraryAsset asset;
}

/// Owns root failure admission and its scan-scoped retry lifetime.
class LibraryPreviewRootFailures {
  LibraryPreviewRootFailures({
    required Duration unavailableCooldown,
    required this._isCurrentSource,
  }) : _unavailableCooldown = unavailableCooldown {
    if (unavailableCooldown.isNegative) {
      throw ArgumentError.value(
        unavailableCooldown,
        "rootUnavailableCooldown",
        "must not be negative",
      );
    }
  }

  final Duration _unavailableCooldown;
  final bool Function(LibraryAsset asset) _isCurrentSource;
  final Stopwatch _clock = Stopwatch()..start();
  final Map<String, _BlockedPreviewRoot> _roots = {};

  LibraryPreviewRootAdmission admit(
    LibraryAsset asset, {
    required bool explicitRetry,
  }) {
    final blocked = _roots[asset.rootId];
    if (blocked != null) {
      if (blocked.activeScanId == asset.activeScanId &&
          !explicitRetry &&
          !blocked.hasCooledDown(_clock.elapsed)) {
        return LibraryPreviewRootBlocked(blocked.failure);
      }
      _roots.remove(asset.rootId);
    }
    if (!explicitRetry &&
        asset.previewStatus == LibraryPreviewStatus.failed &&
        asset.previewIssueCode == "preview_root_unavailable") {
      return LibraryPreviewRootRecovering(
        asset.withPreview(
          previewPath: asset.previewPath,
          width: asset.width,
          height: asset.height,
          previewStatus: LibraryPreviewStatus.pending,
        ),
      );
    }
    return const LibraryPreviewRootAllowed();
  }

  bool record(LibraryAsset asset, LibraryPreviewFailure failure) {
    // A retired image can finish after the query or source changes. Its failure
    // cannot acquire authority over other images that still belong to this root.
    if (!_isCurrentSource(asset)) {
      return false;
    }
    _roots[asset.rootId] = _BlockedPreviewRoot(
      activeScanId: asset.activeScanId,
      failure: failure,
      retryAt: failure.code == "preview_root_unavailable"
          ? _clock.elapsed + _unavailableCooldown
          : null,
    );
    return true;
  }

  void clearRoot(String rootId) => _roots.remove(rootId);

  void clear() => _roots.clear();
}

class _BlockedPreviewRoot {
  const _BlockedPreviewRoot({
    required this.activeScanId,
    required this.failure,
    required this.retryAt,
  });

  final String activeScanId;
  final LibraryPreviewFailure failure;
  final Duration? retryAt;

  bool hasCooledDown(Duration now) {
    final retryAt = this.retryAt;
    return retryAt != null && now >= retryAt;
  }
}
