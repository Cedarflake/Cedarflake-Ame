import "dart:async";

import "package:flutter/material.dart";
import "package:material_symbols_icons/symbols.dart";

import "../../../../app/presentation/ame_overlay_semantics.dart";
import "../library_strings.dart";

/// Keeps end buttons outside the native thumb and track hit regions.
class LibraryViewerFilmstripScrollbar extends StatefulWidget {
  const LibraryViewerFilmstripScrollbar({
    required this.controller,
    required this.step,
    required this.child,
    super.key,
  });

  static const controlExtent = 24.0;

  final ScrollController controller;
  final double step;
  final Widget child;

  @override
  State<LibraryViewerFilmstripScrollbar> createState() =>
      _LibraryViewerFilmstripScrollbarState();
}

class _LibraryViewerFilmstripScrollbarState
    extends State<LibraryViewerFilmstripScrollbar> {
  void _scroll(int direction) {
    final controller = widget.controller;
    if (!controller.hasClients) return;
    final position = controller.position;
    final target = (position.pixels + direction * widget.step).clamp(
      position.minScrollExtent,
      position.maxScrollExtent,
    );
    if (MediaQuery.disableAnimationsOf(context)) {
      controller.jumpTo(target);
    } else {
      unawaited(
        controller.animateTo(
          target,
          duration: const Duration(milliseconds: 120),
          curve: Curves.easeOutCubic,
        ),
      );
    }
  }

  @override
  Widget build(BuildContext context) {
    const extent = LibraryViewerFilmstripScrollbar.controlExtent;
    return NotificationListener<ScrollMetricsNotification>(
      onNotification: (_) {
        setState(() {});
        return false;
      },
      child: Stack(
        children: [
          ScrollbarTheme(
            data: ScrollbarTheme.of(context).copyWith(
              mainAxisMargin: extent,
              crossAxisMargin: 8,
              thickness: const WidgetStatePropertyAll(8),
            ),
            child: Scrollbar(
              controller: widget.controller,
              thumbVisibility: true,
              child: widget.child,
            ),
          ),
          Positioned(
            left: 0,
            right: 0,
            bottom: 0,
            height: extent,
            child: ListenableBuilder(
              listenable: widget.controller,
              builder: (context, _) {
                final controller = widget.controller;
                final position = controller.hasClients
                    ? controller.position
                    : null;
                final ready = position?.hasContentDimensions ?? false;
                final style = IconButton.styleFrom(
                  padding: EdgeInsets.zero,
                  minimumSize: const Size.square(extent),
                  maximumSize: const Size.square(extent),
                  tapTargetSize: MaterialTapTargetSize.shrinkWrap,
                  visualDensity: VisualDensity.standard,
                );
                return Row(
                  children: [
                    AmeTooltip(
                      message: LibraryStrings.scrollFilmstripLeft,
                      child: IconButton(
                        key: const Key("viewer-filmstrip-scroll-left"),
                        style: style,
                        icon: const Icon(Symbols.arrow_left, size: 20, fill: 1),
                        onPressed: ready && position!.extentBefore > 0
                            ? () => _scroll(-1)
                            : null,
                      ),
                    ),
                    const Spacer(),
                    AmeTooltip(
                      message: LibraryStrings.scrollFilmstripRight,
                      child: IconButton(
                        key: const Key("viewer-filmstrip-scroll-right"),
                        style: style,
                        icon: const Icon(
                          Symbols.arrow_right,
                          size: 20,
                          fill: 1,
                        ),
                        onPressed: ready && position!.extentAfter > 0
                            ? () => _scroll(1)
                            : null,
                      ),
                    ),
                  ],
                );
              },
            ),
          ),
        ],
      ),
    );
  }
}
