import "dart:math";

import "../../../app/messages/ame_message.dart";
import "../domain/library_organization.dart";
import "ports/library_organization.dart";
import "library_organization_publication.dart";

String newOrganizationOperationId() {
  final random = Random.secure();
  return List.generate(
    16,
    (_) => random.nextInt(256).toRadixString(16).padLeft(2, "0"),
  ).join();
}

enum LibraryOrganizationErrorSurface { notice, form }

abstract class LibraryOrganizationRequest {
  LibraryOrganizationRequest();
  final String feedbackId = newOrganizationOperationId();
  String id = newOrganizationOperationId();
  bool _cancelled = false;
  bool get isCancelled => _cancelled;
  AmeMessage get successMessage;
  AmeMessage get failureTitle => AmeLabel.changesSaveFailed;
  LibraryOrganizationChange get change =>
      const LibraryOrganizationChange.records();
  bool canUndo(LibraryOrganizationReceipt receipt) => receipt.changedGroups > 0;
  LibraryOrganizationErrorSurface get errorSurface =>
      LibraryOrganizationErrorSurface.notice;
  Future<LibraryOrganizationReceipt> execute(LibraryOrganization service);
  void cancel() => _cancelled = true;
  void requireActive() {
    if (_cancelled) {
      throw StateError("The organization request has been retired");
    }
  }

  void refreshAuthority() => id = newOrganizationOperationId();
}

class LibraryFavoriteRequest extends LibraryOrganizationRequest {
  LibraryFavoriteRequest(this.target, this.favorite);
  final LibraryOrganizationTarget target;
  final bool favorite;
  LibraryOrganizationPrepared? _prepared;

  @override
  AmeMessage get successMessage =>
      favorite ? AmeLabel.favoriteAdded : AmeLabel.favoriteRemoved;

  @override
  Future<LibraryOrganizationReceipt> execute(
    LibraryOrganization service,
  ) async {
    final prepared = _prepared ??= await service.prepare([target]);
    requireActive();
    return service.favorite(id, prepared, favorite);
  }

  @override
  void refreshAuthority() {
    super.refreshAuthority();
    _prepared = null;
  }
}

class LibraryOrganizationUndoRequest extends LibraryOrganizationRequest {
  LibraryOrganizationUndoRequest(this.originalId);
  final String originalId;
  BigInt? _revision;
  String? _retiredAlbumId;

  @override
  LibraryOrganizationChange get change {
    final albumId = _retiredAlbumId;
    return albumId == null
        ? const LibraryOrganizationChange.records()
        : LibraryOrganizationChange.albumRetired(albumId);
  }

  @override
  AmeMessage get successMessage => AmeLabel.undoCompleted;
  @override
  AmeMessage get failureTitle => AmeLabel.undoFailed;
  @override
  bool canUndo(LibraryOrganizationReceipt receipt) => false;

  @override
  Future<LibraryOrganizationReceipt> execute(
    LibraryOrganization service,
  ) async {
    final revision = _revision ??= await service.revision();
    requireActive();
    final result = await service.undo(id, originalId, revision);
    _retiredAlbumId = result.retiredAlbumId;
    return result.operation;
  }

  @override
  void refreshAuthority() {
    super.refreshAuthority();
    _revision = null;
  }
}
