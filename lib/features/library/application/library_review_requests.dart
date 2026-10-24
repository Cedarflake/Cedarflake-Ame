import "../domain/library_organization.dart";
import "../domain/library_review.dart";
import "library_album_requests.dart";
import "library_organization.dart";
import "library_organization_requests.dart";
import "library_review_session.dart";

class LibraryReviewRequest extends LibraryOrganizationRequest {
  LibraryReviewRequest(this.session, this.entry, this.action);
  final LibraryReviewSession session;
  final LibraryReviewEntry entry;
  final LibraryReviewAction action;
  LibraryOrganizationPrepared? _prepared;
  LibraryReviewPosition? _position;

  @override
  LibraryOrganizationErrorSurface get errorSurface =>
      LibraryOrganizationErrorSurface.form;

  @override
  String get successMessage => switch (action) {
    LibraryReviewAlbum(:final album) =>
      album == null ? "已移出相册" : "已加入“${album.name}”",
    LibraryReviewProgress(:final action) => libraryProgressMessage(action),
  };

  @override
  Future<LibraryOrganizationReceipt> execute(
    LibraryOrganization service,
  ) async {
    requireActive();
    session.requireCurrent(entry);
    final position = _position ??= await session.checkpoint(entry);
    final prepared = _prepared ??= await service.prepare([entry.target]);
    requireActive();
    session.requireCurrent(entry);
    final receipt = await session.service.change(
      id,
      position,
      prepared,
      action,
    );
    session.accept(entry, receipt);
    return receipt.operation;
  }

  @override
  void refreshAuthority() {
    super.refreshAuthority();
    _prepared = null;
  }
}

class LibraryReviewAlbumMembership implements LibraryAlbumMembership {
  LibraryReviewAlbumMembership(this.session, this.entry);
  final LibraryReviewSession session;
  final LibraryReviewEntry entry;
  LibraryReviewRequest? _attempt;
  String? _albumId;
  @override
  bool get supportsCancellation => false;

  @override
  LibraryOrganizationRequest request(LibraryOrganizationAlbum? album) {
    if (_attempt != null && _albumId == album?.id) return _attempt!;
    _attempt?.cancel();
    _albumId = album?.id;
    return _attempt = LibraryReviewRequest(
      session,
      entry,
      LibraryReviewAlbum(album),
    );
  }

  @override
  Future<void> albumCreated() async {}

  @override
  void dispose() => _attempt?.cancel();
}

String libraryProgressMessage(LibraryProgressAction action) => switch (action) {
  LibraryProgressAction.defer => "已移至稍后整理",
  LibraryProgressAction.ignore => "已忽略",
  LibraryProgressAction.restore => "已恢复待整理",
};
