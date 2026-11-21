import "package:material_ui/material_ui.dart";
import "package:material_symbols_icons/symbols.dart";

import "../../../../app/presentation/ame_localizations.dart";
import "../../../../app/presentation/ame_overlay_semantics.dart";
import "../../../../app/presentation/ame_typography.dart";
import "../../../../app/presentation/ame_workbench_surfaces.dart";
import "../../domain/library_gallery_collection.dart";
import "library_workbench_metrics.dart";

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
        message: context.l10n.favorites,
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
      selectedTileColor: AmeWorkbenchSurfaces.of(context).selection,
      selectedColor: AmeWorkbenchSurfaces.of(context).onSelection,
      shape: LibraryWorkbenchMetrics.sidebarRowShape,
      minTileHeight: LibraryWorkbenchMetrics.sidebarRowHeight,
      contentPadding: const EdgeInsets.only(left: 16, right: 4),
      minLeadingWidth: 24,
      horizontalTitleGap: 12,
      leading: const SizedBox(width: 24, child: Icon(Symbols.favorite_rounded)),
      title: Text(
        context.l10n.favorites,
        style: Theme.of(context).textTheme.bodyLarge?.copyWith(
          fontWeight: selected ? ameFontWeightSemibold : ameFontWeightRegular,
        ),
      ),
      onTap: onFavorites,
    );
  }
}
