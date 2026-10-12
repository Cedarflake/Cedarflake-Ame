import "../domain/library_state.dart";
import "library_strings.dart";

class LibraryPendingSource {
  const LibraryPendingSource._(this.path, this.status, this.publication);

  final String path;
  final LibraryStatus status;
  final LibraryScanPublication publication;

  static LibraryPendingSource? fromState(LibraryState state) {
    final scan = state.primaryScanSnapshot;
    final path = scan.displayRootPath;
    if (path == null || state.roots.any((root) => root.displayPath == path)) {
      return null;
    }
    return LibraryPendingSource._(path, scan.status, scan.publication);
  }

  bool get isProcessing => switch (status) {
    LibraryStatus.scanning ||
    LibraryStatus.pausing ||
    LibraryStatus.cancelling ||
    LibraryStatus.discarding ||
    LibraryStatus.refreshing => true,
    _ => false,
  };

  String get statusLabel {
    if (publication == LibraryScanPublication.reloadPending && !isProcessing) {
      return LibraryStrings.sourceDisplayReloadPending;
    }
    return switch (status) {
      LibraryStatus.scanning => LibraryStrings.addingSource,
      LibraryStatus.pausing => LibraryStrings.pausingSource,
      LibraryStatus.cancelling ||
      LibraryStatus.discarding => LibraryStrings.cancellingSource,
      LibraryStatus.refreshing => LibraryStrings.refreshingSourceDisplay,
      LibraryStatus.completed => LibraryStrings.sourceAdded,
      _ => LibraryStrings.firstImportIncomplete,
    };
  }
}
