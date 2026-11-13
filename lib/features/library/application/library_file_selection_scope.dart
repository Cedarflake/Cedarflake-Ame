import "../domain/library_file_operations.dart";
import "../domain/library_models.dart";
import "../domain/library_organization.dart";
import "../domain/library_organization_batch.dart";

LibraryFileOrganizationRule initialFileOrganizationRule({
  required LibraryFileAction action,
  required LibraryGalleryQuery query,
}) {
  if (action == LibraryFileAction.move &&
      query.collection is LibraryAlbumCollection) {
    return LibraryFileOrganizationRule.albumYear;
  }
  return LibraryFileOrganizationRule.direct;
}

LibraryFileSelection selectGalleryFilesForAction({
  required LibraryFileAction action,
  required LibraryOrganizationSelection selection,
  LibraryOrganizationTarget? location,
}) {
  final deletion =
      action == LibraryFileAction.recycle ||
      action == LibraryFileAction.deletePermanently;
  final query = selection.query;
  if (location != null && query.hasGroupFilter) {
    return LibraryFilesAtLocations([location]);
  }
  if (query.hasGroupFilter) return LibraryFilesInGallery(selection);
  if (deletion &&
      query.collection is LibraryAllCollection &&
      query.organizationQueue == null &&
      query.duplicateMode == LibraryGalleryDuplicateMode.merged) {
    return LibraryFileGroupsInGallery(selection);
  }
  if (location != null) return LibraryFilesAtLocations([location]);
  return LibraryFilesInGallery(selection);
}
