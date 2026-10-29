import "package:flutter_riverpod/flutter_riverpod.dart";

import "../domain/library_organization.dart";

class LibraryAlbumHeading {
  LibraryOrganizationAlbum? _selected;

  void select(LibraryOrganizationAlbum album) => _selected = album;

  String? resolve(
    String albumId,
    AsyncValue<LibraryOrganizationAlbum?> details,
  ) {
    if (details.hasValue) {
      final album = details.value;
      if (_selected == null || _selected?.id == albumId) {
        _selected = album;
      }
      return album?.name;
    }
    return _selected?.id == albumId ? _selected?.name : null;
  }
}
