import "package:material_ui/material_ui.dart";
import "package:material_symbols_icons/symbols.dart";

import "../../../../app/presentation/ame_localizations.dart";
import "../../../../app/presentation/ame_menu.dart";
import "../../../../app/presentation/ame_overlay_semantics.dart";
import "../../../../app/presentation/ame_workbench_surfaces.dart";
import "../../domain/library_models.dart";
import "../library_asset_summary.dart";

class LibraryViewerToolbar extends StatelessWidget {
  const LibraryViewerToolbar({
    required this.asset,
    this.isFilmstripVisible = false,
    this.onToggleFilmstrip,
    this.onInformation,
    this.onCopyPath,
    this.onCopyImage,
    this.onRevealFile,
    this.onRotate,
    required this.isActualSize,
    required this.onToggleZoomMode,
    required this.sliderValue,
    required this.zoomPercent,
    required this.onSliderChanged,
    required this.sliderSemanticFormatter,
    required this.onZoomOut,
    required this.onZoomIn,
    required this.isFullscreen,
    required this.onToggleFullscreen,
    this.positionLabel,
    this.leading,
    super.key,
  });

  static const double height = 56;
  final LibraryAsset asset;
  final bool isFilmstripVisible;
  final VoidCallback? onToggleFilmstrip;
  final VoidCallback? onInformation;
  final VoidCallback? onCopyPath;
  final VoidCallback? onCopyImage;
  final VoidCallback? onRevealFile;
  final VoidCallback? onRotate;
  final String? positionLabel;
  final Widget? leading;
  final bool isActualSize;
  final VoidCallback? onToggleZoomMode;
  final double sliderValue;
  final int zoomPercent;
  final ValueChanged<double> onSliderChanged;
  final SemanticFormatterCallback sliderSemanticFormatter;
  final VoidCallback? onZoomOut;
  final VoidCallback? onZoomIn;
  final bool isFullscreen;
  final VoidCallback? onToggleFullscreen;

  @override
  Widget build(BuildContext context) {
    final summary = [
      libraryFileType(context.l10n, asset.displayPath),
      if (asset.width > 0 && asset.height > 0)
        "${asset.width} × ${asset.height}",
      libraryFileSize(asset.fileSize),
    ].join("  ·  ");
    final textStyle = Theme.of(context).textTheme.bodyMedium;
    return Material(
      key: const Key("viewer-zoom-controls"),
      color: AmeWorkbenchSurfaces.of(context).chrome,
      child: LayoutBuilder(
        builder: (context, constraints) {
          final compact = constraints.maxWidth < 900;
          final details =
              leading ??
              Row(
                children: [
                  LibraryViewerToolbarButton(
                    id: "viewer-filmstrip-toggle",
                    label: isFilmstripVisible
                        ? context.l10n.hideFilmstrip
                        : context.l10n.showFilmstrip,
                    icon: Symbols.view_carousel_rounded,
                    selected: isFilmstripVisible,
                    onPressed: onToggleFilmstrip,
                  ),
                  LibraryViewerToolbarButton(
                    id: "viewer-information",
                    label: context.l10n.viewInformation,
                    icon: Symbols.info_rounded,
                    onPressed: onInformation,
                  ),
                  AmePopupMenuButton<_ViewerMenuAction>(
                    openAboveAnchor: true,
                    labels: [
                      if (onCopyImage != null) context.l10n.copyPhoto,
                      context.l10n.copyPath,
                      context.l10n.openInExplorer,
                    ],
                    shortcuts: [if (onCopyImage != null) "Ctrl+C"],
                    items: [
                      if (onCopyImage != null)
                        PopupMenuItem(
                          value: _ViewerMenuAction.copyImage,
                          child: AmeMenuItemContent(
                            icon: Symbols.content_copy_rounded,
                            label: context.l10n.copyPhoto,
                            shortcut: "Ctrl+C",
                          ),
                        ),
                      PopupMenuItem(
                        value: _ViewerMenuAction.copyPath,
                        child: AmeMenuItemContent(
                          icon: Symbols.content_copy_rounded,
                          label: context.l10n.copyPath,
                        ),
                      ),
                      PopupMenuItem(
                        value: _ViewerMenuAction.openInExplorer,
                        child: AmeMenuItemContent(
                          icon: Symbols.folder_open_rounded,
                          label: context.l10n.openInExplorer,
                        ),
                      ),
                    ],
                    onSelected: (action) {
                      switch (action) {
                        case _ViewerMenuAction.copyImage:
                          onCopyImage?.call();
                        case _ViewerMenuAction.copyPath:
                          onCopyPath?.call();
                        case _ViewerMenuAction.openInExplorer:
                          onRevealFile?.call();
                      }
                    },
                    builder: (context, openMenu) => LibraryViewerToolbarButton(
                      id: "viewer-more-menu",
                      label: context.l10n.more,
                      icon: Symbols.more_horiz_rounded,
                      onPressed: openMenu,
                    ),
                  ),
                  const SizedBox(width: 8),
                  Flexible(
                    child: AmeTooltip(
                      message: summary,
                      child: Text(
                        summary,
                        key: const Key("viewer-image-summary"),
                        maxLines: 1,
                        overflow: TextOverflow.ellipsis,
                        style: textStyle,
                      ),
                    ),
                  ),
                  if (positionLabel case final position?) ...[
                    const VerticalDivider(
                      width: 24,
                      thickness: 1,
                      indent: 18,
                      endIndent: 18,
                    ),
                    Text(
                      position,
                      key: const Key("viewer-position"),
                      style: textStyle,
                    ),
                  ],
                ],
              );
          final zoomControls = Row(
            mainAxisSize: MainAxisSize.min,
            children: [
              if (onRotate != null)
                LibraryViewerToolbarButton(
                  id: "viewer-rotate-clockwise",
                  label: context.l10n.rotateClockwise,
                  icon: Symbols.rotate_90_degrees_cw_rounded,
                  onPressed: onRotate,
                ),
              LibraryViewerToolbarButton(
                id: "viewer-zoom-mode",
                label: isActualSize
                    ? context.l10n.fitImageToWindow
                    : context.l10n.showImageActualSize,
                icon: isActualSize
                    ? Symbols.fit_screen_rounded
                    : Symbols.aspect_ratio_rounded,
                onPressed: onToggleZoomMode,
              ),
              ConstrainedBox(
                constraints: const BoxConstraints(minWidth: 62),
                child: Text(
                  "$zoomPercent%",
                  key: const Key("viewer-zoom-percentage"),
                  textAlign: TextAlign.center,
                  maxLines: 1,
                  softWrap: false,
                  style: textStyle,
                ),
              ),
              LibraryViewerToolbarButton(
                id: "viewer-zoom-out",
                label: context.l10n.zoomOutShortcut,
                icon: Symbols.remove_rounded,
                onPressed: onZoomOut,
              ),
              SizedBox(
                width: compact ? 72 : 132,
                child: SliderTheme(
                  data: SliderTheme.of(context).copyWith(
                    thumbShape: const RoundSliderThumbShape(
                      enabledThumbRadius: 7,
                    ),
                  ),
                  child: Slider(
                    padding: const EdgeInsets.symmetric(horizontal: 8),
                    value: sliderValue,
                    onChanged: onSliderChanged,
                    semanticFormatterCallback: sliderSemanticFormatter,
                  ),
                ),
              ),
              LibraryViewerToolbarButton(
                id: "viewer-zoom-in",
                label: context.l10n.zoomInShortcut,
                icon: Symbols.add_rounded,
                onPressed: onZoomIn,
              ),
              const SizedBox(width: 8),
              LibraryViewerToolbarButton(
                id: "viewer-fullscreen",
                label: isFullscreen
                    ? context.l10n.exitViewerFullscreen
                    : context.l10n.enterViewerFullscreen,
                icon: isFullscreen
                    ? Symbols.fullscreen_exit_rounded
                    : Symbols.fullscreen_rounded,
                onPressed: onToggleFullscreen,
              ),
            ],
          );
          return Padding(
            padding: const EdgeInsets.symmetric(horizontal: 8),
            child: SizedBox(
              height: height,
              child: Row(
                children: [
                  Expanded(child: details),
                  if (leading == null) const SizedBox(width: 16),
                  zoomControls,
                ],
              ),
            ),
          );
        },
      ),
    );
  }
}

class LibraryViewerToolbarButton extends StatelessWidget {
  const LibraryViewerToolbarButton({
    required this.id,
    required this.label,
    required this.icon,
    required this.onPressed,
    this.selected,
    super.key,
  });

  final String id;
  final String label;
  final IconData icon;
  final VoidCallback? onPressed;
  final bool? selected;

  @override
  Widget build(BuildContext context) => AmeTooltip(
    message: label,
    child: IconButton(
      key: Key(id),
      isSelected: selected,
      onPressed: onPressed,
      icon: Icon(
        icon,
        size: 20,
        opticalSize: 20,
        weight: 400,
        semanticLabel: label,
      ),
    ),
  );
}

enum _ViewerMenuAction { copyImage, copyPath, openInExplorer }
