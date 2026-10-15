import "package:flutter/material.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";

import "../../../../app/window/ame_window_fullscreen.dart";

/// Native full screen belongs to the viewer, not to the hidden gallery's layout.
class LibraryViewerGalleryViewport extends ConsumerStatefulWidget {
  const LibraryViewerGalleryViewport({
    required this.isViewerOpen,
    required this.child,
    super.key,
  });

  final bool isViewerOpen;
  final Widget child;

  @override
  ConsumerState<LibraryViewerGalleryViewport> createState() =>
      _LibraryViewerGalleryViewportState();
}

class _LibraryViewerGalleryViewportState
    extends ConsumerState<LibraryViewerGalleryViewport> {
  Size? _normalSize;

  @override
  Widget build(BuildContext context) {
    final fullscreen = ref.watch(ameWindowFullscreenProvider);
    if (fullscreen == null) return widget.child;
    return ListenableBuilder(
      listenable: fullscreen,
      builder: (context, _) => LayoutBuilder(
        builder: (context, constraints) {
          final preserve =
              widget.isViewerOpen &&
              (fullscreen.isFullscreen || fullscreen.isChanging);
          if (!preserve || _normalSize == null) {
            _normalSize = constraints.biggest;
          }
          final size = _normalSize!;
          return OverflowBox(
            alignment: Alignment.topLeft,
            minWidth: size.width,
            maxWidth: size.width,
            minHeight: size.height,
            maxHeight: size.height,
            child: SizedBox.fromSize(size: size, child: widget.child),
          );
        },
      ),
    );
  }
}
