import "package:flutter/gestures.dart";
import "package:flutter/material.dart";

import "annotated_time_rail.dart";

class LibraryGalleryScrollSurface extends StatefulWidget {
  const LibraryGalleryScrollSurface({
    required this.controller,
    required this.usePositionRail,
    required this.gallery,
    this.timeNavigation,
    this.scrollIdentity,
    super.key,
  });

  final ScrollController controller;
  final bool usePositionRail;
  final Widget gallery;
  final Widget? timeNavigation;
  final Object? scrollIdentity;

  @override
  State<LibraryGalleryScrollSurface> createState() =>
      _LibraryGalleryScrollSurfaceState();
}

class _LibraryGalleryScrollSurfaceState
    extends State<LibraryGalleryScrollSurface> {
  Drag? _drag;

  @override
  void didUpdateWidget(covariant LibraryGalleryScrollSurface oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (widget.scrollIdentity != oldWidget.scrollIdentity ||
        widget.controller != oldWidget.controller ||
        !widget.usePositionRail) {
      _cancelDrag();
    }
  }

  @override
  void dispose() {
    _cancelDrag();
    super.dispose();
  }

  void _cancelDrag() {
    final drag = _drag;
    _drag = null;
    drag?.cancel();
  }

  ScrollPosition? _positionFor(Object? identity) {
    if (!mounted ||
        !widget.usePositionRail ||
        widget.scrollIdentity != identity ||
        !widget.controller.hasClients) {
      return null;
    }
    final position = widget.controller.position;
    return position.hasContentDimensions ? position : null;
  }

  void _startSeek(Object? identity) {
    final position = _positionFor(identity);
    if (position == null) return;
    _cancelDrag();
    _drag = position.drag(DragStartDetails(), () => _drag = null);
  }

  void _seek(double value, Object? identity) {
    final position = _positionFor(identity);
    if (position == null) return;
    final offset =
        position.minScrollExtent +
        value.clamp(0.0, 1.0) *
            (position.maxScrollExtent - position.minScrollExtent);
    final isDiscrete = _drag == null;
    if (isDiscrete) _startSeek(identity);
    final delta = position.pixels - offset;
    _drag?.update(
      DragUpdateDetails(
        delta: Offset(0, delta),
        primaryDelta: delta,
        globalPosition: Offset.zero,
      ),
    );
    if (isDiscrete) _drag?.end(DragEndDetails(primaryVelocity: 0));
  }

  void _finishSeek(double value, Object? identity) {
    if (_positionFor(identity) == null) return;
    _seek(value, identity);
    _drag?.end(DragEndDetails(primaryVelocity: 0));
  }

  Widget _positionRail() => AnimatedBuilder(
    animation: widget.controller,
    builder: (context, child) {
      final identity = widget.scrollIdentity;
      final position = widget.controller.hasClients
          ? widget.controller.position
          : null;
      final extent = position?.hasContentDimensions == true
          ? position!.maxScrollExtent - position.minScrollExtent
          : 0.0;
      final value = extent > 0
          ? (position!.pixels - position.minScrollExtent) / extent
          : 0.0;
      return AnnotatedTimeRail(
        key: const Key("library-position-rail"),
        interactionKey: ValueKey((identity, widget.controller)),
        value: value.clamp(0.0, 1.0),
        maximumScrollOffset: extent,
        buckets: const [],
        onChangeStart: extent > 0 ? (_) => _startSeek(identity) : null,
        onChanged: extent > 0 ? (value) => _seek(value, identity) : null,
        onChangeEnd: extent > 0
            ? (value) => _finishSeek(value, identity)
            : null,
        onStep: extent > 0
            ? (direction) => _seek(value + direction * 50 / extent, identity)
            : null,
      );
    },
  );

  @override
  Widget build(BuildContext context) {
    return NotificationListener<ScrollMetricsNotification>(
      onNotification: (notification) {
        if (notification.depth == 0 && widget.usePositionRail) setState(() {});
        return false;
      },
      child: Row(
        children: [
          Expanded(child: widget.gallery),
          SizedBox(
            width: AnnotatedTimeRail.width,
            child: widget.usePositionRail
                ? _positionRail()
                : widget.timeNavigation,
          ),
        ],
      ),
    );
  }
}
