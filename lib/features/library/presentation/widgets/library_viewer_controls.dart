import "dart:async";

import "package:flutter/material.dart";

import "../../../../app/presentation/ame_menu.dart";
import "../../../../app/presentation/ame_popup_menu_position.dart";
import "../../../../app/window/ame_window_chrome.dart";
import "../library_strings.dart";
import "library_path_text.dart";

class LibraryViewerTopBar extends StatelessWidget {
  const LibraryViewerTopBar({
    required this.displayPath,
    required this.onBack,
    required this.onInformation,
    required this.onCopyPath,
    required this.onRevealFile,
    this.positionLabel,
    super.key,
  });

  final String displayPath;
  final String? positionLabel;
  final VoidCallback onBack;
  final VoidCallback onInformation;
  final VoidCallback onCopyPath;
  final VoidCallback onRevealFile;

  @override
  Widget build(BuildContext context) {
    return Material(
      key: const Key("viewer-window-bar"),
      color: Theme.of(context).colorScheme.surfaceContainerLow,
      child: SizedBox(
        height: 64,
        child: Row(
          children: [
            const SizedBox(width: 8),
            IconButton(
              tooltip: "${LibraryStrings.backToLibrary}（Esc）",
              onPressed: onBack,
              icon: const Icon(Icons.arrow_back),
            ),
            const SizedBox(width: 8),
            Expanded(
              child: AmeWindowDragRegion(
                key: const Key("viewer-window-drag-region"),
                child: Row(
                  children: [
                    Expanded(
                      child: DefaultTextStyle.merge(
                        style: Theme.of(context).textTheme.titleMedium,
                        child: LibraryPathText(
                          text: displayLibraryFileName(displayPath),
                          path: displayPath,
                          alwaysShowPathTooltip: true,
                          textKey: const Key("viewer-source-path"),
                        ),
                      ),
                    ),
                    if (positionLabel case final label?) ...[
                      const SizedBox(width: 16),
                      Text(
                        label,
                        style: Theme.of(context).textTheme.labelLarge?.copyWith(
                          color: Theme.of(context).colorScheme.onSurfaceVariant,
                        ),
                      ),
                    ],
                    const SizedBox(width: 8),
                  ],
                ),
              ),
            ),
            IconButton(
              tooltip: LibraryStrings.viewInformation,
              onPressed: onInformation,
              icon: const Icon(Icons.info_outline),
            ),
            Builder(
              builder: (buttonContext) => IconButton(
                key: const Key("viewer-more-menu"),
                tooltip: LibraryStrings.more,
                onPressed: () {
                  unawaited(_showMoreMenu(buttonContext));
                },
                icon: const Icon(Icons.more_horiz),
              ),
            ),
            const AmeWindowCaptionControls(height: 64),
          ],
        ),
      ),
    );
  }

  Future<void> _showMoreMenu(BuildContext anchorContext) async {
    final position = amePopupMenuBelowAnchor(
      context: anchorContext,
      anchorContext: anchorContext,
      viewportRightMargin: 16,
    );
    if (position == null) {
      return;
    }
    const labels = [LibraryStrings.copyPath, LibraryStrings.openInExplorer];
    final action = await showAmePopupMenu<_ViewerMenuAction>(
      context: anchorContext,
      position: position,
      labels: labels,
      items: const [
        PopupMenuItem(
          value: _ViewerMenuAction.copyPath,
          child: AmeMenuItemContent(
            icon: Icons.content_copy_outlined,
            label: LibraryStrings.copyPath,
          ),
        ),
        PopupMenuItem(
          value: _ViewerMenuAction.revealFile,
          child: AmeMenuItemContent(
            icon: Icons.folder_open_outlined,
            label: LibraryStrings.openInExplorer,
          ),
        ),
      ],
    );
    switch (action) {
      case _ViewerMenuAction.copyPath:
        onCopyPath();
      case _ViewerMenuAction.revealFile:
        onRevealFile();
      case null:
        return;
    }
  }
}

enum _ViewerMenuAction { copyPath, revealFile }

class LibraryViewerNavigationButton extends StatelessWidget {
  const LibraryViewerNavigationButton.previous({
    required this.onPressed,
    super.key,
  }) : isPrevious = true;

  const LibraryViewerNavigationButton.next({required this.onPressed, super.key})
    : isPrevious = false;

  final bool isPrevious;
  final VoidCallback? onPressed;

  @override
  Widget build(BuildContext context) {
    final button = Center(
      child: IconButton.filledTonal(
        key: Key(isPrevious ? "viewer-previous" : "viewer-next"),
        tooltip: isPrevious ? "上一张（←）" : "下一张（→）",
        onPressed: onPressed,
        icon: Icon(isPrevious ? Icons.chevron_left : Icons.chevron_right),
      ),
    );
    return Positioned(
      left: isPrevious ? 20 : null,
      right: isPrevious ? null : 20,
      top: 0,
      bottom: 0,
      child: button,
    );
  }
}

class LibraryViewerZoomControls extends StatelessWidget {
  const LibraryViewerZoomControls({
    required this.sliderValue,
    required this.zoomPercent,
    required this.canZoomOut,
    required this.canZoomIn,
    required this.canShowActualSize,
    required this.onSliderChanged,
    required this.sliderSemanticFormatter,
    required this.onZoomOut,
    required this.onZoomIn,
    required this.onFitToWindow,
    required this.onShowActualSize,
    super.key,
  });

  final double sliderValue;
  final int zoomPercent;
  final bool canZoomOut;
  final bool canZoomIn;
  final bool canShowActualSize;
  final ValueChanged<double> onSliderChanged;
  final SemanticFormatterCallback sliderSemanticFormatter;
  final VoidCallback onZoomOut;
  final VoidCallback onZoomIn;
  final VoidCallback onFitToWindow;
  final VoidCallback onShowActualSize;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    return Material(
      key: const Key("viewer-zoom-controls"),
      color: colorScheme.surfaceContainerHigh.withValues(alpha: 0.94),
      elevation: 2,
      borderRadius: BorderRadius.circular(28),
      child: SizedBox(
        height: 52,
        child: Padding(
          padding: const EdgeInsets.symmetric(horizontal: 4),
          child: Row(
            mainAxisSize: MainAxisSize.min,
            children: [
              IconButton(
                tooltip: "缩小（- / Ctrl+-）",
                onPressed: canZoomOut ? onZoomOut : null,
                icon: const Icon(Icons.remove),
              ),
              SizedBox(
                width: 164,
                child: Slider(
                  value: sliderValue,
                  onChanged: onSliderChanged,
                  semanticFormatterCallback: sliderSemanticFormatter,
                ),
              ),
              SizedBox(
                width: 58,
                child: Text(
                  "$zoomPercent%",
                  key: const Key("viewer-zoom-percentage"),
                  textAlign: TextAlign.center,
                  style: Theme.of(context).textTheme.labelLarge,
                ),
              ),
              IconButton(
                tooltip: "放大（+ / Ctrl++）",
                onPressed: canZoomIn ? onZoomIn : null,
                icon: const Icon(Icons.add),
              ),
              const SizedBox(
                key: Key("viewer-zoom-group-divider"),
                width: 24,
                height: 28,
                child: VerticalDivider(width: 24),
              ),
              IconButton(
                key: const Key("viewer-fit"),
                tooltip: "适合窗口（0 / Ctrl+0）",
                onPressed: onFitToWindow,
                icon: const Icon(Icons.fit_screen_outlined),
              ),
              const SizedBox(width: 4),
              Tooltip(
                message: "实际大小（1 / Ctrl+1）",
                child: TextButton(
                  key: const Key("viewer-actual-size"),
                  onPressed: canShowActualSize ? onShowActualSize : null,
                  child: const Text("1:1"),
                ),
              ),
              const SizedBox(width: 4),
            ],
          ),
        ),
      ),
    );
  }
}
