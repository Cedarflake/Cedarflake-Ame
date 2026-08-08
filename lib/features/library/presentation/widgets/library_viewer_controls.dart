import "package:flutter/material.dart";

import "../library_strings.dart";

class LibraryViewerTopBar extends StatelessWidget {
  const LibraryViewerTopBar({
    required this.relativePath,
    required this.onBack,
    required this.onInformation,
    required this.onCopyPath,
    required this.onRevealFile,
    this.positionLabel,
    super.key,
  });

  final String relativePath;
  final String? positionLabel;
  final VoidCallback onBack;
  final VoidCallback onInformation;
  final VoidCallback onCopyPath;
  final VoidCallback onRevealFile;

  @override
  Widget build(BuildContext context) {
    return SizedBox(
      height: 72,
      child: Padding(
        padding: const EdgeInsets.symmetric(horizontal: 16),
        child: Row(
          children: [
            IconButton(
              tooltip: LibraryStrings.backToLibrary,
              onPressed: onBack,
              icon: const Icon(Icons.arrow_back),
            ),
            const SizedBox(width: 8),
            Expanded(
              child: Text(
                relativePath,
                maxLines: 1,
                overflow: TextOverflow.ellipsis,
                style: Theme.of(context).textTheme.titleMedium,
              ),
            ),
            if (positionLabel case final label?) ...[
              Text(
                label,
                style: Theme.of(context).textTheme.labelLarge?.copyWith(
                  color: Theme.of(context).colorScheme.onSurfaceVariant,
                ),
              ),
              const SizedBox(width: 8),
            ],
            IconButton(
              tooltip: LibraryStrings.viewInformation,
              onPressed: onInformation,
              icon: const Icon(Icons.info_outline),
            ),
            MenuAnchor(
              menuChildren: [
                MenuItemButton(
                  leadingIcon: const Icon(Icons.content_copy_outlined),
                  onPressed: onCopyPath,
                  child: const Text(LibraryStrings.copyPath),
                ),
                MenuItemButton(
                  leadingIcon: const Icon(Icons.folder_open_outlined),
                  onPressed: onRevealFile,
                  child: const Text(LibraryStrings.openInExplorer),
                ),
              ],
              builder: (context, controller, child) => IconButton(
                tooltip: LibraryStrings.more,
                onPressed: () =>
                    controller.isOpen ? controller.close() : controller.open(),
                icon: const Icon(Icons.more_horiz),
              ),
            ),
          ],
        ),
      ),
    );
  }
}

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
      color: colorScheme.surfaceContainerHigh.withValues(alpha: 0.94),
      elevation: 2,
      borderRadius: BorderRadius.circular(28),
      child: SizedBox(
        height: 52,
        child: Row(
          mainAxisSize: MainAxisSize.min,
          children: [
            IconButton(
              tooltip: "缩小（Ctrl+-）",
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
              tooltip: "放大（Ctrl++）",
              onPressed: canZoomIn ? onZoomIn : null,
              icon: const Icon(Icons.add),
            ),
            const SizedBox(height: 28, child: VerticalDivider(width: 1)),
            IconButton(
              key: const Key("viewer-fit"),
              tooltip: "适合窗口（Ctrl+0）",
              onPressed: onFitToWindow,
              icon: const Icon(Icons.fit_screen_outlined),
            ),
            TextButton(
              key: const Key("viewer-actual-size"),
              onPressed: canShowActualSize ? onShowActualSize : null,
              child: const Text("1:1"),
            ),
            const SizedBox(width: 4),
          ],
        ),
      ),
    );
  }
}
