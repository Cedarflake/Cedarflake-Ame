import "../domain/library_models.dart";
import "library_preview_store.dart";

typedef LibraryPreviewSizeDemand = ({LibraryAsset asset, int previewEdge});
typedef _SizeObservation = ({
  LibraryPreviewSourceIdentity source,
  int previewEdge,
});

class LibraryPreviewSizing {
  final Map<String, _SizeObservation> _verified = {};
  final Map<String, _SizeObservation> _failed = {};

  bool needsVerification(LibraryAsset asset, int previewEdge) {
    final verified = _verified[asset.locationId];
    if (verified != null &&
        verified.source.isCompatibleWith(asset) &&
        verified.previewEdge >= previewEdge) {
      return false;
    }
    final failed = _failed[asset.locationId];
    return failed == null ||
        !failed.source.isCompatibleWith(asset) ||
        failed.previewEdge != previewEdge;
  }

  void recordVerified(LibraryAsset asset, int previewEdge) {
    final previous = _verified[asset.locationId];
    if (previous == null ||
        !previous.source.isCompatibleWith(asset) ||
        previous.previewEdge < previewEdge) {
      _verified[asset.locationId] = (
        source: LibraryPreviewSourceIdentity.fromAsset(asset),
        previewEdge: previewEdge,
      );
    }
    _failed.remove(asset.locationId);
  }

  void recordFailure(LibraryAsset asset, int previewEdge) {
    _failed[asset.locationId] = (
      source: LibraryPreviewSourceIdentity.fromAsset(asset),
      previewEdge: previewEdge,
    );
  }

  void retainDemand(Map<String, LibraryPreviewSizeDemand> demand) {
    _verified.removeWhere((locationId, verified) {
      final current = demand[locationId];
      return current == null ||
          !verified.source.isCompatibleWith(current.asset);
    });
    _failed.removeWhere((locationId, failed) {
      final current = demand[locationId];
      return current == null ||
          current.previewEdge != failed.previewEdge ||
          !failed.source.isCompatibleWith(current.asset);
    });
  }

  void allowRetry(String locationId) => _failed.remove(locationId);

  void invalidate(String locationId) {
    _verified.remove(locationId);
    _failed.remove(locationId);
  }

  void invalidateRoot(String rootId) {
    _verified.removeWhere(
      (_, observation) => observation.source.rootId == rootId,
    );
    _failed.removeWhere(
      (_, observation) => observation.source.rootId == rootId,
    );
  }

  void clear() {
    _verified.clear();
    _failed.clear();
  }
}
