import "../../../src/rust/domain.dart" as rust;
import "../../../src/rust/domain/organization_queue.dart" as queue;
import "../domain/library_models.dart";
import "library_gallery_collection_mapping.dart";
import "library_metadata_filter_mapping.dart";

LibraryGalleryQuery mapRustGalleryQuery(rust.GalleryQuery value) =>
    LibraryGalleryQuery(
      collection: mapRustCollection(value.collection),
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
