import "package:flutter/material.dart";
import "package:material_symbols_icons/symbols.dart";

import "../../../../app/presentation/ame_overlay_semantics.dart";
import "../../domain/library_exact_duplicates.dart";
import "../library_duplicate_strings.dart";
import "library_duplicate_reveal_action.dart";
import "library_path_text.dart";

class LibraryDuplicateLocationTile extends StatelessWidget {
  const LibraryDuplicateLocationTile({
    required this.location,
    this.isCurrent = false,
    this.wrapPath = false,
    super.key,
  });
  final LibraryDuplicateLocation location;
  final bool isCurrent;
  final bool wrapPath;

  @override
  Widget build(BuildContext context) {
    final path = location.displayPath.replaceAll("/", "\\");
    final separator = path.lastIndexOf("\\");
    final folder = separator < 0 ? "" : path.substring(0, separator);
    final details = [
      if (isCurrent) LibraryDuplicateStrings.currentImage,
      ?LibraryDuplicateStrings.availability(location.availability),
    ];
    return LibraryDuplicateRevealAction(
      location: location,
      builder: (context, reveal) {
        final tile = ListTile(
          key: ValueKey("duplicate-location-${location.locationId}"),
          leading: const Icon(Symbols.folder_rounded),
          title: Text(
            displayLibraryFileName(path),
            maxLines: wrapPath ? null : 1,
            overflow: wrapPath ? TextOverflow.clip : TextOverflow.ellipsis,
          ),
          subtitle: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              if (folder.isNotEmpty)
                Text(
                  folder,
                  maxLines: wrapPath ? null : 2,
                  overflow: wrapPath
                      ? TextOverflow.clip
                      : TextOverflow.ellipsis,
                ),
              if (details.isNotEmpty) Text(details.join(" · ")),
            ],
          ),
          onTap: reveal,
          trailing: AmeTooltip(
            message: LibraryDuplicateStrings.revealLocation,
            child: IconButton(
              onPressed: reveal,
              icon: const Icon(Symbols.folder_open_rounded),
            ),
          ),
        );
        if (wrapPath) return tile;
        return AmeTooltip(message: location.displayPath, child: tile);
      },
    );
  }
}
