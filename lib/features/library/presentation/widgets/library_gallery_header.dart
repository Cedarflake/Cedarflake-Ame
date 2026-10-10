import "package:flutter/material.dart";
import "package:material_symbols_icons/symbols.dart";

import "../../../../app/presentation/ame_menu.dart";
import "../../../../app/presentation/ame_overlay_semantics.dart";
import "../../domain/library_models.dart";
import "../gallery_view_options.dart";
import "../library_strings.dart";

class LibraryGalleryHeader extends StatelessWidget {
  const LibraryGalleryHeader({
    required this.galleryTitle,
    required this.totalItems,
    required this.selectedCount,
    required this.isSelecting,
    required this.layoutShape,
    required this.thumbnailSize,
    required this.sortKey,
    required this.sortDirection,
    required this.onBeginSelection,
    required this.onCancelSelection,
    required this.onSelectAll,
    required this.onLayoutShapeChanged,
    required this.onThumbnailSizeChanged,
    required this.onSortKeyChanged,
    required this.onSortDirectionChanged,
    super.key,
  });

  final String galleryTitle;
  final int totalItems;
  final int selectedCount;
  final bool isSelecting;
  final GalleryLayoutShape layoutShape;
  final GalleryThumbnailSize thumbnailSize;
  final LibraryGallerySortKey sortKey;
  final LibraryGallerySortDirection sortDirection;
  final VoidCallback onBeginSelection;
  final VoidCallback onCancelSelection;
  final VoidCallback onSelectAll;
  final ValueChanged<GalleryLayoutShape> onLayoutShapeChanged;
  final ValueChanged<GalleryThumbnailSize> onThumbnailSizeChanged;
  final ValueChanged<LibraryGallerySortKey> onSortKeyChanged;
  final ValueChanged<LibraryGallerySortDirection> onSortDirectionChanged;

  @override
  Widget build(BuildContext context) {
    final title = isSelecting ? "已选择 $selectedCount 个项目" : galleryTitle;
    final subtitle = isSelecting ? galleryTitle : "$totalItems 张图片";
    return SizedBox(
      key: const Key("library-gallery-header"),
      width: double.infinity,
      child: ConstrainedBox(
        constraints: const BoxConstraints(minHeight: 104),
        child: Padding(
          padding: const EdgeInsets.fromLTRB(28, 18, 20, 16),
          child: LayoutBuilder(
            builder: (context, constraints) {
              final titleBlock = Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                mainAxisSize: MainAxisSize.min,
                children: [
                  Text(
                    title,
                    key: const Key("library-gallery-title"),
                    style: Theme.of(context).textTheme.headlineSmall,
                  ),
                  const SizedBox(height: 4),
                  Text(
                    subtitle,
                    key: const Key("library-summary"),
                    style: Theme.of(context).textTheme.bodyMedium?.copyWith(
                      color: Theme.of(context).colorScheme.onSurfaceVariant,
                    ),
                  ),
                ],
              );
              final toolbar = isSelecting
                  ? _SelectionToolbar(onCancel: onCancelSelection)
                  : _BrowsingToolbar(
                      layoutShape: layoutShape,
                      thumbnailSize: thumbnailSize,
                      sortKey: sortKey,
                      sortDirection: sortDirection,
                      onBeginSelection: onBeginSelection,
                      onSelectAll: onSelectAll,
                      onLayoutShapeChanged: onLayoutShapeChanged,
                      onThumbnailSizeChanged: onThumbnailSizeChanged,
                      onSortKeyChanged: onSortKeyChanged,
                      onSortDirectionChanged: onSortDirectionChanged,
                    );
              if (constraints.maxWidth < 760) {
                return Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    titleBlock,
                    const SizedBox(height: 12),
                    SingleChildScrollView(
                      scrollDirection: Axis.horizontal,
                      child: toolbar,
                    ),
                  ],
                );
              }
              return Row(
                children: [
                  titleBlock,
                  const SizedBox(width: 24),
                  Expanded(
                    child: SingleChildScrollView(
                      scrollDirection: Axis.horizontal,
                      reverse: true,
                      child: ConstrainedBox(
                        constraints: BoxConstraints(
                          minWidth: constraints.maxWidth - 210,
                        ),
                        child: Align(
                          alignment: Alignment.centerRight,
                          child: toolbar,
                        ),
                      ),
                    ),
                  ),
                ],
              );
            },
          ),
        ),
      ),
    );
  }
}

class _BrowsingToolbar extends StatelessWidget {
  const _BrowsingToolbar({
    required this.layoutShape,
    required this.thumbnailSize,
    required this.sortKey,
    required this.sortDirection,
    required this.onBeginSelection,
    required this.onSelectAll,
    required this.onLayoutShapeChanged,
    required this.onThumbnailSizeChanged,
    required this.onSortKeyChanged,
    required this.onSortDirectionChanged,
  });

  final GalleryLayoutShape layoutShape;
  final GalleryThumbnailSize thumbnailSize;
  final LibraryGallerySortKey sortKey;
  final LibraryGallerySortDirection sortDirection;
  final VoidCallback onBeginSelection;
  final VoidCallback onSelectAll;
  final ValueChanged<GalleryLayoutShape> onLayoutShapeChanged;
  final ValueChanged<GalleryThumbnailSize> onThumbnailSizeChanged;
  final ValueChanged<LibraryGallerySortKey> onSortKeyChanged;
  final ValueChanged<LibraryGallerySortDirection> onSortDirectionChanged;

  @override
  Widget build(BuildContext context) {
    return Row(
      key: const Key("library-browsing-toolbar"),
      mainAxisSize: MainAxisSize.min,
      children: [
        TextButton.icon(
          key: const Key("library-select-button"),
          onPressed: onBeginSelection,
          icon: const Icon(Symbols.check_box_rounded),
          label: const Text(LibraryStrings.select),
        ),
        _SortMenu(
          sortKey: sortKey,
          direction: sortDirection,
          onSortKeyChanged: onSortKeyChanged,
          onDirectionChanged: onSortDirectionChanged,
        ),
        _LayoutMenu(
          shape: layoutShape,
          size: thumbnailSize,
          onShapeChanged: onLayoutShapeChanged,
          onSizeChanged: onThumbnailSizeChanged,
        ),
        _MoreMenu(onSelectAll: onSelectAll),
      ],
    );
  }
}

class _SelectionToolbar extends StatelessWidget {
  const _SelectionToolbar({required this.onCancel});

  final VoidCallback onCancel;

  @override
  Widget build(BuildContext context) {
    return Row(
      key: const Key("library-selection-toolbar"),
      mainAxisSize: MainAxisSize.min,
      children: [
        TextButton.icon(
          key: const Key("library-cancel-selection"),
          onPressed: onCancel,
          icon: const Icon(Symbols.close_rounded),
          label: const Text(LibraryStrings.cancel),
        ),
      ],
    );
  }
}

class _SortMenu extends StatelessWidget {
  const _SortMenu({
    required this.sortKey,
    required this.direction,
    required this.onSortKeyChanged,
    required this.onDirectionChanged,
  });

  final LibraryGallerySortKey sortKey;
  final LibraryGallerySortDirection direction;
  final ValueChanged<LibraryGallerySortKey> onSortKeyChanged;
  final ValueChanged<LibraryGallerySortDirection> onDirectionChanged;

  @override
  Widget build(BuildContext context) {
    return AmePopupMenuButton<_SortMenuAction>(
      labels: const [
        LibraryStrings.captureDate,
        LibraryStrings.createdDate,
        LibraryStrings.modifiedDate,
        LibraryStrings.fileName,
        LibraryStrings.ascending,
        LibraryStrings.descending,
      ],
      leadingIconWidth: AmeMenuMetrics.selectionIndicatorSlotWidth,
      items: [
        _menuChoice(
          value: _SortMenuAction.captureTime,
          label: LibraryStrings.captureDate,
          icon: Symbols.calendar_month_rounded,
          isSelected: sortKey == LibraryGallerySortKey.captureTime,
        ),
        _menuChoice(
          value: _SortMenuAction.createdTime,
          label: LibraryStrings.createdDate,
          icon: Symbols.create_new_folder_rounded,
          isSelected: sortKey == LibraryGallerySortKey.createdTime,
        ),
        _menuChoice(
          value: _SortMenuAction.modifiedTime,
          label: LibraryStrings.modifiedDate,
          icon: Symbols.edit_calendar_rounded,
          isSelected: sortKey == LibraryGallerySortKey.modifiedTime,
        ),
        _menuChoice(
          value: _SortMenuAction.fileName,
          label: LibraryStrings.fileName,
          icon: Symbols.text_fields_rounded,
          isSelected: sortKey == LibraryGallerySortKey.fileName,
        ),
        const PopupMenuDivider(height: AmeMenuMetrics.dividerHeight),
        _menuChoice(
          value: _SortMenuAction.ascending,
          label: LibraryStrings.ascending,
          icon: Symbols.arrow_upward_rounded,
          isSelected: direction == LibraryGallerySortDirection.ascending,
        ),
        _menuChoice(
          value: _SortMenuAction.descending,
          label: LibraryStrings.descending,
          icon: Symbols.arrow_downward_rounded,
          isSelected: direction == LibraryGallerySortDirection.descending,
        ),
      ],
      onSelected: (action) {
        switch (action) {
          case _SortMenuAction.captureTime:
            onSortKeyChanged(LibraryGallerySortKey.captureTime);
          case _SortMenuAction.createdTime:
            onSortKeyChanged(LibraryGallerySortKey.createdTime);
          case _SortMenuAction.modifiedTime:
            onSortKeyChanged(LibraryGallerySortKey.modifiedTime);
          case _SortMenuAction.fileName:
            onSortKeyChanged(LibraryGallerySortKey.fileName);
          case _SortMenuAction.ascending:
            onDirectionChanged(LibraryGallerySortDirection.ascending);
          case _SortMenuAction.descending:
            onDirectionChanged(LibraryGallerySortDirection.descending);
        }
      },
      builder: (context, openMenu) => AmeTooltip(
        message: LibraryStrings.sort,
        child: IconButton(
          key: const Key("library-sort-menu"),
          onPressed: openMenu,
          icon: const Row(
            mainAxisSize: MainAxisSize.min,
            children: [
              Icon(Symbols.swap_vert_rounded),
              Icon(Symbols.arrow_drop_down_rounded, size: 18),
            ],
          ),
        ),
      ),
    );
  }
}

class _LayoutMenu extends StatelessWidget {
  const _LayoutMenu({
    required this.shape,
    required this.size,
    required this.onShapeChanged,
    required this.onSizeChanged,
  });

  final GalleryLayoutShape shape;
  final GalleryThumbnailSize size;
  final ValueChanged<GalleryLayoutShape> onShapeChanged;
  final ValueChanged<GalleryThumbnailSize> onSizeChanged;

  @override
  Widget build(BuildContext context) {
    return AmePopupMenuButton<_LayoutMenuAction>(
      labels: const [
        LibraryStrings.equalHeight,
        LibraryStrings.square,
        LibraryStrings.small,
        LibraryStrings.medium,
        LibraryStrings.large,
      ],
      leadingIconWidth: AmeMenuMetrics.selectionIndicatorSlotWidth,
      items: [
        _menuChoice(
          value: _LayoutMenuAction.equalHeight,
          label: LibraryStrings.equalHeight,
          icon: Symbols.view_quilt_rounded,
          isSelected: shape == GalleryLayoutShape.equalHeight,
        ),
        _menuChoice(
          value: _LayoutMenuAction.square,
          label: LibraryStrings.square,
          icon: Symbols.grid_view_rounded,
          isSelected: shape == GalleryLayoutShape.square,
        ),
        const PopupMenuDivider(height: AmeMenuMetrics.dividerHeight),
        _menuChoice(
          value: _LayoutMenuAction.small,
          label: LibraryStrings.small,
          icon: Symbols.grid_4x4_rounded,
          isSelected: size == GalleryThumbnailSize.small,
        ),
        _menuChoice(
          value: _LayoutMenuAction.medium,
          label: LibraryStrings.medium,
          icon: Symbols.grid_view_rounded,
          isSelected: size == GalleryThumbnailSize.medium,
        ),
        _menuChoice(
          value: _LayoutMenuAction.large,
          label: LibraryStrings.large,
          icon: Symbols.crop_square_rounded,
          isSelected: size == GalleryThumbnailSize.large,
        ),
      ],
      onSelected: (action) {
        switch (action) {
          case _LayoutMenuAction.equalHeight:
            onShapeChanged(GalleryLayoutShape.equalHeight);
          case _LayoutMenuAction.square:
            onShapeChanged(GalleryLayoutShape.square);
          case _LayoutMenuAction.small:
            onSizeChanged(GalleryThumbnailSize.small);
          case _LayoutMenuAction.medium:
            onSizeChanged(GalleryThumbnailSize.medium);
          case _LayoutMenuAction.large:
            onSizeChanged(GalleryThumbnailSize.large);
        }
      },
      builder: (context, openMenu) => AmeTooltip(
        message: LibraryStrings.layout,
        child: IconButton(
          key: const Key("library-layout-menu"),
          onPressed: openMenu,
          icon: const Row(
            mainAxisSize: MainAxisSize.min,
            children: [
              Icon(Symbols.grid_view_rounded),
              Icon(Symbols.arrow_drop_down_rounded, size: 18),
            ],
          ),
        ),
      ),
    );
  }
}

class _MoreMenu extends StatelessWidget {
  const _MoreMenu({required this.onSelectAll});

  final VoidCallback onSelectAll;

  @override
  Widget build(BuildContext context) {
    return AmePopupMenuButton<_MoreMenuAction>(
      labels: const [LibraryStrings.selectAll],
      shortcuts: const ["Ctrl+A"],
      viewportRightMargin: AmeMenuMetrics.viewportPadding,
      items: const [
        PopupMenuItem(
          value: _MoreMenuAction.selectAll,
          child: AmeMenuItemContent(
            icon: Symbols.select_all_rounded,
            label: LibraryStrings.selectAll,
            shortcut: "Ctrl+A",
          ),
        ),
      ],
      onSelected: (_) => onSelectAll(),
      builder: (context, openMenu) => AmeTooltip(
        message: LibraryStrings.more,
        child: IconButton(
          key: const Key("library-more-menu"),
          onPressed: openMenu,
          icon: const Icon(Symbols.more_horiz_rounded),
        ),
      ),
    );
  }
}

PopupMenuItem<T> _menuChoice<T>({
  required T value,
  required String label,
  required IconData icon,
  required bool isSelected,
}) {
  return PopupMenuItem<T>(
    value: value,
    child: Semantics(
      key: ValueKey("menu-choice-$label"),
      checked: isSelected,
      inMutuallyExclusiveGroup: true,
      child: Row(
        children: [
          SizedBox(
            width: AmeMenuMetrics.selectionIndicatorSlotWidth,
            child: isSelected
                ? const ExcludeSemantics(
                    child: Icon(
                      Symbols.circle_rounded,
                      size: AmeMenuMetrics.selectionIndicatorSize,
                      fill: 1,
                    ),
                  )
                : null,
          ),
          const SizedBox(width: AmeMenuMetrics.iconLabelGap),
          Expanded(
            child: AmeMenuItemContent(
              icon: icon,
              label: label,
              isSelected: isSelected,
            ),
          ),
        ],
      ),
    ),
  );
}

enum _SortMenuAction {
  captureTime,
  createdTime,
  modifiedTime,
  fileName,
  ascending,
  descending,
}

enum _LayoutMenuAction { equalHeight, square, small, medium, large }

enum _MoreMenuAction { selectAll }
