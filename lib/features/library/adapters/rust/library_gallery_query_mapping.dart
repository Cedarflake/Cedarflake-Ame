import "../../../../src/rust/domain.dart" as rust;
import "../../../../src/rust/domain/organization_queue.dart" as queue;
import "../../domain/library_models.dart";
import "library_gallery_collection_mapping.dart";
import "library_gallery_group_mapping.dart";
import "library_metadata_filter_mapping.dart";
import "library_semantic_mapping.dart";

LibraryGalleryQuery mapRustGalleryQuery(rust.GalleryQuery value) =>
    LibraryGalleryQuery(
      collection: mapRustCollection(value.collection),
      groupFilter: mapRustGalleryGroupFilter(value.groupFilter),
      organizationQueue: switch (value.organizationQueue) {
        null => null,
        queue.OrganizationQueue.pending => LibraryOrganizationQueue.pending,
        queue.OrganizationQueue.deferred_ => LibraryOrganizationQueue.deferred,
        queue.OrganizationQueue.ignored => LibraryOrganizationQueue.ignored,
      },
      rootId: value.rootId,
      newSinceAdmission: value.newSinceAdmission,
      folderRelativePath: value.folderRelativePath,
      includeDescendants: value.includeDescendants,
      searchText: value.searchText,
      semanticResults: value.semanticResults == null
          ? null
          : mapSemanticResults(value.semanticResults!),
      metadataFilter: mapRustMetadataFilter(value.metadataFilter),
      sortKey: switch (value.sortKey) {
        rust.GallerySortKey.captureTime => LibraryGallerySortKey.captureTime,
        rust.GallerySortKey.createdTime => LibraryGallerySortKey.createdTime,
        rust.GallerySortKey.modifiedTime => LibraryGallerySortKey.modifiedTime,
        rust.GallerySortKey.fileName => LibraryGallerySortKey.fileName,
        rust.GallerySortKey.libraryAdmission =>
          LibraryGallerySortKey.libraryAdmission,
      },
      sortDirection: value.sortDirection == rust.GallerySortDirection.ascending
          ? LibraryGallerySortDirection.ascending
          : LibraryGallerySortDirection.descending,
      duplicateMode: value.duplicateMode == rust.GalleryDuplicateMode.all
          ? LibraryGalleryDuplicateMode.all
          : LibraryGalleryDuplicateMode.merged,
    );

rust.GalleryQuery mapLibraryGalleryQueryToRust(LibraryGalleryQuery query) {
  return rust.GalleryQuery(
    collection: mapLibraryCollection(query.collection),
    groupFilter: mapGalleryGroupFilterToRust(query.groupFilter),
    organizationQueue: switch (query.organizationQueue) {
      null => null,
      LibraryOrganizationQueue.pending => queue.OrganizationQueue.pending,
      LibraryOrganizationQueue.deferred => queue.OrganizationQueue.deferred_,
      LibraryOrganizationQueue.ignored => queue.OrganizationQueue.ignored,
    },
    rootId: query.rootId,
    newSinceAdmission: query.newSinceAdmission,
    folderRelativePath: query.folderRelativePath,
    includeDescendants: query.includeDescendants,
    searchText: query.searchText,
    semanticResults: query.semanticResults == null
        ? null
        : mapSemanticResultsToRust(query.semanticResults!),
    metadataFilter: mapLibraryMetadataFilter(query.metadataFilter),
    sortKey: switch (query.sortKey) {
      LibraryGallerySortKey.captureTime => rust.GallerySortKey.captureTime,
      LibraryGallerySortKey.createdTime => rust.GallerySortKey.createdTime,
      LibraryGallerySortKey.modifiedTime => rust.GallerySortKey.modifiedTime,
      LibraryGallerySortKey.fileName => rust.GallerySortKey.fileName,
      LibraryGallerySortKey.libraryAdmission =>
        rust.GallerySortKey.libraryAdmission,
    },
    sortDirection: switch (query.sortDirection) {
      LibraryGallerySortDirection.ascending =>
        rust.GallerySortDirection.ascending,
      LibraryGallerySortDirection.descending =>
        rust.GallerySortDirection.descending,
    },
    duplicateMode: switch (query.duplicateMode) {
      LibraryGalleryDuplicateMode.all => rust.GalleryDuplicateMode.all,
      LibraryGalleryDuplicateMode.merged => rust.GalleryDuplicateMode.merged,
    },
  );
}
