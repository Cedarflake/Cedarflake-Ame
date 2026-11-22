import "dart:async";

import "package:material_ui/material_ui.dart";
import "package:material_symbols_icons/symbols.dart";

import "../../../../app/presentation/ame_localizations.dart";
import "../../../../app/presentation/ame_menu.dart";
import "../../../../app/presentation/ame_menu_choice.dart";
import "../../../../app/presentation/ame_overlay_semantics.dart";
import "../../domain/library_gallery_group.dart";
import "../../domain/library_models.dart";
import "../coordination/library_metadata_filter_summary.dart";
import "library_metadata_filter_sheet.dart";

sealed class _FilterAction {
  const _FilterAction();
}

enum _FilterChoice implements _FilterAction {
  descendants,
  folderOnly,
  merged,
  unrestrictedGroups,
  exactGroups,
  similarGroups,
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
    final conditions = summary.conditions(context.l10n);
    final active = query.metadataFilter.isActive;
    return AmePopupMenuButton<_FilterAction>(
      labels: [
        context.l10n.showSubfolders,
        context.l10n.hideSubfolders,
        context.l10n.allPhotosFilter,
        context.l10n.duplicatesOnly,
        context.l10n.similarOnly,
        context.l10n.mergeDuplicates,
        context.l10n.moreFilters,
        if (active) context.l10n.clearFilters,
        for (final condition in conditions) condition.label,
      ],
      hasContentIcon: active,
      leadingIconWidth: AmeMenuMetrics.iconSize,
      items: [
        _choice(
          _FilterChoice.descendants,
          context.l10n.showSubfolders,
          query.includeDescendants,
          enabled: query.folderRelativePath != null,
        ),
        _choice(
          _FilterChoice.folderOnly,
          context.l10n.hideSubfolders,
          !query.includeDescendants,
          enabled: query.folderRelativePath != null,
        ),
        const PopupMenuDivider(height: AmeMenuMetrics.dividerHeight),
        _choice(
          _FilterChoice.unrestrictedGroups,
          context.l10n.allPhotosFilter,
          !query.hasGroupFilter,
        ),
        _choice(
          _FilterChoice.exactGroups,
          context.l10n.duplicatesOnly,
          query.groupFilter == LibraryGalleryGroupFilter.exact,
        ),
        _choice(
          _FilterChoice.similarGroups,
          context.l10n.similarOnly,
          query.groupFilter == LibraryGalleryGroupFilter.similar,
        ),
        const PopupMenuDivider(height: AmeMenuMetrics.dividerHeight),
        ameMenuChoice(
          value: _FilterChoice.merged,
          label: context.l10n.mergeDuplicates,
          isSelected: !query.hasGroupFilter && query.foldsDuplicates,
          enabled: !query.requiresMergedDuplicates,
          isToggle: true,
          indicatorWidth: AmeMenuMetrics.iconSize,
        ),
        const PopupMenuDivider(height: AmeMenuMetrics.dividerHeight),
        PopupMenuItem(
          value: _FilterChoice.metadata,
          child: Padding(
            padding: EdgeInsets.only(
              left: AmeMenuMetrics.iconSize + AmeMenuMetrics.iconLabelGap,
            ),
            child: Text(context.l10n.moreFilters),
          ),
        ),
        if (active) ...[
          PopupMenuItem(
            value: _UpdateMetadata(query, const LibraryMetadataFilter()),
            child: Padding(
              padding: EdgeInsets.only(
                left: AmeMenuMetrics.iconSize + AmeMenuMetrics.iconLabelGap,
              ),
              child: Text(context.l10n.clearFilters),
            ),
          ),
          const PopupMenuDivider(height: AmeMenuMetrics.dividerHeight),
          for (final condition in conditions)
            PopupMenuItem(
              value: _UpdateMetadata(query, condition.remaining.normalized),
              child: AmeTooltip(
                message: context.l10n.removeFilter(condition.label),
                child: Semantics(
                  label: context.l10n.removeFilter(condition.label),
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
            ? "${context.l10n.filter}\n${summary.description(context.l10n)}"
            : context.l10n.filter,
        child: IconButton(
          key: const Key("library-filter-menu"),
          onPressed: openMenu,
          icon: const Row(
            mainAxisSize: MainAxisSize.min,
            children: [
              Icon(Symbols.filter_list_rounded),
              Icon(Symbols.arrow_drop_down_rounded, size: 18),
            ],
          ),
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
          _FilterChoice.merged => query.copyWith(
            duplicateMode: query.foldsDuplicates
                ? LibraryGalleryDuplicateMode.all
                : LibraryGalleryDuplicateMode.merged,
          ),
          _FilterChoice.unrestrictedGroups => query.copyWith(
            groupFilter: LibraryGalleryGroupFilter.unrestricted,
          ),
          _FilterChoice.exactGroups => query.copyWith(
            groupFilter: LibraryGalleryGroupFilter.exact,
          ),
          _FilterChoice.similarGroups => query.copyWith(
            groupFilter: LibraryGalleryGroupFilter.similar,
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
    indicatorWidth: AmeMenuMetrics.iconSize,
  );
}
