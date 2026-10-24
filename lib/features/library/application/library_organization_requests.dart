import "dart:math";

import "../domain/library_organization.dart";
import "library_organization.dart";

String newOrganizationOperationId() {
  final random = Random.secure();
  return List.generate(
    16,
    (_) => random.nextInt(256).toRadixString(16).padLeft(2, "0"),
  ).join();
}

sealed class LibraryOrganizationRequest {
  LibraryOrganizationRequest();
  final String feedbackId = newOrganizationOperationId();
  String id = newOrganizationOperationId();
  bool _cancelled = false;
  String get successMessage;
  bool get canUndo => true;
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
  String get successMessage => favorite ? "已加入收藏夹" : "已取消收藏";

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
  @override
  String get successMessage => "已撤销";
  @override
  bool get canUndo => false;

  @override
  Future<LibraryOrganizationReceipt> execute(
    LibraryOrganization service,
  ) async {
    final revision = _revision ??= await service.revision();
    requireActive();
    return service.undo(id, originalId, revision);
  }

  @override
  void refreshAuthority() {
    super.refreshAuthority();
    _revision = null;
  }
}
