import "package:flutter/material.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:material_symbols_icons/symbols.dart";

import "../../adapters/windows_library_platform_actions.dart";
import "../../domain/library_exact_duplicates.dart";
import "../library_duplicate_strings.dart";

class LibraryDuplicateLocationTile extends ConsumerStatefulWidget {
  const LibraryDuplicateLocationTile({required this.location, super.key});
  final LibraryDuplicateLocation location;

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
    final available = switch (location.availability) {
      LibraryDuplicateAvailability.local ||
      LibraryDuplicateAvailability.cloudOnly => true,
      LibraryDuplicateAvailability.offline ||
      LibraryDuplicateAvailability.unavailable => false,
    };
    return Tooltip(
      message: location.displayPath,
      child: ListTile(
        key: ValueKey("duplicate-location-${location.locationId}"),
        leading: const Icon(Symbols.folder_rounded),
        title: Text(location.displayPath, maxLines: 2),
        subtitle: Text(
          LibraryDuplicateStrings.availability(location.availability),
        ),
        onTap: available && !_opening ? _reveal : null,
        trailing: IconButton(
          tooltip: LibraryDuplicateStrings.revealLocation,
          onPressed: available && !_opening ? _reveal : null,
          icon: const Icon(Symbols.folder_open_rounded),
        ),
      ),
    );
  }
}
