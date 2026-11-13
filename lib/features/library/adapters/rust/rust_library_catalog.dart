import "../../../../src/rust/api/catalog.dart" as rust_api;
import "../../../../src/rust/domain.dart" as rust_domain;
import "../../../../src/rust/domain/gallery_query_snapshot.dart" as rust_query;
import "../../../../src/rust/domain/gallery_scroll_section.dart" as rust_scroll;
import "../../../../src/rust/domain/gallery_time_snapshot.dart" as rust_time;
import "../../../../src/rust/domain/exact_duplicates.dart" as rust_exact;
import "../../domain/library_folder_models.dart";
import "../../domain/library_gallery_group.dart";
import "../../domain/library_models.dart";
import "../../domain/library_query_snapshot.dart";
import "../../domain/library_time_snapshot.dart";
import "../../domain/library_group_members.dart";
import "../../domain/library_exact_duplicates.dart";
import "../../domain/library_similar_review.dart";
import "library_asset_mapping.dart";
import "library_gallery_group_mapping.dart";
import "../../application/ports/library_catalog.dart";
import "library_gallery_query_mapping.dart";
import "library_organization_counts_mapping.dart";
import "rust_library_layout_manifest_catalog.dart";

class RustLibraryCatalog
    implements
        LibraryCatalog,
        LibraryFolderCatalog,
        LibraryQueryAnchorCatalog,
        LibraryStableQueryAnchorCatalog,
        LibraryGroupMemberCatalog,
        LibraryStableAssetCatalog {
  const RustLibraryCatalog();

  @override
  Future<LibraryGroupMemberPage> loadGroupMembers({
    required LibraryGalleryQuery query,
    required LibraryGalleryGroupKey group,
    String? after,
  }) async {
    try {
      final page = await rust_api.loadLibraryGalleryGroupMembers(
        query: _mapQuery(query),
        group: mapGalleryGroupKeyToRust(group)!,
        after: after,
      );
      return LibraryGroupMemberPage(
        members: [
          for (final member in page.members)
            LibraryGroupMember(
              asset: mapRustLibraryAsset(member.source),
              preferred: member.preferred,
              availability: switch (member.availability) {
                rust_exact.DuplicateLocationAvailability.local =>
                  LibraryDuplicateAvailability.local,
                rust_exact.DuplicateLocationAvailability.cloudOnly =>
                  LibraryDuplicateAvailability.cloudOnly,
                rust_exact.DuplicateLocationAvailability.offline =>
                  LibraryDuplicateAvailability.offline,
                rust_exact.DuplicateLocationAvailability.unavailable =>
                  LibraryDuplicateAvailability.unavailable,
              },
              content: LibrarySimilarContent(
                scheme: member.content.scheme,
                digest: member.content.digest,
                byteLength: member.content.byteLength,
              ),
            ),
        ],
        total: page.total.toInt(),
        pending: page.pending.toInt(),
        next: page.nextLocationId,
        revision: page.decisionRevision,
      );
    } on Object catch (error) {
      throw _mapFailure(error, "bridge_group_members_load_failed");
    }
  }

  @override
  Future<LibraryTimeSnapshot> resolveTimeIntent({
    required int maxItems,
    required LibraryGalleryQuery query,
    required LibraryTimeIntent intent,
  }) async {
    try {
      final result = await rust_api.loadLibraryTimeSnapshot(
        maxItems: maxItems,
        query: _mapQuery(query),
        intent: rust_time.GalleryTimeIntent(
          monthKey: intent.monthKey,
          itemOffset: BigInt.from(intent.itemOffset),
        ),
      );
      final anchor = result.anchor;
      return LibraryTimeSnapshot(
        snapshot: _mapSnapshot(result.snapshot),
        timeline: _mapTimeline(result.timeline),
        anchor: anchor == null
            ? null
            : LibraryTimeAnchor(
                revision: anchor.revision,
                queryId: anchor.queryId,
                monthKey: anchor.monthKey,
                itemOffset: anchor.itemOffset.toInt(),
              ),
        windowStartItemOffset: result.windowStartOrdinal.toInt(),
        layoutManifest: mapRustGalleryLayoutManifest(result.layoutManifest),
      );
    } on Object catch (error) {
      throw _mapFailure(error, "bridge_time_intent_load_failed");
    }
  }

  @override
  Future<LibraryQuerySnapshot> loadQuerySnapshot({
    required int maxItems,
    required LibraryGalleryQuery query,
    LibraryQueryAnchor? anchor,
  }) async {
    try {
      final result = await rust_api.loadLibraryQuerySnapshot(
        maxItems: maxItems,
        query: _mapQuery(query),
        anchor: anchor == null
            ? null
            : rust_query.GalleryQueryAnchor(
                requestedLocationId: anchor.requestedLocationId,
                groupKey: mapGalleryGroupKeyToRust(anchor.groupKey),
                assetId: anchor.assetId,
                fallbackOrdinal: BigInt.from(anchor.fallbackGlobalItemIndex),
              ),
      );
      final counts = result.organizationCounts;
      return LibraryQuerySnapshot(
        snapshot: _mapSnapshot(result.snapshot),
        timeline: _mapTimeline(result.timeline),
        organizationCounts: counts == null
            ? null
            : mapRustOrganizationCounts(counts),
        layoutManifest: mapRustGalleryLayoutManifest(result.layoutManifest),
      );
    } on Object catch (error) {
      throw _mapFailure(error, "bridge_query_snapshot_load_failed");
    }
  }

  @override
  Future<LibrarySnapshot> load({
    required int maxItems,
    required LibraryGalleryQuery query,
    LibraryCatalogCursor? after,
    LibraryCatalogCursor? before,
  }) async {
    try {
      final Future<rust_domain.CatalogSnapshot> pendingSnapshot = rust_api
          .loadLibraryCatalog(
            maxItems: maxItems,
            query: _mapQuery(query),
            after: after == null
                ? null
                : rust_domain.CatalogCursor(
                    revision: after.revision,
                    queryId: after.queryId,
                    primaryMissing: after.primaryMissing,
                    primaryText: after.primaryText,
                    primaryNumber: after.primaryNumber,
                    rootId: after.rootId,
                    locationId: after.locationId,
                    groupKey: mapGalleryGroupKeyToRust(after.groupKey),
                  ),
            before: before == null
                ? null
                : rust_domain.CatalogCursor(
                    revision: before.revision,
                    queryId: before.queryId,
                    primaryMissing: before.primaryMissing,
                    primaryText: before.primaryText,
                    primaryNumber: before.primaryNumber,
                    rootId: before.rootId,
                    locationId: before.locationId,
                    groupKey: mapGalleryGroupKeyToRust(before.groupKey),
                  ),
          );
      final snapshot = await pendingSnapshot;
      return _mapSnapshot(snapshot);
    } on Object catch (error) {
      throw _mapFailure(error, "bridge_catalog_load_failed");
    }
  }

  @override
  Future<LibraryTimeline> loadTimeline(LibraryGalleryQuery query) async {
    try {
      final timeline = await rust_api.loadLibraryGalleryTimeline(
        query: _mapQuery(query),
      );
      return _mapTimeline(timeline);
    } on Object catch (error) {
      throw _mapFailure(error, "bridge_timeline_load_failed");
    }
  }

  @override
  Future<LibraryFolderPage> loadFolderPage({
    required String rootId,
    required String parentRelativePath,
    required int maxItems,
    LibraryFolderCursor? after,
  }) async {
    try {
      final Future<rust_domain.LibraryFolderPage> pendingPage = rust_api
          .loadLibraryFolderPage(
            rootId: rootId,
            parentRelativePath: parentRelativePath,
            maxItems: maxItems,
            after: after == null
                ? null
                : rust_domain.LibraryFolderCursor(
                    revision: after.revision,
                    rootId: after.rootId,
                    parentRelativePath: after.parentRelativePath,
                    relativePath: after.relativePath,
                  ),
          );
      final page = await pendingPage;
      final nextCursor = page.nextCursor;
      return LibraryFolderPage(
        revision: page.revision,
        rootId: page.rootId,
        parentRelativePath: page.parentRelativePath,
        disposition: switch (page.disposition) {
          rust_domain.LibraryFolderPageDisposition.replace =>
            LibraryFolderPageDisposition.replace,
          rust_domain.LibraryFolderPageDisposition.append =>
            LibraryFolderPageDisposition.append,
        },
        folders: List.unmodifiable(
          page.folders.map(
            (folder) => LibraryFolder(
              rootId: folder.rootId,
              relativePath: folder.relativePath,
              name: folder.name,
              directAssetCount: folder.directAssetCount.toInt(),
              descendantAssetCount: folder.descendantAssetCount.toInt(),
            ),
          ),
        ),
        nextCursor: nextCursor == null
            ? null
            : LibraryFolderCursor(
                revision: nextCursor.revision,
                rootId: nextCursor.rootId,
                parentRelativePath: nextCursor.parentRelativePath,
                relativePath: nextCursor.relativePath,
              ),
      );
    } on Object catch (error) {
      throw _mapFailure(error, "bridge_folder_page_load_failed");
    }
  }

  @override
  Future<LibrarySnapshot> loadAtTime({
    required int maxItems,
    required LibraryGalleryQuery query,
    required LibraryTimeAnchor anchor,
  }) async {
    try {
      final Future<rust_domain.CatalogSnapshot> pendingSnapshot = rust_api
          .loadLibraryCatalogAtTime(
            maxItems: maxItems,
            query: _mapQuery(query),
            anchor: rust_domain.GalleryTimeAnchor(
              revision: anchor.revision,
              queryId: anchor.queryId,
              monthKey: anchor.monthKey,
              itemOffset: BigInt.from(anchor.itemOffset),
            ),
          );
      final snapshot = await pendingSnapshot;
      return _mapSnapshot(snapshot);
    } on Object catch (error) {
      throw _mapFailure(error, "bridge_time_anchor_load_failed");
    }
  }

  @override
  Future<LibrarySnapshot> loadAroundLocation({
    required int maxItems,
    required LibraryGalleryQuery query,
    required String anchorLocationId,
  }) async {
    try {
      final snapshot = await rust_api.loadLibraryCatalogAroundLocation(
        maxItems: maxItems,
        query: _mapQuery(query),
        anchorLocationId: anchorLocationId,
      );
      return _mapSnapshot(snapshot);
    } on Object catch (error) {
      throw _mapFailure(error, "bridge_location_anchor_load_failed");
    }
  }

  @override
  Future<LibrarySnapshot> loadAroundAsset({
    required int maxItems,
    required LibraryGalleryQuery query,
    required String requestedLocationId,
    required String anchorAssetId,
    required int fallbackGlobalItemIndex,
  }) async {
    try {
      final snapshot = await rust_api.loadLibraryCatalogAroundAsset(
        maxItems: maxItems,
        query: _mapQuery(query),
        requestedLocationId: requestedLocationId,
        anchorAssetId: anchorAssetId,
        fallbackOrdinal: BigInt.from(fallbackGlobalItemIndex),
      );
      return _mapSnapshot(snapshot);
    } on Object catch (error) {
      throw _mapFailure(error, "bridge_asset_anchor_load_failed");
    }
  }

  @override
  Future<LibraryAsset?> loadAssetById({
    required String assetId,
    String? preferredLocationId,
  }) async {
    try {
      final asset = await rust_api.loadLibraryAssetById(
        assetId: assetId,
        preferredLocationId: preferredLocationId,
      );
      return asset == null ? null : mapRustLibraryAsset(asset);
    } on Object catch (error) {
      throw _mapFailure(error, "bridge_asset_identity_load_failed");
    }
  }

  @override
  Future<bool> unregisterRoot(String rootId) async {
    try {
      return await rust_api.removeLibraryRoot(rootId: rootId);
    } on Object catch (error) {
      throw _mapFailure(error, "bridge_root_unregister_failed");
    }
  }

  LibrarySnapshot _mapSnapshot(rust_domain.CatalogSnapshot snapshot) {
    return LibrarySnapshot(
      catalogPath: snapshot.catalogPath,
      revision: snapshot.revision,
      queryId: snapshot.queryId,
      roots: List.unmodifiable(snapshot.roots.map(_mapRoot)),
      assets: List.unmodifiable(
        snapshot.assets.map(
          (entry) => mapRustLibraryAsset(
            entry.source,
            galleryGroup: mapRustGalleryGroup(entry.group),
          ),
        ),
      ),
      previousCursor: _mapCursor(snapshot.previousCursor),
      nextCursor: _mapCursor(snapshot.nextCursor),
      queryAnchorResolution: snapshot.queryAnchorResolution == null
          ? null
          : LibraryQueryAnchorResolution(
              requestedLocationId:
                  snapshot.queryAnchorResolution!.requestedLocationId,
              locationId: snapshot.queryAnchorResolution!.locationId,
              ordinal: snapshot.queryAnchorResolution!.ordinal?.toInt(),
              windowStartItemOffset: snapshot
                  .queryAnchorResolution!
                  .windowStartOrdinal
                  .toInt(),
            ),
    );
  }

  LibraryTimeline _mapTimeline(rust_domain.GalleryTimeline timeline) {
    return LibraryTimeline(
      revision: timeline.revision,
      queryId: timeline.queryId,
      totalItems: timeline.totalItems.toInt(),
      totalSourceItems: timeline.totalSourceItems.toInt(),
      scrollSections: List.unmodifiable([
        for (final section in timeline.scrollSections)
          LibraryScrollSection(
            kind: switch (section.kind) {
              rust_scroll.GalleryScrollSectionKind.name =>
                LibraryScrollSectionKind.name,
              rust_scroll.GalleryScrollSectionKind.newArrivals =>
                LibraryScrollSectionKind.newArrivals,
              rust_scroll.GalleryScrollSectionKind.earlier =>
                LibraryScrollSectionKind.earlier,
            },
            key: section.key,
            startOrdinal: section.startOrdinal.toInt(),
            itemCount: section.itemCount.toInt(),
          ),
      ]),
      buckets: List.unmodifiable(
        timeline.buckets.map(
          (bucket) => LibraryTimeBucket(
            monthKey: bucket.monthKey,
            itemCount: bucket.itemCount.toInt(),
            aspectRatioSum: bucket.aspectRatioMilliSum.toInt() / 1000,
          ),
        ),
      ),
    );
  }

  LibraryCatalogFailure _mapFailure(Object error, String fallbackCode) {
    if (error case rust_domain.ScanError(:final code, :final message)) {
      return LibraryCatalogFailure(code: code, message: message);
    }
    return LibraryCatalogFailure(code: fallbackCode, message: error.toString());
  }

  LibraryCatalogCursor? _mapCursor(rust_domain.CatalogCursor? cursor) {
    if (cursor == null) {
      return null;
    }
    return LibraryCatalogCursor(
      revision: cursor.revision,
      queryId: cursor.queryId,
      primaryMissing: cursor.primaryMissing,
      primaryText: cursor.primaryText,
      primaryNumber: cursor.primaryNumber,
      rootId: cursor.rootId,
      locationId: cursor.locationId,
      groupKey: mapRustGalleryGroupKey(cursor.groupKey),
    );
  }

  rust_domain.GalleryQuery _mapQuery(LibraryGalleryQuery query) {
    return mapLibraryGalleryQueryToRust(query);
  }

  LibraryRoot _mapRoot(rust_domain.LibraryRootView root) {
    return LibraryRoot(
      id: root.rootId,
      path: root.path,
      displayPath: root.displayPath,
      activeScanId: root.activeScanId,
      createdUnixMs: root.createdUnixMs,
      assetCount: root.assetCount.toInt(),
      issueCount: root.issueCount.toInt(),
      availability: switch (root.availability) {
        rust_domain.LibraryRootAvailability.unknown =>
          LibraryRootAvailability.unknown,
        rust_domain.LibraryRootAvailability.available =>
          LibraryRootAvailability.available,
        rust_domain.LibraryRootAvailability.missing =>
          LibraryRootAvailability.missing,
        rust_domain.LibraryRootAvailability.inaccessible =>
          LibraryRootAvailability.inaccessible,
        rust_domain.LibraryRootAvailability.offline =>
          LibraryRootAvailability.offline,
      },
      availabilityMessage: root.availabilityMessage,
    );
  }
}
