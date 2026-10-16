import "package:flutter/material.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:material_symbols_icons/symbols.dart";

import "../../../../app/presentation/ame_overlay_semantics.dart";
import "../../adapters/windows_library_platform_actions.dart";
import "../../domain/library_exact_duplicates.dart";
import "../library_duplicate_strings.dart";
import "library_path_text.dart";

class LibraryDuplicateLocationTile extends ConsumerStatefulWidget {
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
  ConsumerState<LibraryDuplicateLocationTile> createState() =>
      _LibraryDuplicateLocationTileState();
}

class _LibraryDuplicateLocationTileState
    extends ConsumerState<LibraryDuplicateLocationTile> {
  bool _opening = false;

  Future<void> _reveal() async {
    if (_opening) {
      return;
    }
    setState(() => _opening = true);
    try {
      await ref
          .read(libraryPlatformActionsProvider)
          .revealFile(widget.location.displayPath);
    } on Object {
      if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          const SnackBar(content: Text(LibraryDuplicateStrings.revealFailed)),
        );
      }
    } finally {
      if (mounted) {
        setState(() => _opening = false);
      }
    }
  }

  @override
  Widget build(BuildContext context) {
    final location = widget.location;
    final path = location.displayPath.replaceAll("/", "\\");
    final separator = path.lastIndexOf("\\");
    final folder = separator < 0 ? "" : path.substring(0, separator);
    final availability = LibraryDuplicateStrings.availability(
      location.availability,
    );
    final details = [
      if (widget.isCurrent) LibraryDuplicateStrings.currentImage,
      if (availability != null) availability,
    ];
    final available = switch (location.availability) {
      LibraryDuplicateAvailability.local ||
      LibraryDuplicateAvailability.cloudOnly => true,
      LibraryDuplicateAvailability.offline ||
      LibraryDuplicateAvailability.unavailable => false,
    };
    final tile = ListTile(
      key: ValueKey("duplicate-location-${location.locationId}"),
      leading: const Icon(Symbols.folder_rounded),
      title: Text(
        displayLibraryFileName(path),
        maxLines: widget.wrapPath ? null : 1,
        overflow: widget.wrapPath ? TextOverflow.clip : TextOverflow.ellipsis,
      ),
      subtitle: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          if (folder.isNotEmpty)
            Text(
              folder,
              maxLines: widget.wrapPath ? null : 2,
              overflow: widget.wrapPath
                  ? TextOverflow.clip
                  : TextOverflow.ellipsis,
            ),
          if (details.isNotEmpty) Text(details.join(" · ")),
        ],
      ),
      onTap: available && !_opening ? _reveal : null,
      trailing: AmeTooltip(
        message: LibraryDuplicateStrings.revealLocation,
        child: IconButton(
          onPressed: available && !_opening ? _reveal : null,
          icon: const Icon(Symbols.folder_open_rounded),
        ),
      ),
    );
    if (widget.wrapPath) {
      return tile;
    }
    return AmeTooltip(message: location.displayPath, child: tile);
  }
}
