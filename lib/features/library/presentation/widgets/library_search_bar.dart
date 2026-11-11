import "package:material_ui/material_ui.dart";
import "package:material_symbols_icons/symbols.dart";

import "../../../../app/presentation/ame_progress.dart";
import "../../../../app/presentation/ame_menu.dart";
import "../../../../app/presentation/ame_menu_choice.dart";
import "../../../../app/presentation/ame_overlay_semantics.dart";
import "../../domain/library_view_preferences.dart";
import "../coordination/library_search.dart";
import "../library_search_strings.dart";
import "../library_strings.dart";

class LibrarySearchBar extends StatelessWidget {
  const LibrarySearchBar({
    required this.controller,
    required this.enabled,
    this.onChanged,
    this.discovery,
    super.key,
  });

  final TextEditingController controller;
  final bool enabled;
  final ValueChanged<String>? onChanged;
  final LibrarySearch? discovery;

  @override
  Widget build(BuildContext context) {
    final search = discovery;
    final semantic = search?.mode == LibrarySearchMode.content;
    final hint = switch (search?.referenceName) {
      final String name => "与 $name 相关的图片",
      null =>
        semantic
            ? LibrarySearchStrings.contentHint
            : LibrarySearchStrings.fileHint,
    };
    return SearchBar(
      key: const Key("library-search"),
      constraints: const BoxConstraints(minHeight: 44, maxHeight: 44),
      controller: controller,
      enabled: enabled,
      hintText: hint,
      leading: search == null
          ? const Icon(Symbols.search_rounded)
          : _SearchMenu(search: search, enabled: enabled),
      trailing: [
        SizedBox.square(
          dimension: 40,
          child:
              search != null &&
                  !search.loading &&
                  (search.hasUpdates || search.error != null)
              ? AmeTooltip(
                  message: search.error != null
                      ? LibrarySearchStrings.retry
                      : LibrarySearchStrings.update,
                  child: IconButton(
                    onPressed: enabled ? search.retry : null,
                    icon: const Icon(Symbols.refresh_rounded),
                  ),
                )
              : Center(
                  child: SizedBox.square(
                    dimension: 18,
                    child: AmeCircularProgress(
                      active: search?.loading ?? false,
                      strokeWidth: 2,
                    ),
                  ),
                ),
        ),
        if (controller.text.isNotEmpty || (search?.related ?? false))
          AmeTooltip(
            message: LibraryStrings.clearSearch,
            child: IconButton(
              onPressed: enabled
                  ? () {
                      if (search != null) {
                        search.clear();
                      } else {
                        controller.clear();
                        onChanged?.call("");
                      }
                    }
                  : null,
              icon: const Icon(Symbols.close_rounded),
            ),
          ),
      ],
      onChanged: onChanged,
      onSubmitted: search == null ? onChanged : (_) => search.submit(),
    );
  }
}

enum _SearchChoice { content, fileName, currentScope, entireLibrary }

class _SearchMenu extends StatelessWidget {
  const _SearchMenu({required this.search, required this.enabled});
  final LibrarySearch search;
  final bool enabled;

  @override
  Widget build(BuildContext context) {
    final label = search.mode == LibrarySearchMode.content
        ? LibrarySearchStrings.content
        : LibrarySearchStrings.fileName;
    return AmePopupMenuButton<_SearchChoice>(
      labels: const [
        LibrarySearchStrings.content,
        LibrarySearchStrings.fileName,
        LibrarySearchStrings.currentScope,
        LibrarySearchStrings.entireLibrary,
      ],
      leadingIconWidth: AmeMenuMetrics.selectionIndicatorSlotWidth,
      items: [
        if (search.available)
          ameMenuChoice(
            value: _SearchChoice.content,
            label: LibrarySearchStrings.content,
            icon: Symbols.image_rounded,
            isSelected: search.mode == LibrarySearchMode.content,
          ),
        ameMenuChoice(
          value: _SearchChoice.fileName,
          label: LibrarySearchStrings.fileName,
          icon: Symbols.text_fields_rounded,
          isSelected: search.mode == LibrarySearchMode.fileName,
        ),
        const PopupMenuDivider(height: AmeMenuMetrics.dividerHeight),
        ameMenuChoice(
          value: _SearchChoice.currentScope,
          label: LibrarySearchStrings.currentScope,
          icon: Symbols.folder_open_rounded,
          isSelected: !search.wholeLibrary,
        ),
        ameMenuChoice(
          value: _SearchChoice.entireLibrary,
          label: LibrarySearchStrings.entireLibrary,
          icon: Symbols.photo_library_rounded,
          isSelected: search.wholeLibrary,
        ),
      ],
      onSelected: (choice) {
        switch (choice) {
          case _SearchChoice.content:
            search.selectMode(LibrarySearchMode.content);
          case _SearchChoice.fileName:
            search.selectMode(LibrarySearchMode.fileName);
          case _SearchChoice.currentScope:
            search.selectScope(false);
          case _SearchChoice.entireLibrary:
            search.selectScope(true);
        }
      },
      builder: (context, openMenu) => AmeTooltip(
        message:
            "$label · ${search.wholeLibrary ? LibrarySearchStrings.entireLibrary : LibrarySearchStrings.currentScope}",
        child: IconButton(
          key: const Key("library-search-menu"),
          onPressed: enabled ? openMenu : null,
          icon: Row(
            mainAxisSize: MainAxisSize.min,
            children: [
              Icon(
                search.mode == LibrarySearchMode.content
                    ? Symbols.image_rounded
                    : Symbols.search_rounded,
              ),
              const Icon(Symbols.arrow_drop_down_rounded, size: 18),
            ],
          ),
        ),
      ),
    );
  }
}
