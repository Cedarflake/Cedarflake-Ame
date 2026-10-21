import "../domain/library_models.dart";

enum LibraryViewerSourceNotice { changed, missing }

/// Keeps the displayed version until the user accepts a confirmed replacement.
class LibraryViewerSourceState {
  LibraryViewerSourceState(LibraryAsset asset) : _asset = asset;

  LibraryAsset _asset;
  LibraryAsset? _replacement;
  LibraryViewerSourceNotice? _notice;

  LibraryAsset get asset => _asset;
  LibraryAsset? get replacement => _replacement;
  LibraryViewerSourceNotice? get notice => _notice;

  void reconcile(LibraryAsset? latest) {
    if (latest == null) {
      _replacement = null;
      _notice = LibraryViewerSourceNotice.missing;
    } else if (_sameVersion(latest)) {
      _asset = latest;
      _replacement = null;
      _notice = null;
    } else {
      _replacement = latest;
      _notice = LibraryViewerSourceNotice.changed;
    }
  }

  bool _sameVersion(LibraryAsset latest) =>
      latest.assetId == asset.assetId &&
      latest.sourceGeneration == asset.sourceGeneration &&
      latest.fileSize == asset.fileSize &&
      latest.modifiedUnixMs == asset.modifiedUnixMs &&
      latest.sourceRevision == asset.sourceRevision &&
      latest.fileIdentity?.scheme == asset.fileIdentity?.scheme &&
      latest.fileIdentity?.value == asset.fileIdentity?.value;
}
