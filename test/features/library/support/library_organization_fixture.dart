import "dart:async";

import "package:cedarflake_ame/features/library/application/library_controller.dart";
import "package:cedarflake_ame/features/library/application/library_organization.dart";
import "package:cedarflake_ame/features/library/domain/library_organization.dart";
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
  final writes =
      <({String id, LibraryOrganizationPrepared prepared, bool favorite})>[];
  final undos = <({String id, String originalId, BigInt revision})>[];
  int preparationCount = 0;
  bool isFavorite = false;
  Future<LibraryOrganizationInspection> Function(LibraryOrganizationTarget)?
  inspectTarget;
  Completer<LibraryOrganizationPrepared>? preparing;
  Future<LibraryOrganizationReceipt> Function(String)? save;
  Future<LibraryOrganizationReceipt> Function(String)? undoResult;

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
