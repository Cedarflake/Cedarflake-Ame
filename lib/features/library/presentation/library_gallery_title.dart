import "package:flutter_riverpod/flutter_riverpod.dart";

import "../domain/library_classification_configuration.dart";
import "../domain/library_models.dart";
import "../domain/library_state.dart";
import "library_strings.dart";
import "widgets/library_source_navigation_tile.dart" show librarySourceName;

class LibrarySmartAlbumHeading {
  LibraryClassificationCategory? _selected;

  void select(LibraryClassificationCategory? category) => _selected = category;

  String? resolve(
    String? categoryId,
    AsyncValue<List<LibraryClassificationCategory>> categories,
  ) {
    if (categoryId == null) return null;
    if (categories.hasValue) {
      for (final category
          in categories.value ?? const <LibraryClassificationCategory>[]) {
        if (category.id == categoryId) {
          _selected = category;
          return category.label;
        }
      }
      if (!categories.isLoading) {
        _selected = null;
        return null;
      }
    }
    return _selected?.id == categoryId ? _selected?.label : null;
  }
}

String libraryGalleryTitle(
  LibraryState state, {
  String? albumName,
  String? categoryName,
}) {
  final collectionName = switch (state.query.collection) {
    LibraryFavoritesCollection() => "收藏夹",
    LibraryAlbumCollection() => albumName ?? "相册",
    LibrarySmartAlbumCollection() => categoryName ?? "智能相册",
    LibraryAllCollection() => null,
  };
  final folder = state.query.folderRelativePath;
  if (folder != null) {
    final name = folder.split("/").last;
    return collectionName == null ? name : "$collectionName · $name";
  }
  final rootId = state.query.rootId;
  if (rootId == null) return collectionName ?? LibraryStrings.library;
  for (final root in state.roots) {
    if (root.id == rootId) {
      final name = librarySourceName(root.displayPath);
      return collectionName == null ? name : "$collectionName · $name";
    }
  }
  return LibraryStrings.library;
}
