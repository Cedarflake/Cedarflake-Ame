import "../domain/library_file_operations.dart";
import "../domain/library_models.dart";
import "../domain/library_organization.dart";
import "../domain/library_organization_batch.dart";

LibraryFileSelection selectGalleryFilesForAction({
  required LibraryFileAction action,
  required LibraryOrganizationSelection selection,
  LibraryOrganizationTarget? location,
}) {
  final deletion =
      action == LibraryFileAction.recycle ||
      action == LibraryFileAction.deletePermanently;
  final query = selection.query;
  if (deletion &&
      query.collection is LibraryAllCollection &&
      query.organizationQueue == null &&
      query.duplicateMode == LibraryGalleryDuplicateMode.merged) {
    return LibraryFileGroupsInGallery(selection);
  }
  if (location != null) return LibraryFilesAtLocations([location]);
  return LibraryFilesInGallery(selection);
}
