import "package:flutter/material.dart";
import "package:material_symbols_icons/symbols.dart";

import "../../../../app/presentation/ame_menu.dart";
import "../../../../app/presentation/ame_overlay_semantics.dart";
import "../../domain/library_models.dart";
import "../library_asset_summary.dart";
import "../library_strings.dart";

class LibraryViewerToolbar extends StatelessWidget {
  const LibraryViewerToolbar({
    required this.asset,
    required this.isFilmstripVisible,
    required this.onToggleFilmstrip,
    required this.onInformation,
    required this.onCopyPath,
    required this.onRevealFile,
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
    this.organizationActions,
    super.key,
  });

  static const double height = 56;
  final LibraryAsset asset;
  final bool isFilmstripVisible;
  final VoidCallback? onToggleFilmstrip;
  final VoidCallback onInformation;
  final VoidCallback onCopyPath;
  final VoidCallback onRevealFile;
  final String? positionLabel;
  final Widget? organizationActions;
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
      libraryFileType(asset.displayPath),
      if (asset.width > 0 && asset.height > 0)
        "${asset.width} × ${asset.height}",
      libraryFileSize(asset.fileSize),
    ].join("  ·  ");
    final textStyle = Theme.of(context).textTheme.bodyMedium;
    return Material(
      key: const Key("viewer-zoom-controls"),
      color: Theme.of(context).colorScheme.surfaceContainerLow,
      child: SizedBox(
        height: height,
        child: LayoutBuilder(
          builder: (context, constraints) {
            final compact = constraints.maxWidth < 900;
            return Padding(
              padding: const EdgeInsets.symmetric(horizontal: 8),
              child: Row(
                children: [
                  Expanded(
                    child: Row(
                      children: [
                        _button(
                          key: "viewer-filmstrip-toggle",
                          label: isFilmstripVisible
                              ? LibraryStrings.hideFilmstrip
                              : LibraryStrings.showFilmstrip,
                          icon: Symbols.view_carousel_rounded,
                          selected: isFilmstripVisible,
                          onPressed: onToggleFilmstrip,
                        ),
                        ?organizationActions,
                        _button(
                          key: "viewer-information",
                          label: LibraryStrings.viewInformation,
                          icon: Symbols.info_rounded,
                          onPressed: onInformation,
                        ),
                        AmePopupMenuButton<_ViewerMenuAction>(
                          openAboveAnchor: true,
                          labels: const [
                            LibraryStrings.copyPath,
                            LibraryStrings.openInExplorer,
                          ],
                          items: const [
                            PopupMenuItem(
                              value: _ViewerMenuAction.copyPath,
                              child: AmeMenuItemContent(
                                icon: Symbols.content_copy_rounded,
                                label: LibraryStrings.copyPath,
                              ),
                            ),
                            PopupMenuItem(
                              value: _ViewerMenuAction.openInExplorer,
                              child: AmeMenuItemContent(
                                icon: Symbols.folder_open_rounded,
                                label: LibraryStrings.openInExplorer,
                              ),
                            ),
                          ],
                          onSelected: (action) {
                            switch (action) {
                              case _ViewerMenuAction.copyPath:
                                onCopyPath();
                              case _ViewerMenuAction.openInExplorer:
                                onRevealFile();
                            }
                          },
                          builder: (context, openMenu) => _button(
                            key: "viewer-more-menu",
                            label: LibraryStrings.more,
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
                    ),
                  ),
                  const SizedBox(width: 16),
                  _button(
                    key: "viewer-zoom-mode",
                    label: isActualSize
                        ? LibraryStrings.fitImageToWindow
                        : LibraryStrings.showImageActualSize,
                    icon: isActualSize
                        ? Symbols.fit_screen_rounded
                        : Symbols.aspect_ratio_rounded,
                    onPressed: onToggleZoomMode,
                  ),
                  SizedBox(
                    width: 62,
                    child: Text(
                      "$zoomPercent%",
                      key: const Key("viewer-zoom-percentage"),
                      textAlign: TextAlign.center,
                      style: textStyle,
                    ),
                  ),
                  _button(
                    key: "viewer-zoom-out",
                    label: "缩小（- / Ctrl+-）",
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
                  _button(
                    key: "viewer-zoom-in",
                    label: "放大（+ / Ctrl++）",
                    icon: Symbols.add_rounded,
                    onPressed: onZoomIn,
                  ),
                  const SizedBox(width: 8),
                  _button(
                    key: "viewer-fullscreen",
                    label: isFullscreen
                        ? LibraryStrings.exitViewerFullscreen
                        : LibraryStrings.enterViewerFullscreen,
                    icon: isFullscreen
                        ? Symbols.fullscreen_exit_rounded
                        : Symbols.fullscreen_rounded,
                    onPressed: onToggleFullscreen,
                  ),
                ],
              ),
            );
          },
        ),
      ),
    );
  }

  Widget _button({
    required String key,
    required String label,
    required IconData icon,
    required VoidCallback? onPressed,
    bool? selected,
  }) => AmeTooltip(
    message: label,
    child: IconButton(
      key: Key(key),
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

enum _ViewerMenuAction { copyPath, openInExplorer }
