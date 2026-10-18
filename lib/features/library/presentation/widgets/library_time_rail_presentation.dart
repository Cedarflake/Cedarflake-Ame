import "package:flutter/material.dart";

import "../../domain/library_models.dart";
import "../gallery_view_options.dart";
import "annotated_time_rail.dart";
import "library_timeline_projection.dart";

enum LibraryTimeRailPositionOrigin { gallery, interaction }

/// A painted rail has no catalog or layout authority of its own.
class LibraryTimeRailFrame {
  const LibraryTimeRailFrame({
    required this.projection,
    required this.value,
    this.origin = LibraryTimeRailPositionOrigin.gallery,
  });

  final LibraryTimelineProjection projection;
  final double value;
  final LibraryTimeRailPositionOrigin origin;
}

class LibraryTimeRailPresentation extends StatefulWidget {
  const LibraryTimeRailPresentation({
    required this.timeline,
    required this.layoutShape,
    required this.scrollController,
    required this.frame,
    required this.navigationProjection,
    required this.isLoading,
    required this.onChangeStart,
    required this.onChanged,
    required this.onChangeEnd,
    required this.onStep,
    required this.onInteractionCancelled,
    super.key,
  });

  final LibraryTimeline? timeline;
  final GalleryLayoutShape layoutShape;
  final ScrollController scrollController;
  final LibraryTimeRailFrame? frame;
  final LibraryTimelineProjection? navigationProjection;
  final bool isLoading;
  final ValueChanged<double> onChangeStart;
  final ValueChanged<double>? onChanged;
  final ValueChanged<double>? onChangeEnd;
  final ValueChanged<int>? onStep;
  final VoidCallback onInteractionCancelled;

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
        oldWidget.scrollController != widget.scrollController ||
        widget.timeline == null ||
        widget.timeline!.buckets.isEmpty) {
      _retainedFrame = null;
    }
    final incoming = widget.frame;
    final retained = _retainedFrame;
    // A refresh may expose window estimates before its full layout is ready.
    // Keep the complete frame until its replacement has the same precision.
    final awaitsCompleteLayout =
        retained?.projection.layoutMetrics != null &&
        incoming?.projection.layoutMetrics == null;
    if (incoming != null && !awaitsCompleteLayout) {
      _retainedFrame = incoming;
    } else if (incoming != null &&
        retained != null &&
        incoming.origin == LibraryTimeRailPositionOrigin.interaction) {
      // Provisional window offsets cannot move retained paint, but user intent can.
      _retainedFrame = LibraryTimeRailFrame(
        projection: retained.projection,
        value: retained.projection.valueFromProjection(
          incoming.projection,
          incoming.value,
        ),
        origin: LibraryTimeRailPositionOrigin.interaction,
      );
    }
    if (!identical(
      oldWidget.navigationProjection,
      widget.navigationProjection,
    )) {
      final cancelInteraction = oldWidget.onInteractionCancelled;
      WidgetsBinding.instance.addPostFrameCallback((_) {
        if (mounted) cancelInteraction();
      });
    }
  }

  bool _ownsInput(LibraryTimelineProjection projection) =>
      mounted && identical(widget.navigationProjection, projection);

  @override
  Widget build(BuildContext context) {
    final frame = _retainedFrame;
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
    final navigation = widget.navigationProjection;
    // A new gesture uses current date evidence while the painted ticks stay
    // stable. Replacing its projection still retires the old Material pointer.
    return AnnotatedTimeRail(
      interactionKey: ValueKey((
        widget.timeline?.revision,
        widget.timeline?.queryId,
        widget.layoutShape,
        widget.scrollController,
        navigation,
      )),
      key: const Key("library-time-rail"),
      value: frame.value,
      maximumScrollOffset: frame.projection.maximumOffset,
      buckets: frame.projection.railBuckets,
      projection: frame.projection.projection,
      onChangeStart: navigation != null
          ? (value) {
              if (_ownsInput(navigation)) {
                widget.onChangeStart(
                  navigation.valueFromProjection(frame.projection, value),
                );
              }
            }
          : null,
      onChanged: navigation != null && widget.onChanged != null
          ? (value) {
              if (_ownsInput(navigation)) {
                widget.onChanged?.call(
                  navigation.valueFromProjection(frame.projection, value),
                );
              }
            }
          : null,
      onChangeEnd: navigation != null && widget.onChangeEnd != null
          ? (value) {
              if (_ownsInput(navigation)) {
                widget.onChangeEnd?.call(
                  navigation.valueFromProjection(frame.projection, value),
                );
              }
            }
          : null,
      onStep: navigation != null && widget.onStep != null
          ? (direction) {
              if (_ownsInput(navigation)) widget.onStep?.call(direction);
            }
          : null,
    );
  }
}
