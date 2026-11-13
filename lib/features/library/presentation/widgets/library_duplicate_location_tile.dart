import "package:material_ui/material_ui.dart";
import "package:material_symbols_icons/symbols.dart";

import "../../../../app/presentation/ame_overlay_semantics.dart";
import "../../domain/library_exact_duplicates.dart";
import "../library_duplicate_strings.dart";
import "library_duplicate_reveal_action.dart";
import "library_path_text.dart";
import "library_sidebar_selection_icon.dart";

class LibraryDuplicateLocationTile extends StatelessWidget {
  const LibraryDuplicateLocationTile({
    required this.location,
    required this.onOpen,
    this.selected = false,
    this.onSelect,
    this.onPreference,
    this.preferred = false,
    this.preferenceLabel = "设为首选",
    this.wrapPath = false,
    super.key,
  });
  final LibraryDuplicateLocation location;
  final VoidCallback onOpen;
  final bool selected;
  final VoidCallback? onSelect;
  final VoidCallback? onPreference;
  final bool preferred;
  final String preferenceLabel;
  final bool wrapPath;

  @override
  Widget build(BuildContext context) {
    final path = location.displayPath.replaceAll("/", "\\");
    final separator = path.lastIndexOf("\\");
    final folder = separator < 0 ? "" : path.substring(0, separator);
    final availability = LibraryDuplicateStrings.availability(
      location.availability,
    );
    return LibraryDuplicateRevealAction(
      location: location,
      builder: (context, reveal) {
        final tile = ListTile(
          shape: RoundedRectangleBorder(
            borderRadius: BorderRadius.circular(12),
          ),
          contentPadding: const EdgeInsets.symmetric(
            horizontal: 12,
            vertical: 4,
          ),
          key: ValueKey("duplicate-location-${location.locationId}"),
          selected: selected,
          leading: onSelect == null
              ? const Icon(Symbols.folder_rounded)
              : LibrarySidebarSelectionIcon(
                  icon: const Icon(Symbols.folder_rounded),
                  name: displayLibraryFileName(path),
                  selected: selected,
                  onPressed: onSelect,
                ),
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
              if (availability != null) Text(availability),
            ],
          ),
          onTap: onOpen,
          trailing: Row(
            mainAxisSize: MainAxisSize.min,
            children: [
              if (onSelect != null)
                IconButton(
                  tooltip: preferenceLabel,
                  onPressed: onPreference,
                  isSelected: preferred,
                  icon: const Icon(Symbols.bookmark_rounded),
                  selectedIcon: const Icon(Symbols.bookmark_rounded, fill: 1),
                ),
              IconButton(
                tooltip: LibraryDuplicateStrings.revealLocation,
                onPressed: reveal,
                icon: const Icon(Symbols.folder_open_rounded),
              ),
            ],
          ),
        );
        return Padding(
          padding: const EdgeInsets.only(bottom: 4),
          child: ClipRRect(
            borderRadius: BorderRadius.circular(12),
            child: Material(
              color: Colors.transparent,
              child: wrapPath
                  ? tile
                  : AmeTooltip(message: location.displayPath, child: tile),
            ),
          ),
        );
      },
    );
  }
}
