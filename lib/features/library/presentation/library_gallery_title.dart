import "package:flutter_riverpod/flutter_riverpod.dart";

import "../../../app/presentation/ame_localizations.dart";
import "../domain/library_classification_configuration.dart";
import "../domain/library_models.dart";
import "../domain/library_state.dart";
import "library_category_label.dart";
import "widgets/library_source_navigation_tile.dart" show librarySourceName;

class LibrarySmartAlbumHeading {
  LibraryClassificationCategory? _selected;

  void select(LibraryClassificationCategory? category) => _selected = category;

  String? resolve(
    AppLocalizations strings,
    String? categoryId,
    AsyncValue<List<LibraryClassificationCategory>> categories,
  ) {
    if (categoryId == null) return null;
    if (categories.hasValue) {
      for (final category
          in categories.value ?? const <LibraryClassificationCategory>[]) {
        if (category.id == categoryId) {
          _selected = category;
          return libraryCategoryLabel(strings, category);
        }
      }
      if (!categories.isLoading) {
        _selected = null;
        return null;
      }
    }
    final selected = _selected;
    return selected != null && selected.id == categoryId
        ? libraryCategoryLabel(strings, selected)
        : null;
  }
}

String libraryGalleryTitle(
  AppLocalizations strings,
  LibraryState state, {
  String? albumName,
  String? categoryName,
}) {
  final collectionName = switch (state.query.collection) {
    LibraryFavoritesCollection() => strings.favorites,
    LibraryAlbumCollection() => albumName ?? strings.albums,
    LibrarySmartAlbumCollection() => categoryName ?? strings.smartAlbums,
    LibraryAllCollection() => null,
  };
  final folder = state.query.folderRelativePath;
  if (folder != null) {
    final name = folder.split("/").last;
    return collectionName == null ? name : "$collectionName · $name";
  }
  final rootId = state.query.rootId;
  if (rootId == null) return collectionName ?? strings.allImages;
  for (final root in state.roots) {
    if (root.id == rootId) {
      final name = librarySourceName(root.displayPath);
      return collectionName == null ? name : "$collectionName · $name";
    }
  }
  return strings.allImages;
}
