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
  LibraryOrganizationAlbum? defaultAlbum;
  if (preferences.defaultAlbumId case final id?) {
    defaultAlbum = await service.findAlbum(id);
  }
  final shouldAsk =
      preferences.askBeforeAdding ||
      defaultAlbum == null ||
      decision.hasConflict;
  var destination = defaultAlbum;
  if (shouldAsk) destination = decision.album ?? defaultAlbum;
  if (decision.hasConflict) destination = null;
  return LibraryAlbumProposal(
    current: decision.album,
    destination: destination,
    shouldAsk: shouldAsk,
    hasConflict: decision.hasConflict,
  );
}
