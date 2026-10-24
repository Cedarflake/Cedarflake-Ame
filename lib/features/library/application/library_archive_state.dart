import "../domain/library_organization_archive.dart";
import "library_archive_requests.dart";

sealed class LibraryArchiveState {
  const LibraryArchiveState();
}

final class LibraryArchiveIdle extends LibraryArchiveState {
  const LibraryArchiveIdle();
}

final class LibraryArchiveWorking extends LibraryArchiveState {
  const LibraryArchiveWorking(
    this.request, {
    this.phase,
    this.stopping = false,
    this.cancelFailed = false,
  });
  final LibraryArchiveRequest request;
  final LibraryArchivePhase? phase;
  final bool stopping;
  final bool cancelFailed;
}

final class LibraryArchiveReady extends LibraryArchiveState {
  const LibraryArchiveReady(this.path, this.inspection);
  final String path;
  final LibraryArchiveInspection inspection;
}

final class LibraryArchiveFailed extends LibraryArchiveState {
  const LibraryArchiveFailed(this.request, this.failure);
  final LibraryArchiveRequest request;
  final LibraryArchiveFailure failure;
}

final class LibraryArchiveFinished extends LibraryArchiveState {
  const LibraryArchiveFinished(this.request, {this.cancelled = false});
  final LibraryArchiveRequest request;
  final bool cancelled;
}
