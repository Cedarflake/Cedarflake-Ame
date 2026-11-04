import "../../domain/library_organization.dart";
import "../../domain/library_organization_conflicts.dart";

abstract interface class LibraryOrganizationConflicts {
  Future<LibraryConflictStatus> status();
  Future<Set<String>> annotations(BigInt revision, List<String> locationIds);
  Future<LibraryConflictAlbums> albums(
    LibraryOrganizationTarget target, {
    String? after,
  });
}
