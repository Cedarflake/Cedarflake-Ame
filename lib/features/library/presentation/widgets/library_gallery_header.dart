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

class _SortMenu extends StatefulWidget {
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
  State<_SortMenu> createState() => _SortMenuState();
}

class _SortMenuState extends State<_SortMenu> {
  final MenuController _controller = MenuController();

  @override
  Widget build(BuildContext context) {
    final menuWidth = amePopupMenuContentWidth(
      context: context,
      labels: const [
        LibraryStrings.captureDate,
        LibraryStrings.createdDate,
        LibraryStrings.modifiedDate,
        LibraryStrings.fileName,
        LibraryStrings.ascending,
        LibraryStrings.descending,
      ],
      leadingIconWidth: AmeMenuMetrics.selectionIndicatorSlotWidth,
    );
    Widget fixedWidthChoice(Widget choice) {
      return ameFixedWidthMenuItem(width: menuWidth, child: choice);
    }

    return AmeMenuAnchor(
      controller: _controller,
      style: ameFixedWidthMenuStyle(menuWidth),
      alignmentOffset: ameMenuBelowEndAlignment(menuWidth: menuWidth),
      menuChildren: [
        fixedWidthChoice(
          _menuChoice(
            label: LibraryStrings.captureDate,
            icon: Symbols.calendar_month_rounded,
            isSelected: widget.sortKey == LibraryGallerySortKey.captureTime,
            onPressed: () =>
                widget.onSortKeyChanged(LibraryGallerySortKey.captureTime),
          ),
        ),
        fixedWidthChoice(
          _menuChoice(
            label: LibraryStrings.createdDate,
            icon: Symbols.create_new_folder_rounded,
            isSelected: widget.sortKey == LibraryGallerySortKey.createdTime,
            onPressed: () =>
                widget.onSortKeyChanged(LibraryGallerySortKey.createdTime),
          ),
        ),
        fixedWidthChoice(
          _menuChoice(
            label: LibraryStrings.modifiedDate,
            icon: Symbols.edit_calendar_rounded,
            isSelected: widget.sortKey == LibraryGallerySortKey.modifiedTime,
            onPressed: () =>
                widget.onSortKeyChanged(LibraryGallerySortKey.modifiedTime),
          ),
        ),
        fixedWidthChoice(
          _menuChoice(
            label: LibraryStrings.fileName,
            icon: Symbols.text_fields_rounded,
            isSelected: widget.sortKey == LibraryGallerySortKey.fileName,
            onPressed: () =>
                widget.onSortKeyChanged(LibraryGallerySortKey.fileName),
          ),
        ),
        const Divider(height: AmeMenuMetrics.dividerHeight),
        fixedWidthChoice(
          _menuChoice(
            label: LibraryStrings.ascending,
            icon: Symbols.arrow_upward_rounded,
            isSelected:
                widget.direction == LibraryGallerySortDirection.ascending,
            onPressed: () => widget.onDirectionChanged(
              LibraryGallerySortDirection.ascending,
            ),
          ),
        ),
        fixedWidthChoice(
          _menuChoice(
            label: LibraryStrings.descending,
            icon: Symbols.arrow_downward_rounded,
            isSelected:
                widget.direction == LibraryGallerySortDirection.descending,
            onPressed: () => widget.onDirectionChanged(
              LibraryGallerySortDirection.descending,
            ),
          ),
        ),
      ],
      builder: (context, controller, child) => AmeTooltip(
        message: LibraryStrings.sort,
        child: IconButton(
          key: const Key("library-sort-menu"),
          onPressed: () => toggleAmeMenu(controller),
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

class _LayoutMenu extends StatefulWidget {
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
  State<_LayoutMenu> createState() => _LayoutMenuState();
}

class _LayoutMenuState extends State<_LayoutMenu> {
  final MenuController _controller = MenuController();

  @override
  Widget build(BuildContext context) {
    return AmeMenuAnchor(
      controller: _controller,
      menuChildren: [
        _menuChoice(
          label: LibraryStrings.equalHeight,
          icon: Symbols.view_quilt_rounded,
          isSelected: widget.shape == GalleryLayoutShape.equalHeight,
          onPressed: () =>
              widget.onShapeChanged(GalleryLayoutShape.equalHeight),
        ),
        _menuChoice(
          label: LibraryStrings.square,
          icon: Symbols.grid_view_rounded,
          isSelected: widget.shape == GalleryLayoutShape.square,
          onPressed: () => widget.onShapeChanged(GalleryLayoutShape.square),
        ),
        const Divider(height: AmeMenuMetrics.dividerHeight),
        _menuChoice(
          label: LibraryStrings.small,
          icon: Symbols.grid_4x4_rounded,
          isSelected: widget.size == GalleryThumbnailSize.small,
          onPressed: () => widget.onSizeChanged(GalleryThumbnailSize.small),
        ),
        _menuChoice(
          label: LibraryStrings.medium,
          icon: Symbols.grid_view_rounded,
          isSelected: widget.size == GalleryThumbnailSize.medium,
          onPressed: () => widget.onSizeChanged(GalleryThumbnailSize.medium),
        ),
        _menuChoice(
          label: LibraryStrings.large,
          icon: Symbols.crop_square_rounded,
          isSelected: widget.size == GalleryThumbnailSize.large,
          onPressed: () => widget.onSizeChanged(GalleryThumbnailSize.large),
        ),
      ],
      builder: (context, controller, child) => AmeTooltip(
        message: LibraryStrings.layout,
        child: IconButton(
          key: const Key("library-layout-menu"),
          onPressed: () => toggleAmeMenu(controller),
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
    final menuWidth = amePopupMenuContentWidth(
      context: context,
      labels: const [LibraryStrings.selectAll],
      shortcuts: const ["Ctrl+A"],
    );
    return AmeMenuAnchor(
      style: ameFixedWidthMenuStyle(menuWidth),
      alignmentOffset: ameMenuBelowEndAlignment(
        menuWidth: menuWidth,
        endOffset: 8,
      ),
      menuChildren: [
        ameFixedWidthMenuItem(
          width: menuWidth,
          child: MenuItemButton(
            onPressed: onSelectAll,
            child: const AmeMenuItemContent(
              icon: Symbols.select_all_rounded,
              label: LibraryStrings.selectAll,
              shortcut: "Ctrl+A",
            ),
          ),
        ),
      ],
      builder: (context, controller, child) => AmeTooltip(
        message: LibraryStrings.more,
        child: IconButton(
          key: const Key("library-more-menu"),
          onPressed: () => toggleAmeMenu(controller),
          icon: const Icon(Symbols.more_horiz_rounded),
        ),
      ),
    );
  }
}

Widget _menuChoice({
  required String label,
  required IconData icon,
  required bool isSelected,
  required VoidCallback onPressed,
}) {
  return Semantics(
    key: ValueKey("menu-choice-$label"),
    checked: isSelected,
    inMutuallyExclusiveGroup: true,
    child: MenuItemButton(
      onPressed: onPressed,
      leadingIcon: SizedBox(
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
      child: AmeMenuItemContent(
        icon: icon,
        label: label,
        isSelected: isSelected,
      ),
    ),
  );
}
