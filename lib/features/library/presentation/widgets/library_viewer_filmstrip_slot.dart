import "package:flutter/widgets.dart";

/// Retains the opened strip until the viewer closes, without loading it eagerly.
class LibraryViewerFilmstripSlot extends StatefulWidget {
  const LibraryViewerFilmstripSlot({
    required this.visible,
    required this.child,
    super.key,
  });

  static const height = 104.0;

  final bool visible;
  final Widget child;

  @override
  State<LibraryViewerFilmstripSlot> createState() =>
      _LibraryViewerFilmstripSlotState();
}

class _LibraryViewerFilmstripSlotState
    extends State<LibraryViewerFilmstripSlot> {
  late bool _wasOpened = widget.visible;

  @override
  void didUpdateWidget(covariant LibraryViewerFilmstripSlot oldWidget) {
    super.didUpdateWidget(oldWidget);
    _wasOpened = _wasOpened || widget.visible;
  }

  @override
  Widget build(BuildContext context) {
    if (!_wasOpened) return const SizedBox.shrink();
    return Visibility(
      visible: widget.visible,
      maintainState: true,
      child: SizedBox(
        height: LibraryViewerFilmstripSlot.height,
        child: widget.child,
      ),
    );
  }
}
