import "package:material_ui/material_ui.dart";

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
  ({
    LibraryTimelineProjection paint,
    LibraryTimelineProjection input,
    double value,
  })?
  _gesture;

  @override
  void initState() {
    super.initState();
    _retainedFrame = widget.frame;
  }

  @override
  void didUpdateWidget(covariant LibraryTimeRailPresentation oldWidget) {
    super.didUpdateWidget(oldWidget);
    // A query change retires input while keeping paint as the next motion's origin.
    if (oldWidget.scrollController != widget.scrollController ||
        widget.timeline == null ||
        widget.timeline?.buckets.isEmpty == true) {
      _retainedFrame = null;
    }
    final incoming = widget.frame;
    final retained = _retainedFrame;
    // A same-query refresh may expose estimates before its full layout is ready.
    // Keep the complete frame until its replacement has the same precision.
    final awaitsCompleteLayout =
        retained?.projection.queryId == incoming?.projection.queryId &&
        retained?.projection.layoutMetrics != null &&
        incoming?.projection.layoutMetrics == null;
    if (incoming != null && !awaitsCompleteLayout) {
      _retainedFrame = incoming;
    } else if (incoming != null &&
        retained != null &&
        incoming.projection.queryId == retained.projection.queryId &&
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
    if (oldWidget.timeline?.queryId != widget.timeline?.queryId ||
        oldWidget.layoutShape != widget.layoutShape ||
        oldWidget.scrollController != widget.scrollController ||
        widget.navigationProjection == null) {
      _gesture = null;
      final cancelInteraction = oldWidget.onInteractionCancelled;
      WidgetsBinding.instance.addPostFrameCallback((_) {
        if (mounted) cancelInteraction();
      });
    }
  }

  bool _ownsInput(LibraryTimelineProjection projection) =>
      mounted &&
      identical(widget.navigationProjection, projection) &&
      projection.queryId == widget.timeline?.queryId &&
      projection.queryId == _retainedFrame?.projection.queryId;

  @override
  Widget build(BuildContext context) {
    final retained = _retainedFrame;
    final gesture = _gesture;
    final frame = gesture == null
        ? retained
        : LibraryTimeRailFrame(
            projection: gesture.paint,
            value: gesture.value,
            origin: LibraryTimeRailPositionOrigin.interaction,
          );
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
    final currentNavigation = widget.navigationProjection;
    final navigation =
        currentNavigation != null && _ownsInput(currentNavigation)
        ? currentNavigation
        : null;
    // Keep pointer paint in the gesture's scale; a layout round-trip can snap it
    // to a different row. Navigation still resolves through the current query.
    return AnnotatedTimeRail(
      interactionKey: ValueKey((
        widget.timeline?.queryId,
        widget.layoutShape,
        widget.scrollController,
        _gesture?.input ?? navigation,
      )),
      key: const Key("library-time-rail"),
      value: frame.value,
      maximumScrollOffset: frame.projection.maximumOffset,
      buckets: frame.projection.railBuckets,
      projection: frame.projection.projection,
      onChangeStart: navigation != null
          ? (value) {
              if (_ownsInput(navigation)) {
                setState(
                  () => _gesture = (
                    paint: frame.projection,
                    input: navigation,
                    value: value,
                  ),
                );
                widget.onChangeStart(
                  navigation.valueFromProjection(frame.projection, value),
                );
              }
            }
          : null,
      onChanged: navigation != null && widget.onChanged != null
          ? (value) {
              if (_ownsInput(navigation)) {
                if (_gesture case final gesture?) {
                  setState(
                    () => _gesture = (
                      paint: gesture.paint,
                      input: gesture.input,
                      value: value,
                    ),
                  );
                }
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
                setState(() => _gesture = null);
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
