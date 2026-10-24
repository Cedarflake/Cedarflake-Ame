import "package:flutter/material.dart";
import "package:material_symbols_icons/symbols.dart";

import "../../../../app/presentation/ame_overlay_semantics.dart";
import "../../../../app/presentation/ame_typography.dart";
import "../../domain/library_gallery_collection.dart";

class LibraryCollectionNavigation extends StatelessWidget {
  const LibraryCollectionNavigation({
    required this.collection,
    required this.isCompact,
    required this.onFavorites,
    super.key,
  });
  final LibraryGalleryCollection collection;
  final bool isCompact;
  final VoidCallback? onFavorites;

  @override
  Widget build(BuildContext context) {
    final selected = collection is LibraryFavoritesCollection;
    if (isCompact) {
      return AmeTooltip(
        message: "收藏夹",
        child: IconButton(
          key: const Key("library-sidebar-favorites"),
          isSelected: selected,
          onPressed: onFavorites,
          icon: const Icon(Symbols.favorite_rounded),
        ),
      );
    }
    return ListTile(
      key: const Key("library-sidebar-favorites"),
      selected: selected,
      selectedTileColor: Theme.of(context).colorScheme.secondaryContainer,
      shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(12)),
      contentPadding: const EdgeInsets.only(left: 16, right: 4),
      minLeadingWidth: 24,
      horizontalTitleGap: 12,
      leading: const SizedBox(width: 24, child: Icon(Symbols.favorite_rounded)),
      title: Text(
        "收藏夹",
        style: Theme.of(context).textTheme.bodyLarge?.copyWith(
          fontWeight: selected ? ameFontWeightSemibold : ameFontWeightMedium,
        ),
      ),
      onTap: onFavorites,
    );
  }
}
