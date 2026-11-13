import "package:cedarflake_ame/features/library/application/ports/library_catalog.dart";
import "package:cedarflake_ame/features/library/domain/library_exact_duplicates.dart";
import "package:cedarflake_ame/features/library/domain/library_group_members.dart";
import "package:cedarflake_ame/features/library/domain/library_gallery_group.dart";
import "package:cedarflake_ame/features/library/domain/library_models.dart";

class LocationPageGroupCatalog implements LibraryGroupMemberCatalog {
  const LocationPageGroupCatalog(this.read);

  final Future<LibraryDuplicateLocationPage> Function(String? after) read;

  @override
  Future<LibraryGroupMemberPage> loadGroupMembers({
    required LibraryGalleryQuery query,
    required LibraryGalleryGroupKey group,
    String? after,
  }) async {
    final page = await read(after);
    return LibraryGroupMemberPage(
      total: page.locationCount ?? page.locations.length,
      pending: page.pendingLocationCount ?? 0,
      revision: BigInt.zero,
      next: page.nextLocationId,
      members: [
        for (final item in page.locations)
          LibraryGroupMember(
            preferred: false,
            availability: item.availability,
            asset: LibraryAsset(
              assetId: "asset-${item.locationId}",
              locationId: item.locationId,
              rootId: "root",
              activeScanId: "scan",
              sourcePath: item.displayPath,
              displayPath: item.displayPath,
              relativePath: "${item.locationId}.png",
              previewPath: "",
              previewStatus: LibraryPreviewStatus.failed,
              fileSize: BigInt.from(1024),
              modifiedUnixMs: 1,
              sourceRevision: null,
              sourceGeneration: BigInt.one,
              width: 240,
              height: 160,
            ),
          ),
      ],
    );
  }
}
