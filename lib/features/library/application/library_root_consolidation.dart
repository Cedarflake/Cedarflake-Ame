import "../domain/library_models.dart";

LibraryRootConsolidation? recoverPublishedRootConsolidation({
  required List<LibraryRoot> previousRoots,
  required List<LibraryRoot> currentRoots,
  required LibraryRoot publishedParent,
}) {
  final currentIds = currentRoots.map((root) => root.id).toSet();
  final parent = _pathComponents(publishedParent.path);
  final covered = <LibraryConsolidatedRoot>[];
  for (final root in previousRoots) {
    if (currentIds.contains(root.id)) {
      continue;
    }
    final child = _pathComponents(root.path);
    if (child.length <= parent.length) {
      continue;
    }
    var contains = true;
    for (var index = 0; index < parent.length; index += 1) {
      if (parent[index] != child[index]) {
        contains = false;
        break;
      }
    }
    if (contains) {
      covered.add(
        LibraryConsolidatedRoot(
          previousRootId: root.id,
          relativePath: child.skip(parent.length).join("/"),
        ),
      );
    }
  }
  if (covered.isEmpty) {
    return null;
  }
  return LibraryRootConsolidation(
    destinationRootId: publishedParent.id,
    roots: List.unmodifiable(covered),
  );
}

List<String> _pathComponents(String path) {
  final components = path.replaceAll("\\", "/").split("/");
  if (components.isNotEmpty && components.last.isEmpty) {
    components.removeLast();
  }
  return components;
}

LibraryGalleryQuery resolveConsolidatedRootQuery(
  LibraryGalleryQuery query,
  LibraryRootConsolidation? consolidation,
) {
  if (consolidation == null || query.rootId == null) {
    return query;
  }
  for (final root in consolidation.roots) {
    if (root.previousRootId != query.rootId) {
      continue;
    }
    final folder = query.folderRelativePath;
    return query.copyWith(
      rootId: consolidation.destinationRootId,
      folderRelativePath: folder == null || folder.isEmpty
          ? root.relativePath
          : "${root.relativePath}/$folder",
    );
  }
  return query;
}
