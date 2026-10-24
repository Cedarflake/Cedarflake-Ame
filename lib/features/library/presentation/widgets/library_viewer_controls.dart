import "package:flutter/material.dart";
import "package:material_symbols_icons/symbols.dart";

import "../../../../app/presentation/ame_overlay_semantics.dart";
import "../../../../app/window/ame_window_chrome.dart";
import "../library_strings.dart";
import "library_path_text.dart";

class LibraryViewerTopBar extends StatelessWidget {
  const LibraryViewerTopBar({
    required this.displayPath,
    required this.onBack,
    this.organizationActions,
    super.key,
  });

  final String displayPath;
  final VoidCallback onBack;
  final Widget? organizationActions;

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
            AmeTooltip(
              message: "${LibraryStrings.backToLibrary}（Esc）",
              child: IconButton(
                key: const Key("viewer-back-button"),
                onPressed: onBack,
                icon: const Icon(Symbols.arrow_back_rounded),
              ),
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
                    const SizedBox(width: 8),
                  ],
                ),
              ),
            ),
            if (organizationActions case final actions?) ...[
              actions,
              const SizedBox(width: 16),
            ],
            const AmeWindowCaptionControls(height: 64),
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
      child: AmeTooltip(
        message: isPrevious ? "上一张（←）" : "下一张（→）",
        child: IconButton.filledTonal(
          key: Key(isPrevious ? "viewer-previous" : "viewer-next"),
          onPressed: onPressed,
          icon: Icon(
            isPrevious
                ? Symbols.chevron_left_rounded
                : Symbols.chevron_right_rounded,
          ),
        ),
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
