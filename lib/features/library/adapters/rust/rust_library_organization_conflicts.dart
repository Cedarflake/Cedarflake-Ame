import "../../../../src/rust/api/organization_conflicts.dart" as api;
import "../../../../src/rust/domain/organization.dart" as dto;
import "../../domain/library_organization.dart";
import "../../domain/library_organization_conflicts.dart";
import "../../application/ports/library_organization_conflicts.dart";
import "library_organization_failure.dart";

class RustLibraryOrganizationConflicts implements LibraryOrganizationConflicts {
  const RustLibraryOrganizationConflicts();

  @override
  Future<LibraryConflictStatus> status() async {
    final result = await invokeOrganizationBridge(
      () => api.loadOrganizationConflictStatus(),
    );
    return LibraryConflictStatus(
      revision: result.catalogRevision,
      groups: result.conflictingGroups.toInt(),
    );
  }

  @override
  Future<Set<String>> annotations(
    BigInt revision,
    List<String> locationIds,
  ) async {
    final result = await invokeOrganizationBridge(
      () => api.loadOrganizationConflictAnnotations(
        revision: revision,
        locationIds: locationIds,
      ),
    );
    if (result.catalogRevision < revision) {
      throw StateError(
        "Conflict markers precede the requested catalog revision",
      );
    }
    return Set.unmodifiable(result.locationIds);
  }

  @override
  Future<LibraryConflictAlbums> albums(
    LibraryOrganizationTarget target, {
    String? after,
  }) async {
    final result = await invokeOrganizationBridge(
      () => api.loadOrganizationConflictAlbums(
        selection: dto.OrganizationSelection(
          assetId: target.assetId,
          locationId: target.locationId,
          sourceGeneration: target.sourceGeneration,
        ),
        after: after,
      ),
    );
    return LibraryConflictAlbums(
      revision: result.organizationRevision,
      nextAlbumId: result.nextAlbumId,
      albums: [
        for (final album in result.albums)
          LibraryConflictAlbum(
            id: album.albumId,
            name: album.name,
            retired: album.retired,
          ),
      ],
    );
  }
}
