import "../domain/library_organization.dart";
import "../domain/library_organization_batch.dart";
import "../domain/library_review.dart";
import "library_album_requests.dart";
import "library_organization.dart";
import "library_organization_requests.dart";
import "library_selection_session.dart";
import "library_review_requests.dart";

abstract class _BatchRequest extends LibraryOrganizationRequest {
  _BatchRequest(this.session, this.action);
  final LibrarySelectionSession session;
  final LibraryBatchAction action;

  @override
  Future<LibraryOrganizationReceipt> execute(
    LibraryOrganization service,
  ) async {
    requireActive();
    try {
      return await session.apply(id, action);
    } on LibraryOrganizationFailure catch (error) {
      if (error.isStale) {
        throw const LibraryOrganizationFailure(
          "organization_selection_expired",
          "Select the images again before saving",
        );
      }
      rethrow;
    }
  }
}

class LibraryBatchFavoriteRequest extends _BatchRequest {
  LibraryBatchFavoriteRequest(LibrarySelectionSession session, this.favorite)
    : super(session, LibraryBatchFavorite(favorite));
  final bool favorite;
  @override
  String get successMessage => favorite ? "已加入收藏夹" : "已取消收藏";

  @override
  void cancel() {
    super.cancel();
    session.dispose();
  }
}

class LibraryBatchProgressRequest extends _BatchRequest {
  LibraryBatchProgressRequest(LibrarySelectionSession session, this.progress)
    : super(session, LibraryBatchProgressChange(progress));
  final LibraryProgressAction progress;
  @override
  String get successMessage => libraryProgressMessage(progress);
  @override
  void cancel() {
    super.cancel();
    session.dispose();
  }
}

class LibraryBatchAlbumMembership implements LibraryAlbumMembership {
  LibraryBatchAlbumMembership(this.session);
  final LibrarySelectionSession session;
  @override
  bool get supportsCancellation => true;
  _BatchAlbumRequest? _attempt;

  @override
  LibraryOrganizationRequest request(LibraryOrganizationAlbum? album) {
    final existing = _attempt;
    if (existing != null && existing.album?.id == album?.id) return existing;
    existing?.cancel();
    return _attempt = _BatchAlbumRequest(session, album);
  }

  @override
  Future<void> albumCreated() async {
    try {
      await session.refresh();
    } on LibraryOrganizationFailure catch (error) {
      if (error.isStale) {
        throw const LibraryOrganizationFailure(
          "organization_selection_expired",
          "Select the images again before saving",
        );
      }
      rethrow;
    }
  }

  @override
  void dispose() {
    _attempt?.cancel();
    session.dispose();
  }
}

class _BatchAlbumRequest extends _BatchRequest {
  _BatchAlbumRequest(LibrarySelectionSession session, this.album)
    : super(session, LibraryBatchAlbum(album?.id));
  final LibraryOrganizationAlbum? album;
  @override
  LibraryOrganizationErrorSurface get errorSurface =>
      LibraryOrganizationErrorSurface.form;
  @override
  String get successMessage {
    if (album == null) return "已移出相册";
    return session.summary?.assigned == 0
        ? "已加入“${album!.name}”"
        : "已移至“${album!.name}”";
  }
}
