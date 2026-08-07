import "package:flutter/material.dart";

import "../library_strings.dart";

class LibraryGlobalBar extends StatelessWidget {
  const LibraryGlobalBar({
    required this.isBusy,
    required this.searchController,
    required this.onSearchChanged,
    required this.onImport,
    super.key,
  });

  final bool isBusy;
  final TextEditingController searchController;
  final ValueChanged<String> onSearchChanged;
  final VoidCallback onImport;

  @override
  Widget build(BuildContext context) {
    return SizedBox(
      height: 64,
      child: Padding(
        padding: const EdgeInsets.symmetric(horizontal: 16),
        child: Row(
          children: [
            const SizedBox(
              width: 244,
              child: Row(
                children: [
                  Icon(Icons.photo_library_outlined),
                  SizedBox(width: 12),
                  Text(LibraryStrings.appName),
                ],
              ),
            ),
            const Spacer(),
            Flexible(
              child: ConstrainedBox(
                constraints: const BoxConstraints(maxWidth: 720),
                child: SearchBar(
                  key: const Key("library-search"),
                  controller: searchController,
                  enabled: !isBusy,
                  hintText: LibraryStrings.searchHint,
                  leading: const Icon(Icons.search),
                  trailing: [
                    if (searchController.text.isNotEmpty)
                      IconButton(
                        tooltip: LibraryStrings.clearSearch,
                        onPressed: () {
                          searchController.clear();
                          onSearchChanged("");
                        },
                        icon: const Icon(Icons.close),
                      ),
                  ],
                  onChanged: onSearchChanged,
                ),
              ),
            ),
            const Spacer(),
            FilledButton.tonalIcon(
              key: const Key("library-import-button"),
              onPressed: isBusy ? null : onImport,
              icon: const Icon(Icons.add_photo_alternate_outlined),
              label: const Text(LibraryStrings.import),
            ),
          ],
        ),
      ),
    );
  }
}
