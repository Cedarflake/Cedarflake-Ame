import "package:flutter/material.dart";

import "../../domain/library_models.dart";
import "../library_strings.dart";
import "library_source_navigation_tile.dart";

Future<List<LibraryRoot>?> showLibraryUpdateDialog({
  required BuildContext context,
  required List<LibraryRoot> roots,
  required Set<String> activeRootIds,
  String? initialRootId,
}) {
  return showDialog<List<LibraryRoot>>(
    context: context,
    builder: (context) => _LibraryUpdateDialog(
      roots: roots,
      activeRootIds: activeRootIds,
      initialRootId: initialRootId,
    ),
  );
}

class _LibraryUpdateDialog extends StatefulWidget {
  const _LibraryUpdateDialog({
    required this.roots,
    required this.activeRootIds,
    required this.initialRootId,
  });

  final List<LibraryRoot> roots;
  final Set<String> activeRootIds;
  final String? initialRootId;

  @override
  State<_LibraryUpdateDialog> createState() => _LibraryUpdateDialogState();
}

class _LibraryUpdateDialogState extends State<_LibraryUpdateDialog> {
  late final Set<String> _selectedRootIds;

  @override
  void initState() {
    super.initState();
    _selectedRootIds = {
      if (widget.initialRootId case final rootId?)
        if (_isSelectable(rootId)) rootId,
    };
  }

  bool _isSelectable(String rootId) {
    if (widget.activeRootIds.contains(rootId)) {
      return false;
    }
    return widget.roots.any(
      (root) =>
          root.id == rootId &&
          root.availability == LibraryRootAvailability.available,
    );
  }

  @override
  Widget build(BuildContext context) {
    return AlertDialog(
      key: const Key("library-update-dialog"),
      title: const Text(LibraryStrings.updateLibrary),
      content: SizedBox(
        width: 440,
        child: ConstrainedBox(
          constraints: const BoxConstraints(maxHeight: 360),
          child: ListView.builder(
            shrinkWrap: true,
            itemCount: widget.roots.length,
            itemBuilder: (context, index) {
              final root = widget.roots[index];
              final isActive = widget.activeRootIds.contains(root.id);
              final isAvailable =
                  root.availability == LibraryRootAvailability.available;
              final isEnabled = !isActive && isAvailable;
              final isSelected = _selectedRootIds.contains(root.id);
              return CheckboxListTile(
                key: ValueKey("library-update-root-${root.id}"),
                value: isSelected,
                enabled: isEnabled,
                controlAffinity: ListTileControlAffinity.leading,
                title: Text(librarySourceName(root.displayPath)),
                subtitle: Text(
                  isActive
                      ? LibraryStrings.updatingLibrary
                      : isAvailable
                      ? root.displayPath
                      : LibraryStrings.sourceUnavailable,
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                ),
                onChanged: isEnabled
                    ? (value) {
                        setState(() {
                          if (value ?? false) {
                            _selectedRootIds.add(root.id);
                          } else {
                            _selectedRootIds.remove(root.id);
                          }
                        });
                      }
                    : null,
              );
            },
          ),
        ),
      ),
      actions: [
        TextButton(
          onPressed: () => Navigator.of(context).pop(),
          child: const Text(LibraryStrings.cancel),
        ),
        FilledButton(
          key: const Key("library-update-confirm"),
          onPressed: _selectedRootIds.isEmpty
              ? null
              : () {
                  Navigator.of(context).pop([
                    for (final root in widget.roots)
                      if (_selectedRootIds.contains(root.id)) root,
                  ]);
                },
          child: const Text(LibraryStrings.updateLibrary),
        ),
      ],
    );
  }
}
