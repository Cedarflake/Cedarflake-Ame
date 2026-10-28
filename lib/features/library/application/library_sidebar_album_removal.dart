import "../domain/library_organization.dart";
import "library_album_requests.dart";
import "library_organization_actions.dart";
import "library_sidebar_removal.dart";

LibrarySidebarRemoval createSidebarAlbumRemoval({
  required List<LibraryOrganizationAlbum> albums,
  required BigInt revision,
  required LibraryOrganizationActions actions,
}) {
  var currentRevision = revision;
  LibraryAlbumEditSession? pending;
  return LibrarySidebarRemoval(
    entries: [
      for (final album in albums)
        LibrarySidebarRemovalEntry(album.id, album.name),
    ],
    remove: (id) async {
      final album = albums.singleWhere((album) => album.id == id);
      if (pending?.album?.id != id) {
        pending?.dispose();
        pending = LibraryAlbumEditSession(
          kind: LibraryAlbumEditKind.delete,
          revision: currentRevision,
          album: album,
        );
      }
      final result = await actions.submit(pending!.request(album.name));
      switch (result) {
        case LibraryOrganizationSaved(:final receipt):
          currentRevision = receipt.revision;
          return const LibrarySidebarRemovalOutcome(committed: true);
        case LibraryOrganizationRejected(:final failure):
          return LibrarySidebarRemovalOutcome(
            committed: false,
            error: failure.isStale
                ? "相册信息已更新，请重试剩余项目。"
                : "未能删除“${album.name}”，请重试。",
          );
        case LibraryOrganizationRetired():
          return const LibrarySidebarRemovalOutcome(
            committed: false,
            error: "操作已停止。",
          );
      }
    },
  );
}
