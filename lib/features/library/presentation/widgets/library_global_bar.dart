import "dart:math" as math;

import "package:flutter/material.dart";
import "package:material_symbols_icons/symbols.dart";

import "../../../../app/presentation/ame_overlay_semantics.dart";
import "../../../../app/window/ame_window_chrome.dart";
import "../library_strings.dart";

class LibraryGlobalBar extends StatelessWidget {
  const LibraryGlobalBar({
    required this.isBusy,
    required this.searchController,
    required this.onSearchChanged,
    super.key,
  });

  final bool isBusy;
  final TextEditingController searchController;
  final ValueChanged<String> onSearchChanged;

  @override
  Widget build(BuildContext context) {
    return Material(
      key: const Key("library-global-surface"),
      color: Theme.of(context).colorScheme.surfaceContainerLow,
      child: SizedBox(
        key: const Key("library-global-bar"),
        height: 64,
        child: LayoutBuilder(
          builder: (context, constraints) {
            final isCompact = constraints.maxWidth < 980;
            final leadingWidth = isCompact ? 72.0 : 244.0;
            const captionWidth = 48.0 * 3 + 8;
            final edgeReserve = math.max(leadingWidth, captionWidth);
            final searchWidth = math.min(
              720.0,
              math.max(360.0, constraints.maxWidth - edgeReserve * 2 - 32),
            );
            return Stack(
              fit: StackFit.expand,
              children: [
                const Positioned.fill(
                  child: AmeWindowDragRegion(child: SizedBox.expand()),
                ),
                Align(
                  alignment: Alignment.centerLeft,
                  child: AmeWindowDragRegion(
                    child: SizedBox(
                      width: leadingWidth,
                      child: Padding(
                        padding: const EdgeInsets.only(left: 18),
                        child: Row(
                          children: [
                            const Icon(Symbols.photo_library_rounded),
                            if (!isCompact) ...[
                              const SizedBox(width: 12),
                              const Text(LibraryStrings.appName),
                            ],
                          ],
                        ),
                      ),
                    ),
                  ),
                ),
                Align(
                  child: SizedBox(
                    width: searchWidth,
                    child: Align(
                      key: const Key("library-search-alignment"),
                      child: SearchBar(
                        key: const Key("library-search"),
                        constraints: const BoxConstraints(
                          minHeight: 44,
                          maxHeight: 44,
                        ),
                        controller: searchController,
                        enabled: !isBusy,
                        hintText: LibraryStrings.searchHint,
                        leading: const Icon(Symbols.search_rounded),
                        trailing: [
                          if (searchController.text.isNotEmpty)
                            AmeTooltip(
                              message: LibraryStrings.clearSearch,
                              child: IconButton(
                                onPressed: () {
                                  searchController.clear();
                                  onSearchChanged("");
                                },
                                icon: const Icon(Symbols.close_rounded),
                              ),
                            ),
                        ],
                        onChanged: onSearchChanged,
                      ),
                    ),
                  ),
                ),
                const Align(
                  alignment: Alignment.centerRight,
                  child: AmeWindowCaptionControls(height: 64),
                ),
              ],
            );
          },
        ),
      ),
    );
  }
}
