import "../application/library_classification_progress.dart";
import "library_organization_mode.dart";

/// Restores an explicit classification continuation through the shared gallery.
class LibraryClassificationContinuationNavigation {
  LibraryClassificationContinuationNavigation(this.service, this.surface);
  final LibraryClassificationProgress service;
  final LibraryReviewSurface surface;
  bool _disposed = false;
  Object? _request;

  Future<String?> resume() async {
    if (_disposed) return null;
    final request = Object();
    _request = request;
    final observedQuery = surface.library.query;
    final result = await service.continuation();
    if (!_owns(request) || surface.library.query != observedQuery) return null;
    final query = result.query;
    final target = result.target;
    if (query == null || target == null) return result.status.message;
    final shown = await surface.showReviewQuery(query, target: target);
    if (!_owns(request)) return null;
    if (!shown) {
      if (surface.library.query != observedQuery &&
          surface.library.query != query) {
        return null;
      }
      return "未能恢复分类确认位置，请重试";
    }
    if (surface.library.query != query) return null;
    final asset = surface.library.assets
        .where(
          (asset) =>
              asset.assetId == target.assetId &&
              asset.locationId == target.locationId &&
              asset.sourceGeneration == target.sourceGeneration,
        )
        .firstOrNull;
    if (asset == null) return "上次确认的图片已变化，请重新继续确认";
    surface.openReviewImage(asset);
    return null;
  }

  bool _owns(Object request) => !_disposed && identical(_request, request);

  void dispose() {
    _disposed = true;
    _request = null;
  }
}
