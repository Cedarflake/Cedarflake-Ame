import "package:flutter_riverpod/flutter_riverpod.dart";

import "../domain/library_organization.dart";
import "library_organization.dart";
import "library_organization_actions.dart";

final libraryAlbumPageProvider = FutureProvider.autoDispose
    .family<LibraryOrganizationAlbumPage, String?>((ref, after) {
      ref.watch(
        libraryOrganizationActionsProvider.select((state) => state.sequence),
      );
      return ref.read(libraryOrganizationProvider).albums(after: after);
    });

final libraryAlbumProvider = FutureProvider.autoDispose
    .family<LibraryOrganizationAlbum?, String>((ref, id) {
      ref.watch(
        libraryOrganizationActionsProvider.select((state) => state.sequence),
      );
      return ref.read(libraryOrganizationProvider).findAlbum(id);
    });
