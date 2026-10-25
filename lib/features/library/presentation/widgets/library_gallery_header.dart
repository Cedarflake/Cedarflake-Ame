import "package:flutter/material.dart";
import "package:material_symbols_icons/symbols.dart";

import "../../../../app/presentation/ame_menu.dart";
import "../../../../app/presentation/ame_menu_choice.dart";
import "../../../../app/presentation/ame_overlay_semantics.dart";
import "../../../../app/presentation/ame_scroll_motion.dart";
import "../../domain/library_models.dart";
import "../gallery_view_options.dart";
import "../library_strings.dart";
import "library_gallery_header_actions.dart";
import "library_sidebar_content.dart";

class LibraryGalleryHeader extends StatelessWidget {
  const LibraryGalleryHeader({
    required this.galleryTitle,
    this.leading,
    this.filter,
    this.selectionToolbar,
    this.reviewToolbar,
    this.reviewCounts,
    this.isReviewing = false,
    this.onToggleReview,
    required this.totalItems,
    this.mergedItems,
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
  final Widget? leading;
  final Widget? filter;
  final Widget? selectionToolbar;
  final Widget? reviewToolbar;
  final LibraryOrganizationCounts? reviewCounts;
  final bool isReviewing;
  final VoidCallback? onToggleReview;
  final int totalItems;
  final int? mergedItems;
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
    var subtitle = "$totalItems 张图片";
    if (mergedItems case final count?) {
      subtitle = "$subtitle · 去重后 $count 张";
    }
    if (reviewCounts case final counts?) {
      subtitle =
          "${counts.total} 张图片 · 已整理 ${counts.organized} · 待整理 ${counts.pending} · 已忽略 ${counts.ignored}";
    } else if (isSelecting) {
      subtitle = galleryTitle;
    }
    return SizedBox(
      key: const Key("library-gallery-header"),
      width: double.infinity,
      child: ConstrainedBox(
        constraints: const BoxConstraints(minHeight: 104),
        child: Padding(
          padding: const EdgeInsets.fromLTRB(28, 18, 20, 16),
          child: LayoutBuilder(
            builder: (context, constraints) {
              final titleText = Wrap(
                spacing: 12,
                crossAxisAlignment: WrapCrossAlignment.center,
                children: [
                  Text(
                    title,
                    key: const Key("library-gallery-title"),
                    style: Theme.of(context).textTheme.headlineSmall,
                  ),
                  if (isReviewing)
                    Text(
                      "整理模式",
                      style: Theme.of(context).textTheme.bodyMedium?.copyWith(
                        color: Theme.of(context).colorScheme.primary,
                      ),
                    ),
                ],
              );
              final summary = LibrarySidebarLeading(
                child: AmeScrollMotion(
                  scrollDirection: Axis.horizontal,
                  child: SingleChildScrollView(
                    scrollDirection: Axis.horizontal,
                    child: Text(
                      subtitle,
                      key: const Key("library-summary"),
                      style: Theme.of(context).textTheme.bodyMedium?.copyWith(
                        color: Theme.of(context).colorScheme.onSurfaceVariant,
                      ),
                    ),
                  ),
                ),
              );
              final titleBlock = LibrarySidebarLeading(
                child: Row(
                  mainAxisSize: MainAxisSize.min,
                  children: [
                    if (leading != null) ...[
                      leading!,
                      const SizedBox(width: 8),
                    ],
                    Flexible(child: titleText),
                  ],
                ),
              );
              final toolbar = isSelecting
                  ? selectionToolbar ??
                        _SelectionToolbar(onCancel: onCancelSelection)
                  : _BrowsingToolbar(
                      filter: filter,
                      isReviewing: isReviewing,
                      onToggleReview: onToggleReview,
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
                    const SizedBox(height: 4),
                    summary,
                    const SizedBox(height: 12),
                    Align(
                      alignment: Alignment.centerRight,
                      child: LibraryGalleryHeaderActions(
                        primary: toolbar,
                        secondary: reviewToolbar,
                      ),
                    ),
                  ],
                );
              }
              return LibraryGalleryHeaderActions(
                title: titleBlock,
                summary: summary,
                primary: toolbar,
                secondary: reviewToolbar,
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
    this.filter,
    required this.isReviewing,
    this.onToggleReview,
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

  final Widget? filter;
  final bool isReviewing;
  final VoidCallback? onToggleReview;
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
          isReviewing: isReviewing,
          sortKey: sortKey,
          direction: sortDirection,
          onSortKeyChanged: onSortKeyChanged,
          onDirectionChanged: onSortDirectionChanged,
        ),
        ?filter,
        _LayoutMenu(
          shape: layoutShape,
          size: thumbnailSize,
          onShapeChanged: onLayoutShapeChanged,
          onSizeChanged: onThumbnailSizeChanged,
        ),
        _MoreMenu(
          onSelectAll: onSelectAll,
          isReviewing: isReviewing,
          onToggleReview: onToggleReview,
        ),
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
    required this.isReviewing,
    required this.sortKey,
    required this.direction,
    required this.onSortKeyChanged,
    required this.onDirectionChanged,
  });

  final bool isReviewing;
  final LibraryGallerySortKey sortKey;
  final LibraryGallerySortDirection direction;
  final ValueChanged<LibraryGallerySortKey> onSortKeyChanged;
  final ValueChanged<LibraryGallerySortDirection> onDirectionChanged;

  @override
  Widget build(BuildContext context) {
    return AmePopupMenuButton<_SortMenuAction>(
      labels: [
        LibraryStrings.captureDate,
        LibraryStrings.createdDate,
        LibraryStrings.modifiedDate,
        LibraryStrings.fileName,
        if (isReviewing) LibraryStrings.newestAddedFirst,
        LibraryStrings.ascending,
        LibraryStrings.descending,
      ],
      leadingIconWidth: AmeMenuMetrics.selectionIndicatorSlotWidth,
      items: [
        ameMenuChoice(
          value: _SortMenuAction.captureTime,
          label: LibraryStrings.captureDate,
          icon: Symbols.calendar_month_rounded,
          isSelected: sortKey == LibraryGallerySortKey.captureTime,
        ),
        ameMenuChoice(
          value: _SortMenuAction.createdTime,
          label: LibraryStrings.createdDate,
          icon: Symbols.create_new_folder_rounded,
          isSelected: sortKey == LibraryGallerySortKey.createdTime,
        ),
        ameMenuChoice(
          value: _SortMenuAction.modifiedTime,
          label: LibraryStrings.modifiedDate,
          icon: Symbols.edit_calendar_rounded,
          isSelected: sortKey == LibraryGallerySortKey.modifiedTime,
        ),
        ameMenuChoice(
          value: _SortMenuAction.fileName,
          label: LibraryStrings.fileName,
          icon: Symbols.text_fields_rounded,
          isSelected: sortKey == LibraryGallerySortKey.fileName,
        ),
        if (isReviewing)
          ameMenuChoice(
            value: _SortMenuAction.libraryAdmission,
            label: LibraryStrings.newestAddedFirst,
            icon: Symbols.new_releases_rounded,
            isSelected: sortKey == LibraryGallerySortKey.libraryAdmission,
          ),
        if (sortKey != LibraryGallerySortKey.libraryAdmission) ...[
          const PopupMenuDivider(height: AmeMenuMetrics.dividerHeight),
          ameMenuChoice(
            value: _SortMenuAction.ascending,
            label: LibraryStrings.ascending,
            icon: Symbols.arrow_upward_rounded,
            isSelected: direction == LibraryGallerySortDirection.ascending,
          ),
          ameMenuChoice(
            value: _SortMenuAction.descending,
            label: LibraryStrings.descending,
            icon: Symbols.arrow_downward_rounded,
            isSelected: direction == LibraryGallerySortDirection.descending,
          ),
        ],
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
          case _SortMenuAction.libraryAdmission:
            onSortKeyChanged(LibraryGallerySortKey.libraryAdmission);
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
        ameMenuChoice(
          value: _LayoutMenuAction.equalHeight,
          label: LibraryStrings.equalHeight,
          icon: Symbols.view_quilt_rounded,
          isSelected: shape == GalleryLayoutShape.equalHeight,
        ),
        ameMenuChoice(
          value: _LayoutMenuAction.square,
          label: LibraryStrings.square,
          icon: Symbols.grid_view_rounded,
          isSelected: shape == GalleryLayoutShape.square,
        ),
        const PopupMenuDivider(height: AmeMenuMetrics.dividerHeight),
        ameMenuChoice(
          value: _LayoutMenuAction.small,
          label: LibraryStrings.small,
          icon: Symbols.grid_4x4_rounded,
          isSelected: size == GalleryThumbnailSize.small,
        ),
        ameMenuChoice(
          value: _LayoutMenuAction.medium,
          label: LibraryStrings.medium,
          icon: Symbols.grid_view_rounded,
          isSelected: size == GalleryThumbnailSize.medium,
        ),
        ameMenuChoice(
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
  const _MoreMenu({
    required this.onSelectAll,
    required this.isReviewing,
    this.onToggleReview,
  });

  final VoidCallback onSelectAll;
  final bool isReviewing;
  final VoidCallback? onToggleReview;

  @override
  Widget build(BuildContext context) {
    final reviewLabel = isReviewing ? "退出整理" : "整理模式";
    return AmePopupMenuButton<_MoreMenuAction>(
      labels: [
        LibraryStrings.selectAll,
        if (onToggleReview != null) reviewLabel,
      ],
      shortcuts: const ["Ctrl+A"],
      viewportRightMargin: AmeMenuMetrics.viewportPadding,
      items: [
        const PopupMenuItem(
          value: _MoreMenuAction.selectAll,
          child: AmeMenuItemContent(
            icon: Symbols.select_all_rounded,
            label: LibraryStrings.selectAll,
            shortcut: "Ctrl+A",
          ),
        ),
        if (onToggleReview != null)
          PopupMenuItem(
            value: _MoreMenuAction.review,
            child: AmeMenuItemContent(
              icon: Symbols.checklist_rounded,
              label: reviewLabel,
            ),
          ),
      ],
      onSelected: (action) {
        switch (action) {
          case _MoreMenuAction.selectAll:
            onSelectAll();
          case _MoreMenuAction.review:
            onToggleReview?.call();
        }
      },
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

enum _SortMenuAction {
  libraryAdmission,
  captureTime,
  createdTime,
  modifiedTime,
  fileName,
  ascending,
  descending,
}

enum _LayoutMenuAction { equalHeight, square, small, medium, large }

enum _MoreMenuAction { selectAll, review }
