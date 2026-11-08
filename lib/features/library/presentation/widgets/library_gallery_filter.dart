import "dart:async";

import "package:material_ui/material_ui.dart";
import "package:material_symbols_icons/symbols.dart";

import "../../../../app/presentation/ame_menu.dart";
import "../../../../app/presentation/ame_menu_choice.dart";
import "../../../../app/presentation/ame_overlay_semantics.dart";
import "../../domain/library_models.dart";
import "../coordination/library_metadata_filter_summary.dart";
import "../library_duplicate_strings.dart";
import "../library_filter_strings.dart";
import "library_metadata_filter_sheet.dart";

sealed class _FilterAction {
  const _FilterAction();
}

enum _FilterChoice implements _FilterAction {
  descendants,
  folderOnly,
  all,
  merged,
  metadata,
}

class _UpdateMetadata extends _FilterAction {
  const _UpdateMetadata(this.openingQuery, this.filter);

  final LibraryGalleryQuery openingQuery;
  final LibraryMetadataFilter filter;
}

class LibraryGalleryFilter extends StatefulWidget {
  const LibraryGalleryFilter({
    required this.query,
    required this.onChanged,
    super.key,
  });

  final LibraryGalleryQuery query;
  final ValueChanged<LibraryGalleryQuery> onChanged;

  @override
  State<LibraryGalleryFilter> createState() => _LibraryGalleryFilterState();
}

class _LibraryGalleryFilterState extends State<LibraryGalleryFilter> {
  LibraryGalleryQuery get query => widget.query;

  Future<void> _editMetadata() async {
    final openingQuery = query;
    final filter = await showLibraryMetadataFilterSheet(
      context,
      query.metadataFilter,
    );
    if (!mounted || filter == null || query != openingQuery) return;
    if (filter != query.metadataFilter) {
      widget.onChanged(query.copyWith(metadataFilter: filter));
    }
  }

  @override
  Widget build(BuildContext context) {
    final summary = LibraryMetadataFilterSummary(query.metadataFilter);
    final conditions = summary.conditions;
    final active = query.metadataFilter.isActive;
    return AmePopupMenuButton<_FilterAction>(
      labels: [
        LibraryDuplicateStrings.showDescendants,
        LibraryDuplicateStrings.hideDescendants,
        LibraryDuplicateStrings.allImages,
        LibraryDuplicateStrings.mergeDuplicates,
        LibraryFilterStrings.more,
        if (active) LibraryFilterStrings.clear,
        for (final condition in conditions) condition.label,
      ],
      hasContentIcon: active,
      leadingIconWidth: AmeMenuMetrics.selectionIndicatorSlotWidth,
      items: [
        _choice(
          _FilterChoice.descendants,
          LibraryDuplicateStrings.showDescendants,
          query.includeDescendants,
          enabled: query.folderRelativePath != null,
        ),
        _choice(
          _FilterChoice.folderOnly,
          LibraryDuplicateStrings.hideDescendants,
          !query.includeDescendants,
          enabled: query.folderRelativePath != null,
        ),
        const PopupMenuDivider(height: AmeMenuMetrics.dividerHeight),
        _choice(
          _FilterChoice.all,
          LibraryDuplicateStrings.allImages,
          !query.foldsDuplicates,
          enabled: !query.requiresMergedDuplicates,
        ),
        _choice(
          _FilterChoice.merged,
          LibraryDuplicateStrings.mergeDuplicates,
          query.foldsDuplicates,
          enabled: !query.requiresMergedDuplicates,
        ),
        const PopupMenuDivider(height: AmeMenuMetrics.dividerHeight),
        const PopupMenuItem(
          value: _FilterChoice.metadata,
          child: Padding(
            padding: EdgeInsets.only(
              left:
                  AmeMenuMetrics.selectionIndicatorSlotWidth +
                  AmeMenuMetrics.iconLabelGap,
            ),
            child: Text(LibraryFilterStrings.more),
          ),
        ),
        if (active) ...[
          PopupMenuItem(
            value: _UpdateMetadata(query, const LibraryMetadataFilter()),
            child: const Padding(
              padding: EdgeInsets.only(
                left:
                    AmeMenuMetrics.selectionIndicatorSlotWidth +
                    AmeMenuMetrics.iconLabelGap,
              ),
              child: Text(LibraryFilterStrings.clear),
            ),
          ),
          const PopupMenuDivider(height: AmeMenuMetrics.dividerHeight),
          for (final condition in conditions)
            PopupMenuItem(
              value: _UpdateMetadata(query, condition.remaining.normalized),
              child: AmeTooltip(
                message: "移除${condition.label}",
                child: Semantics(
                  label: "移除${condition.label}",
                  excludeSemantics: true,
                  child: Row(
                    children: [
                      Expanded(
                        child: Text(
                          condition.label,
                          maxLines: 1,
                          overflow: TextOverflow.ellipsis,
                        ),
                      ),
                      const SizedBox(width: AmeMenuMetrics.iconLabelGap),
                      const Icon(Symbols.close_rounded, size: 18),
                    ],
                  ),
                ),
              ),
            ),
        ],
      ],
      onSelected: (action) => _select(context, action),
      builder: (context, openMenu) => AmeTooltip(
        message: active
            ? "${LibraryDuplicateStrings.filter}\n${summary.description}"
            : LibraryDuplicateStrings.filter,
        child: IconButton(
          key: const Key("library-filter-menu"),
          onPressed: openMenu,
          isSelected: active,
          icon: const Icon(Symbols.filter_list_rounded),
        ),
      ),
    );
  }

  void _select(BuildContext context, _FilterAction action) {
    if (!context.mounted) return;
    switch (action) {
      case _FilterChoice.metadata:
        unawaited(_editMetadata());
      case _UpdateMetadata(:final openingQuery, :final filter):
        if (query == openingQuery) {
          widget.onChanged(query.copyWith(metadataFilter: filter));
        }
      case final _FilterChoice choice:
        widget.onChanged(switch (choice) {
          _FilterChoice.descendants => query.copyWith(includeDescendants: true),
          _FilterChoice.folderOnly => query.copyWith(includeDescendants: false),
          _FilterChoice.all => query.copyWith(
            duplicateMode: LibraryGalleryDuplicateMode.all,
          ),
          _FilterChoice.merged => query.copyWith(
            duplicateMode: LibraryGalleryDuplicateMode.merged,
          ),
          _FilterChoice.metadata => query,
        });
    }
  }

  PopupMenuEntry<_FilterAction> _choice(
    _FilterChoice choice,
    String label,
    bool selected, {
    bool enabled = true,
  }) => ameMenuChoice(
    value: choice,
    isSelected: selected,
    enabled: enabled,
    label: label,
  );
}
