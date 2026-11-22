import "package:material_ui/material_ui.dart";
import "package:material_symbols_icons/symbols.dart";

import "../../../../app/presentation/ame_localizations.dart";
import "../../../../app/presentation/ame_overlay_semantics.dart";
import "../../domain/library_organization.dart";

IconData libraryAlbumIcon(LibraryAlbumIcon icon) => switch (icon) {
  LibraryAlbumIcon.album => Symbols.photo_album_rounded,
  LibraryAlbumIcon.travel => Symbols.flight_rounded,
  LibraryAlbumIcon.nature => Symbols.forest_rounded,
  LibraryAlbumIcon.pets => Symbols.pets_rounded,
  LibraryAlbumIcon.people => Symbols.group_rounded,
  LibraryAlbumIcon.food => Symbols.restaurant_rounded,
  LibraryAlbumIcon.art => Symbols.palette_rounded,
  LibraryAlbumIcon.work => Symbols.work_rounded,
  LibraryAlbumIcon.study => Symbols.school_rounded,
  LibraryAlbumIcon.games => Symbols.sports_esports_rounded,
  LibraryAlbumIcon.music => Symbols.music_note_rounded,
  LibraryAlbumIcon.memories => Symbols.celebration_rounded,
};

String libraryAlbumIconLabel(AppLocalizations strings, LibraryAlbumIcon icon) =>
    switch (icon) {
      LibraryAlbumIcon.album => strings.albums,
      LibraryAlbumIcon.travel => strings.albumIconTravel,
      LibraryAlbumIcon.nature => strings.albumIconNature,
      LibraryAlbumIcon.pets => strings.albumIconPets,
      LibraryAlbumIcon.people => strings.albumIconPeople,
      LibraryAlbumIcon.food => strings.albumIconFood,
      LibraryAlbumIcon.art => strings.albumIconArt,
      LibraryAlbumIcon.work => strings.albumIconWork,
      LibraryAlbumIcon.study => strings.albumIconStudy,
      LibraryAlbumIcon.games => strings.albumIconGames,
      LibraryAlbumIcon.music => strings.albumIconMusic,
      LibraryAlbumIcon.memories => strings.albumIconMemories,
    };

class LibraryAlbumIconPicker extends StatelessWidget {
  const LibraryAlbumIconPicker({
    required this.value,
    this.onChanged,
    super.key,
  });

  final LibraryAlbumIcon value;
  final ValueChanged<LibraryAlbumIcon>? onChanged;

  @override
  Widget build(BuildContext context) => ConstrainedBox(
    constraints: const BoxConstraints(maxWidth: 368),
    child: Wrap(
      spacing: 16,
      runSpacing: 8,
      children: [
        for (final icon in LibraryAlbumIcon.values)
          SizedBox.square(
            dimension: 48,
            child: AmeTooltip(
              message: libraryAlbumIconLabel(context.l10n, icon),
              child: IconButton.filledTonal(
                key: ValueKey("album-icon-${icon.name}"),
                isSelected: value == icon,
                onPressed: onChanged == null ? null : () => onChanged!(icon),
                icon: Icon(libraryAlbumIcon(icon)),
              ),
            ),
          ),
      ],
    ),
  );
}
