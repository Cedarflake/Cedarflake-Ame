import "../../settings/application/album_preferences.dart";
import "../domain/library_organization.dart";
import "library_organization.dart";

class LibraryAlbumProposal {
  const LibraryAlbumProposal({
    required this.current,
    required this.destination,
    required this.shouldAsk,
    required this.hasConflict,
  });
  final LibraryOrganizationAlbum? current;
  final LibraryOrganizationAlbum? destination;
  final bool shouldAsk;
  final bool hasConflict;
}

Future<LibraryAlbumProposal> prepareLibraryAlbumProposal({
  required LibraryOrganization service,
  required LibraryOrganizationTarget target,
  required AlbumPreferences preferences,
}) async {
  final inspection = await service.inspect([target]);
  final decision = inspection.decisions.single;
  return resolveLibraryAlbumProposal(
    service: service,
    preferences: preferences,
    current: decision.album,
    hasConflict: decision.hasConflict,
  );
}

Future<LibraryAlbumProposal> resolveLibraryAlbumProposal({
  required LibraryOrganization service,
  required AlbumPreferences preferences,
  required LibraryOrganizationAlbum? current,
  required bool hasConflict,
}) async {
  LibraryOrganizationAlbum? defaultAlbum;
  if (preferences.defaultAlbumId case final id?) {
    defaultAlbum = await service.findAlbum(id);
  }
  final shouldAsk =
      preferences.askBeforeAdding || defaultAlbum == null || hasConflict;
  var destination = defaultAlbum;
  if (shouldAsk) destination = current ?? defaultAlbum;
  if (hasConflict) destination = null;
  return LibraryAlbumProposal(
    current: current,
    destination: destination,
    shouldAsk: shouldAsk,
    hasConflict: hasConflict,
  );
}
