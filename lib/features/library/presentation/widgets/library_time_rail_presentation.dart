import "package:flutter/material.dart";

import "../../domain/library_models.dart";
import "../gallery_view_options.dart";
import "annotated_time_rail.dart";
import "library_timeline_projection.dart";

/// A painted rail has no catalog or layout authority of its own.
class LibraryTimeRailFrame {
  const LibraryTimeRailFrame({required this.projection, required this.value});

  final LibraryTimelineProjection projection;
  final double value;
}

class LibraryTimeRailPresentation extends StatefulWidget {
  const LibraryTimeRailPresentation({
    required this.timeline,
    required this.layoutShape,
    required this.scrollController,
    required this.frame,
    required this.isLoading,
    required this.onChangeStart,
    required this.onChanged,
    required this.onChangeEnd,
    required this.onStep,
    super.key,
  });

  final LibraryTimeline? timeline;
  final GalleryLayoutShape layoutShape;
  final ScrollController scrollController;
  final LibraryTimeRailFrame? frame;
  final bool isLoading;
  final ValueChanged<double> onChangeStart;
  final ValueChanged<double>? onChanged;
  final ValueChanged<double>? onChangeEnd;
  final ValueChanged<int>? onStep;

  @override
  State<LibraryTimeRailPresentation> createState() =>
      _LibraryTimeRailPresentationState();
}

class _LibraryTimeRailPresentationState
    extends State<LibraryTimeRailPresentation> {
  LibraryTimeRailFrame? _retainedFrame;

  @override
  void initState() {
    super.initState();
    _retainedFrame = widget.frame;
  }

  @override
  void didUpdateWidget(covariant LibraryTimeRailPresentation oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.timeline?.queryId != widget.timeline?.queryId ||
        oldWidget.layoutShape != widget.layoutShape ||
        oldWidget.scrollController != widget.scrollController ||
        widget.timeline == null ||
        widget.timeline!.buckets.isEmpty) {
      _retainedFrame = null;
    }
    _retainedFrame = widget.frame ?? _retainedFrame;
  }

  bool _ownsInput(LibraryTimeRailFrame frame) =>
      mounted && identical(widget.frame, frame);

  @override
  Widget build(BuildContext context) {
    final frame = widget.frame ?? _retainedFrame;
    if (frame == null) {
      return SizedBox(
        width: AnnotatedTimeRail.width,
        child: widget.isLoading && widget.timeline == null
            ? const Center(
                child: SizedBox.square(
                  dimension: 24,
                  child: CircularProgressIndicator(strokeWidth: 3),
                ),
              )
            : null,
      );
    }
    final hasCurrentGeometry = identical(widget.frame, frame);
    // Material retains an active pointer across disabled/enabled updates. Retire
    // that interaction when its catalog/layout owner changes, even if its last
    // frame remains visible while replacement geometry is pending.
    return KeyedSubtree(
      key: ValueKey((
        widget.timeline?.revision,
        widget.timeline?.queryId,
        widget.layoutShape,
        widget.scrollController,
      )),
      child: AnnotatedTimeRail(
        key: const Key("library-time-rail"),
        value: frame.value,
        maximumScrollOffset: frame.projection.maximumOffset,
        buckets: frame.projection.railBuckets,
        projection: frame.projection.projection,
        onChangeStart: hasCurrentGeometry
            ? (value) {
                if (_ownsInput(frame)) widget.onChangeStart(value);
              }
            : null,
        onChanged: hasCurrentGeometry
            ? (value) {
                if (_ownsInput(frame)) widget.onChanged?.call(value);
              }
            : null,
        onChangeEnd: hasCurrentGeometry
            ? (value) {
                if (_ownsInput(frame)) widget.onChangeEnd?.call(value);
              }
            : null,
        onStep: hasCurrentGeometry
            ? (direction) {
                if (_ownsInput(frame)) widget.onStep?.call(direction);
              }
            : null,
      ),
    );
  }
}
