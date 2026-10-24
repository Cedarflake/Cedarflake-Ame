import "../domain/library_organization.dart";
import "library_organization.dart";
import "library_organization_requests.dart";

enum LibraryAlbumEditKind { create, rename, delete }

class LibraryAlbumEditSession {
  LibraryAlbumEditSession({
    required this.kind,
    required this.revision,
    this.album,
  }) : assert(kind == LibraryAlbumEditKind.create || album != null);

  final LibraryAlbumEditKind kind;
  final BigInt revision;
  final LibraryOrganizationAlbum? album;
  LibraryAlbumEditRequest? _attempt;

  LibraryAlbumEditRequest request(String name) {
    final normalized = name.trim();
    if (_attempt?.name != normalized) {
      _attempt?.cancel();
      _attempt = LibraryAlbumEditRequest._(
        kind,
        album?.id,
        normalized,
        revision,
      );
    }
    return _attempt!;
  }

  void dispose() => _attempt?.cancel();
}

class LibraryAlbumEditRequest extends LibraryOrganizationRequest {
  LibraryAlbumEditRequest._(this.kind, this.albumId, this.name, BigInt revision)
    : _revision = revision;

  final LibraryAlbumEditKind kind;
  final String? albumId;
  final String name;
  BigInt? _revision;

  @override
  LibraryOrganizationErrorSurface get errorSurface =>
      LibraryOrganizationErrorSurface.form;

  @override
  String get successMessage => switch (kind) {
    LibraryAlbumEditKind.create => "已新建相册",
    LibraryAlbumEditKind.rename => "已重命名相册",
    LibraryAlbumEditKind.delete => "已删除相册",
  };

  @override
  bool canUndo(LibraryOrganizationReceipt receipt) =>
      _revision != null && receipt.revision > _revision!;

  @override
  Future<LibraryOrganizationReceipt> execute(
    LibraryOrganization service,
  ) async {
    final revision = _revision ??= await service.revision();
    requireActive();
    return switch (kind) {
      LibraryAlbumEditKind.create => service.createAlbum(id, revision, name),
      LibraryAlbumEditKind.rename => service.renameAlbum(
        id,
        revision,
        albumId!,
        name,
      ),
      LibraryAlbumEditKind.delete => service.deleteAlbum(
        id,
        revision,
        albumId!,
      ),
    };
  }

  @override
  void refreshAuthority() {
    super.refreshAuthority();
    _revision = null;
  }
}

class LibraryAlbumMembershipSession {
  LibraryAlbumMembershipSession(
    List<LibraryOrganizationTarget> targets, {
    required this.isMove,
  }) : targets = List.unmodifiable(targets);
  final List<LibraryOrganizationTarget> targets;
  final bool isMove;
  LibraryAlbumMembershipRequest? _attempt;

  LibraryAlbumMembershipRequest request(LibraryOrganizationAlbum? album) {
    final existing = _attempt;
    if (existing != null && existing.album?.id == album?.id) return existing;
    existing?.cancel();
    return _attempt = LibraryAlbumMembershipRequest(
      targets: targets,
      album: album,
      isMove: isMove,
    );
  }

  void dispose() => _attempt?.cancel();
}

class LibraryAlbumMembershipRequest extends LibraryOrganizationRequest {
  LibraryAlbumMembershipRequest({
    required this.targets,
    this.album,
    this.isMove = false,
  });

  final List<LibraryOrganizationTarget> targets;
  final LibraryOrganizationAlbum? album;
  final bool isMove;
  LibraryOrganizationPrepared? _prepared;

  @override
  LibraryOrganizationErrorSurface get errorSurface =>
      LibraryOrganizationErrorSurface.form;

  @override
  String get successMessage {
    final destination = album;
    if (destination == null) return "已移出相册";
    return isMove ? "已移至“${destination.name}”" : "已加入“${destination.name}”";
  }

  @override
  Future<LibraryOrganizationReceipt> execute(
    LibraryOrganization service,
  ) async {
    final prepared = _prepared ??= await service.prepare(targets);
    requireActive();
    final destination = album;
    if (destination == null) return service.removeAlbum(id, prepared);
    return service.assignAlbum(id, prepared, destination.id);
  }

  @override
  void refreshAuthority() {
    super.refreshAuthority();
    _prepared = null;
  }
}
