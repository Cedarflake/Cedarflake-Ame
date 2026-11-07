import "package:flutter/material.dart";
import "package:material_symbols_icons/symbols.dart";

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

String libraryAlbumIconLabel(LibraryAlbumIcon icon) => switch (icon) {
  LibraryAlbumIcon.album => "相册",
  LibraryAlbumIcon.travel => "旅行",
  LibraryAlbumIcon.nature => "自然",
  LibraryAlbumIcon.pets => "宠物",
  LibraryAlbumIcon.people => "人物",
  LibraryAlbumIcon.food => "美食",
  LibraryAlbumIcon.art => "艺术",
  LibraryAlbumIcon.work => "工作",
  LibraryAlbumIcon.study => "学习",
  LibraryAlbumIcon.games => "游戏",
  LibraryAlbumIcon.music => "音乐",
  LibraryAlbumIcon.memories => "纪念",
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
  Widget build(BuildContext context) => Wrap(
    spacing: 16,
    runSpacing: 8,
    children: [
      for (final icon in LibraryAlbumIcon.values)
        SizedBox.square(
          dimension: 48,
          child: IconButton.filledTonal(
            key: ValueKey("album-icon-${icon.name}"),
            tooltip: libraryAlbumIconLabel(icon),
            isSelected: value == icon,
            onPressed: onChanged == null ? null : () => onChanged!(icon),
            icon: Icon(libraryAlbumIcon(icon)),
          ),
        ),
    ],
  );
}
