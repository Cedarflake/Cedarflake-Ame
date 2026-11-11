import "package:flutter/widgets.dart";

import "../../../../app/presentation/ame_motion.dart";
import "library_viewer_transition.dart";

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
    final active = widget.visible && LibraryViewerActivity.isActiveOf(context);
    return AmeStateTransition(
      active: widget.visible,
      motion: AmeMotion.fastSpatial,
      child: _wasOpened
          ? LibraryViewerActivity(
              isActive: active,
              child: TickerMode(
                enabled: active,
                child: ExcludeFocus(
                  excluding: !active,
                  child: ExcludeSemantics(
                    excluding: !active,
                    child: IgnorePointer(
                      ignoring: !active,
                      child: SizedBox(
                        height: LibraryViewerFilmstripSlot.height,
                        child: widget.child,
                      ),
                    ),
                  ),
                ),
              ),
            )
          : const SizedBox.shrink(),
      builder: (context, animation, child) => AnimatedBuilder(
        animation: animation,
        child: child,
        builder: (context, child) => Visibility(
          visible: widget.visible || animation.value > 0,
          maintainState: true,
          child: ClipRect(
            child: FractionalTranslation(
              translation: Offset(0, 1 - animation.value.clamp(0, 1)),
              child: child,
            ),
          ),
        ),
      ),
    );
  }
}
