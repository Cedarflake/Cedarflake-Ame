import "dart:async";

import "package:cedarflake_ame/features/library/application/library_controller.dart";
import "package:cedarflake_ame/features/library/application/library_organization.dart";
import "package:cedarflake_ame/features/library/domain/library_organization.dart";
import "package:cedarflake_ame/features/library/domain/library_organization_history.dart";
import "package:cedarflake_ame/features/library/domain/library_review.dart";
import "package:cedarflake_ame/features/library/domain/library_state.dart";

LibraryOrganizationTarget organizationTarget(String id) =>
    LibraryOrganizationTarget(
      assetId: "asset-$id",
      locationId: id,
      sourceGeneration: BigInt.one,
    );

LibraryOrganizationInspection organizationInspection(bool favorite) =>
    LibraryOrganizationInspection(
      revision: BigInt.one,
      decisions: [
        LibraryOrganizationDecision(
          favorite: favorite,
          disposition: LibraryOrganizationDisposition.pending,
        ),
      ],
    );

class OrganizationPreparedFixture extends LibraryOrganizationPrepared {
  const OrganizationPreparedFixture();
}

class OrganizationServiceFixture implements LibraryOrganization {
  final progressWrites =
      <
        ({
          String id,
          LibraryOrganizationPrepared prepared,
          LibraryProgressAction action,
        })
      >[];
  Future<LibraryOrganizationReceipt> Function(String)? saveProgress;

  @override
  Future<LibraryOrganizationReceipt> progress(
    String operationId,
    LibraryOrganizationPrepared prepared,
    LibraryProgressAction action,
  ) async {
    progressWrites.add((id: operationId, prepared: prepared, action: action));
    return saveProgress?.call(operationId) ?? receipt(operationId);
  }

  LibraryOrganizationHistory storedHistory = LibraryOrganizationHistory(
    revision: BigInt.two,
  );
  Future<LibraryOrganizationHistory> Function()? readHistory;
  final historySteps =
      <
        ({
          String id,
          LibraryOrganizationHistoryDirection direction,
          String originalId,
          BigInt revision,
        })
      >[];
  Future<LibraryOrganizationHistoryReceipt> Function(String)? saveHistory;

  @override
  Future<LibraryOrganizationHistory> history() async =>
      readHistory?.call() ?? storedHistory;

  @override
  Future<LibraryOrganizationHistoryReceipt> stepHistory(
    String operationId,
    LibraryOrganizationHistoryDirection direction,
    String originalId,
    BigInt revision,
  ) async {
    historySteps.add((
      id: operationId,
      direction: direction,
      originalId: originalId,
      revision: revision,
    ));
    return saveHistory?.call(operationId) ??
        LibraryOrganizationHistoryReceipt(operation: receipt(operationId));
  }

  final writes =
      <({String id, LibraryOrganizationPrepared prepared, bool favorite})>[];
  final undos = <({String id, String originalId, BigInt revision})>[];
  int preparationCount = 0;
  final preparedTargets = <List<LibraryOrganizationTarget>>[];
  bool isFavorite = false;
  Future<LibraryOrganizationInspection> Function(LibraryOrganizationTarget)?
  inspectTarget;
  Completer<LibraryOrganizationPrepared>? preparing;
  Future<LibraryOrganizationReceipt> Function(String)? save;
  Future<LibraryOrganizationReceipt> Function(String)? undoResult;
  final albumWrites =
      <({String id, BigInt revision, String? albumId, String? name})>[];
  final memberships =
      <({String id, LibraryOrganizationPrepared prepared, String? albumId})>[];
  List<LibraryOrganizationAlbum> albumItems = [];
  Future<LibraryOrganizationAlbumPage> Function(String?)? readAlbums;
  Future<LibraryOrganizationReceipt> Function(String)? saveAlbum;
  Future<LibraryOrganizationReceipt> Function(String)? saveMembership;

  LibraryOrganizationReceipt receipt(String id) => LibraryOrganizationReceipt(
    operationId: id,
    revision: BigInt.two,
    changedGroups: 1,
  );

  @override
  Future<LibraryOrganizationInspection> inspect(
    List<LibraryOrganizationTarget> targets,
  ) async =>
      inspectTarget?.call(targets.single) ?? organizationInspection(isFavorite);

  @override
  Future<LibraryOrganizationPrepared> prepare(
    List<LibraryOrganizationTarget> targets,
  ) async {
    preparationCount++;
    preparedTargets.add(List.unmodifiable(targets));
    return preparing?.future ?? const OrganizationPreparedFixture();
  }

  @override
  Future<LibraryOrganizationReceipt> favorite(
    String operationId,
    LibraryOrganizationPrepared prepared,
    bool favorite,
  ) async {
    writes.add((id: operationId, prepared: prepared, favorite: favorite));
    final result =
        await (save?.call(operationId) ?? Future.value(receipt(operationId)));
    isFavorite = favorite;
    return result;
  }

  @override
  Future<BigInt> revision() async => BigInt.two;

  @override
  Future<LibraryOrganizationAlbumPage> albums({String? after}) async =>
      readAlbums?.call(after) ??
      LibraryOrganizationAlbumPage(
        albums: albumItems,
        revision: BigInt.one,
        total: albumItems.length,
      );

  @override
  Future<LibraryOrganizationAlbum?> findAlbum(String id) async =>
      albumItems.where((album) => album.id == id).firstOrNull;

  @override
  Future<LibraryOrganizationReceipt> createAlbum(
    String operationId,
    BigInt revision,
    String name,
  ) => _album(operationId, revision, null, name);

  @override
  Future<LibraryOrganizationReceipt> renameAlbum(
    String operationId,
    BigInt revision,
    String albumId,
    String name,
  ) => _album(operationId, revision, albumId, name);

  @override
  Future<LibraryOrganizationReceipt> deleteAlbum(
    String operationId,
    BigInt revision,
    String albumId,
  ) => _album(operationId, revision, albumId, null);

  Future<LibraryOrganizationReceipt> _album(
    String id,
    BigInt revision,
    String? albumId,
    String? name,
  ) async {
    albumWrites.add((id: id, revision: revision, albumId: albumId, name: name));
    if (saveAlbum case final save?) return save(id);
    final album = name == null
        ? null
        : LibraryOrganizationAlbum(
            albumId ?? 'album-${albumItems.length}',
            name,
          );
    albumItems = [
      for (final item in albumItems)
        if (item.id != albumId) item,
      ?album,
    ];
    return LibraryOrganizationReceipt(
      operationId: id,
      revision: revision + BigInt.one,
      changedGroups: 0,
      album: album,
    );
  }

  @override
  Future<LibraryOrganizationReceipt> assignAlbum(
    String operationId,
    LibraryOrganizationPrepared prepared,
    String albumId,
  ) => _membership(operationId, prepared, albumId);

  @override
  Future<LibraryOrganizationReceipt> removeAlbum(
    String operationId,
    LibraryOrganizationPrepared prepared,
  ) => _membership(operationId, prepared, null);

  Future<LibraryOrganizationReceipt> _membership(
    String id,
    LibraryOrganizationPrepared prepared,
    String? albumId,
  ) async {
    memberships.add((id: id, prepared: prepared, albumId: albumId));
    return saveMembership?.call(id) ?? receipt(id);
  }

  @override
  Future<LibraryOrganizationReceipt> undo(
    String operationId,
    String originalId,
    BigInt revision,
  ) async {
    undos.add((id: operationId, originalId: originalId, revision: revision));
    final result =
        await (undoResult?.call(operationId) ??
            Future.value(receipt(operationId)));
    isFavorite = !isFavorite;
    return result;
  }
}

class OrganizationLibraryFixture extends LibraryController {
  int refreshes = 0;
  bool refreshSucceeds = true;

  @override
  LibraryState build() => LibraryState(catalogRevision: BigInt.one);

  @override
  Future<bool> refreshCurrentQuery() async {
    refreshes++;
    return refreshSucceeds;
  }
}
