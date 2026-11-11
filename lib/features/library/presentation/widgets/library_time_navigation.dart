import "dart:async";

import "package:material_ui/material_ui.dart";
import "package:flutter/rendering.dart";
import "package:flutter/scheduler.dart";

import "../../../../app/presentation/ame_scroll_motion.dart";
import "../../domain/library_models.dart";
import "../coordination/library_time_seek_scheduler.dart";
import "../gallery_view_options.dart";
import "library_gallery_layout.dart";
import "library_time_rail_presentation.dart";
import "library_time_seek_alignment.dart";
import "library_timeline_projection.dart";
import "library_timeline_projection_cache.dart";
import "library_virtual_gallery_geometry.dart";

typedef LibraryTimelineSeekCallback =
    Future<bool> Function(LibraryTimeBucket bucket, int itemOffset);

class LibraryTimeNavigation extends StatefulWidget {
  const LibraryTimeNavigation({
    required this.isLoading,
    required this.scrollController,
    required this.layoutMetrics,
    required this.timeline,
    required this.layoutShape,
    required this.windowStartItemOffset,
    required this.loadedItemCount,
    required this.onSeek,
    this.onBeginNavigation,
    this.onCancelSeek,
    this.onPrefetch,
    this.virtualGeometry,
    this.resolvedTimeAnchor,
    super.key,
  });

  final bool isLoading;
  final ScrollController scrollController;
  final LibraryGalleryLayoutMetrics? layoutMetrics;
  final LibraryTimeline? timeline;
  final GalleryLayoutShape layoutShape;
  final LibraryVirtualGalleryGeometry? virtualGeometry;
  final LibraryTimeAnchor? resolvedTimeAnchor;
  final int windowStartItemOffset;
  final int loadedItemCount;
  final LibraryTimelineSeekCallback onSeek;
  final VoidCallback? onBeginNavigation;
  final VoidCallback? onCancelSeek;
  final LibraryTimelineSeekCallback? onPrefetch;

  @override
  State<LibraryTimeNavigation> createState() => _LibraryTimeNavigationState();
}

class _LibraryTimeNavigationState extends State<LibraryTimeNavigation> {
  static const Duration _galleryScrollSettleDelay = Duration(milliseconds: 180);

  double? _interactiveValue;
  double? _pendingFrameValue;
  bool _isDragging = false;
  bool _hasUserScrollIntent = false;
  bool _isFrameScheduled = false;
  late final _seeks = LibraryTimeSeekScheduler(_startSeek);
  LibraryTimeSeekAlignment? _navigationAlignment;
  int _timelineGeneration = 0;
  Timer? _galleryScrollSettleTimer;
  Timer? _userScrollIntentTimer;
  double? _pendingGalleryScrollValue;
  final _projectionCache = LibraryTimelineProjectionCache();
  LibraryGalleryLayoutMetrics? _stableLayoutMetrics;
  LibraryVirtualGalleryGeometry? _stableVirtualGeometry;
  var _stableWindowStartItemOffset = 0;
  var _stableLoadedItemCount = 0;

  bool get _isSeeking => _seeks.isRunning;

  @override
  void initState() {
    super.initState();
    _rememberStableLayout();
    widget.scrollController.addListener(_handleGalleryScroll);
  }

  @override
  void didUpdateWidget(covariant LibraryTimeNavigation oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.scrollController != widget.scrollController) {
      oldWidget.scrollController.removeListener(_handleGalleryScroll);
      widget.scrollController.addListener(_handleGalleryScroll);
    }
    final didChangeContext =
        oldWidget.timeline?.queryId != widget.timeline?.queryId ||
        oldWidget.scrollController != widget.scrollController ||
        oldWidget.layoutShape != widget.layoutShape;
    if (didChangeContext) {
      _timelineGeneration += 1;
      _seeks.cancel();
      _navigationAlignment = null;
      _galleryScrollSettleTimer?.cancel();
      _userScrollIntentTimer?.cancel();
      _interactiveValue = null;
      _pendingFrameValue = null;
      _pendingGalleryScrollValue = null;
      _isDragging = false;
      _hasUserScrollIntent = false;
      _isFrameScheduled = false;
      _projectionCache.clear();
      _stableLayoutMetrics = null;
      _stableVirtualGeometry = null;
      _stableWindowStartItemOffset = 0;
      _stableLoadedItemCount = 0;
    } else if (oldWidget.timeline?.revision != widget.timeline?.revision) {
      final previousProjection = _projectionCache.current;
      _timelineGeneration++;
      _isFrameScheduled = false;
      _cancelGalleryScrollSettle();
      _projectionCache.clear();
      _stableLayoutMetrics = null;
      _stableVirtualGeometry = null;
      final currentProjection = _projectionForCurrentTimeline();
      if (previousProjection != null && currentProjection != null) {
        if (_interactiveValue case final value?) {
          _interactiveValue = currentProjection.valueFromProjection(
            previousProjection,
            value,
          );
        }
        if (_pendingFrameValue case final value?) {
          _pendingFrameValue = currentProjection.valueFromProjection(
            previousProjection,
            value,
          );
          _scheduleNavigationFrame(currentProjection);
        }
      }
    }
    _rememberStableLayout();
  }

  @override
  void dispose() {
    _navigationAlignment = null;
    _timelineGeneration += 1;
    _seeks.dispose();
    _galleryScrollSettleTimer?.cancel();
    _userScrollIntentTimer?.cancel();
    widget.scrollController.removeListener(_handleGalleryScroll);
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    _rememberStableLayout();
    final metrics = widget.layoutMetrics ?? _stableLayoutMetrics;
    return AnimatedBuilder(
      animation: widget.scrollController,
      builder: (context, child) {
        final globalProjection = _projectionForCurrentTimeline();
        final position = widget.scrollController.hasClients
            ? widget.scrollController.position
            : null;
        LibraryTimeRailFrame? frame;
        final interactiveValue =
            _isDragging || widget.layoutMetrics?.isQueryWide != true
            ? _interactiveValue
            : null;
        if (globalProjection != null && interactiveValue != null) {
          frame = LibraryTimeRailFrame(
            projection: globalProjection,
            value: interactiveValue,
            origin: LibraryTimeRailPositionOrigin.interaction,
          );
        } else if (globalProjection != null &&
            metrics != null &&
            metrics.dateAnchors.isNotEmpty) {
          frame = LibraryTimeRailFrame(
            projection: globalProjection,
            origin: _hasUserScrollIntent
                ? LibraryTimeRailPositionOrigin.interaction
                : LibraryTimeRailPositionOrigin.gallery,
            value: _valueFromGallery(
              globalProjection,
              position,
              metrics,
              geometry: widget.layoutMetrics == null
                  ? _stableVirtualGeometry
                  : widget.virtualGeometry,
              windowStartItemOffset: widget.layoutMetrics == null
                  ? _stableWindowStartItemOffset
                  : widget.windowStartItemOffset,
              loadedItemCount: widget.layoutMetrics == null
                  ? _stableLoadedItemCount
                  : widget.loadedItemCount,
            ),
          );
        }
        final interactionGeneration = _seeks.generation;
        return LibraryTimeRailPresentation(
          timeline: widget.timeline,
          layoutShape: widget.layoutShape,
          scrollController: widget.scrollController,
          frame: frame,
          navigationProjection: globalProjection,
          isLoading: widget.isLoading,
          onChangeStart: _beginInteraction,
          onChanged: globalProjection == null
              ? null
              : (value) => _handleChanged(globalProjection, value),
          onChangeEnd: globalProjection == null
              ? null
              : (value) => _finishInteraction(globalProjection, value),
          onStep: metrics == null
              ? null
              : (direction) => _moveOneRow(metrics, direction),
          onInteractionCancelled: () {
            // Retire an unfinished gesture; a released date seek still owns
            // its alignment when the resulting catalog layout is published.
            if (mounted &&
                _isDragging &&
                interactionGeneration == _seeks.generation) {
              _cancelTimelineIntent();
            }
          },
        );
      },
    );
  }

  double _valueFromGallery(
    LibraryTimelineProjection globalProjection,
    ScrollPosition? position,
    LibraryGalleryLayoutMetrics metrics, {
    required LibraryVirtualGalleryGeometry? geometry,
    required int windowStartItemOffset,
    required int loadedItemCount,
  }) {
    if (position == null || !position.hasContentDimensions) {
      return globalProjection.valueForGlobalItemOffset(
        windowStartItemOffset.toDouble(),
      );
    }
    if (metrics.isQueryWide) {
      if (identical(globalProjection.layoutMetrics, metrics)) {
        return globalProjection.projection.offsetToValue(position.pixels);
      }
      final globalItemOffset =
          metrics.itemIndexBase +
          metrics.itemIndexForScrollOffset(position.pixels);
      return globalProjection.valueForGlobalItemOffset(
        globalItemOffset.toDouble(),
      );
    }
    if (loadedItemCount <= 1) {
      return globalProjection.valueForGlobalItemOffset(
        windowStartItemOffset.toDouble(),
      );
    }
    if (geometry == null) {
      final localItemOffset = metrics.itemIndexForScrollOffset(position.pixels);
      final globalItemOffset = metrics.itemIndexBase + localItemOffset;
      return globalProjection.valueForGlobalItemOffset(
        globalItemOffset.toDouble(),
      );
    }
    final localScrollOffset = position.pixels - geometry.leadingExtent;
    if (localScrollOffset >= 0 &&
        localScrollOffset < geometry.loadedContentExtent) {
      final localItemOffset = metrics.itemIndexForScrollOffset(
        localScrollOffset,
      );
      final globalItemOffset = windowStartItemOffset + localItemOffset;
      return globalProjection.valueForGlobalItemOffset(
        globalItemOffset.toDouble(),
      );
    }
    return geometry.valueForScrollOffset(position.pixels);
  }

  void _beginInteraction(double value) {
    _cancelGalleryScrollSettle();
    _cancelSeek();
    _userScrollIntentTimer?.cancel();
    _userScrollIntentTimer = null;
    _hasUserScrollIntent = false;
    setState(() {
      _isDragging = true;
      _interactiveValue = value;
    });
  }

  void _handleChanged(
    LibraryTimelineProjection globalProjection,
    double value,
  ) {
    widget.onBeginNavigation?.call();
    _pendingFrameValue = value;
    _scheduleNavigationFrame(globalProjection);
  }

  void _finishInteraction(
    LibraryTimelineProjection globalProjection,
    double value,
  ) {
    setState(() {
      _isDragging = false;
      _interactiveValue = value;
    });
    _pendingFrameValue = null;
    if (!_isTargetLoaded(globalProjection, value)) {
      if (_hasTargetGeometry(globalProjection, value)) {
        _moveGalleryToValue(globalProjection, value);
      }
      _queueSeek(globalProjection, value, immediate: true);
    } else {
      _cancelSeek();
      _moveGalleryToValue(globalProjection, value);
      setState(() => _interactiveValue = null);
    }
  }

  void _scheduleNavigationFrame(LibraryTimelineProjection globalProjection) {
    if (_isFrameScheduled) {
      return;
    }
    _isFrameScheduled = true;
    final generation = _timelineGeneration;
    SchedulerBinding.instance.scheduleFrameCallback((_) {
      if (!mounted || generation != _timelineGeneration) {
        return;
      }
      _isFrameScheduled = false;
      final value = _pendingFrameValue;
      _pendingFrameValue = null;
      if (value == null) {
        return;
      }
      if (_interactiveValue != value) {
        setState(() => _interactiveValue = value);
      }
      final projection = _projectionForCurrentTimeline() ?? globalProjection;
      final isTargetLoaded = _isTargetLoaded(projection, value);
      if (isTargetLoaded) _cancelSeek();
      if (isTargetLoaded || _hasTargetGeometry(projection, value)) {
        _moveGalleryToValue(projection, value);
      }
      if (!isTargetLoaded) {
        _queueSeek(projection, value);
      }
    });
  }

  void _queueSeek(
    LibraryTimelineProjection globalProjection,
    double value, {
    bool immediate = false,
    LibraryTimeSeekIntent intent = LibraryTimeSeekIntent.navigation,
  }) {
    if (intent == LibraryTimeSeekIntent.navigation) {
      _cancelGalleryScrollSettle();
    }
    _seeks.schedule(
      LibraryTimeSeekRequest(
        projection: globalProjection,
        value: value,
        intent: intent,
      ),
      immediate: immediate,
    );
  }

  Future<void> _startSeek(
    LibraryTimeSeekRequest request,
    int seekIntentGeneration,
  ) async {
    if (!mounted) {
      return;
    }
    final generation = _timelineGeneration;
    final globalProjection =
        _projectionForCurrentTimeline() ?? request.projection;
    final value = globalProjection.valueFromProjection(
      request.projection,
      request.value,
    );
    final intent = request.intent;
    if (_isTargetLoaded(globalProjection, value)) {
      if (intent == LibraryTimeSeekIntent.navigation) {
        _moveGalleryToValue(globalProjection, value);
        if (!_isDragging) {
          setState(() => _interactiveValue = null);
        }
      }
      return;
    }
    final target = _detailLoadTarget(globalProjection, value);
    final alignment =
        intent == LibraryTimeSeekIntent.navigation && widget.timeline != null
        ? LibraryTimeSeekAlignment(
            generation: seekIntentGeneration,
            timeline: widget.timeline!,
            target: globalProjection.targetForValue(value),
            positionedLayout:
                identical(
                      globalProjection.layoutMetrics,
                      widget.layoutMetrics,
                    ) &&
                    widget.scrollController.hasClients &&
                    widget.scrollController.position.hasContentDimensions
                ? globalProjection.layoutMetrics
                : null,
          )
        : null;
    if (alignment != null) {
      _navigationAlignment = alignment;
    }
    final seek = intent == LibraryTimeSeekIntent.prefetch
        ? widget.onPrefetch ?? widget.onSeek
        : widget.onSeek;
    try {
      final didSeek = await seek(target.bucket, target.itemOffset);
      if (!_canCompleteSeek(generation, seekIntentGeneration, alignment)) {
        return;
      }
      if (didSeek && alignment != null) {
        await _alignLoadedSeekTarget(alignment);
      }
      if (!_canCompleteSeek(generation, seekIntentGeneration, alignment)) {
        return;
      }
    } finally {
      if (identical(_navigationAlignment, alignment)) {
        _navigationAlignment = null;
      }
    }
    if (!_isDragging && _pendingFrameValue == null && !_seeks.hasPending) {
      setState(() => _interactiveValue = null);
    }
  }

  bool _canCompleteSeek(
    int generation,
    int seekIntentGeneration,
    LibraryTimeSeekAlignment? alignment,
  ) {
    if (!mounted || !_seeks.isCurrent(seekIntentGeneration)) {
      return false;
    }
    if (alignment != null) {
      return identical(_navigationAlignment, alignment);
    }
    return generation == _timelineGeneration;
  }

  Future<void> _alignLoadedSeekTarget(
    LibraryTimeSeekAlignment alignment,
  ) async {
    for (var attempt = 0; attempt < 8; attempt++) {
      WidgetsBinding.instance.scheduleFrame();
      await WidgetsBinding.instance.endOfFrame;
      if (!mounted ||
          !identical(_navigationAlignment, alignment) ||
          !_seeks.isCurrent(alignment.generation)) {
        return;
      }
      final timeline = widget.timeline;
      final projection = _projectionForCurrentTimeline();
      if (timeline == null || projection == null) {
        return;
      }
      final offset = alignment.currentItemOffset(
        timeline,
        widget.resolvedTimeAnchor,
      );
      if (offset == null ||
          widget.layoutMetrics == null ||
          !_isGlobalTargetLoaded(offset)) {
        continue;
      }
      // The rail already positioned this complete layout, including the fraction within a row.
      if (alignment.retainsExactPosition(timeline, widget.layoutMetrics)) {
        return;
      }
      if (_isDragging || _seeks.hasPending) {
        return;
      }
      _moveGalleryToGlobalTarget(projection, offset);
      return;
    }
  }

  void _moveGalleryToValue(
    LibraryTimelineProjection globalProjection,
    double value,
  ) {
    if (identical(globalProjection.layoutMetrics, widget.layoutMetrics) &&
        globalProjection.layoutMetrics != null) {
      final position = widget.scrollController.hasClients
          ? widget.scrollController.position
          : null;
      if (position == null || !position.hasContentDimensions) {
        return;
      }
      final pixels = globalProjection.projection
          .valueToOffset(value)
          .clamp(position.minScrollExtent, position.maxScrollExtent)
          .toDouble();
      if ((position.pixels - pixels).abs() >= 0.5) {
        position.jumpTo(pixels);
      }
      return;
    }
    _moveGalleryToGlobalTarget(
      globalProjection,
      globalProjection.targetForValue(value).globalItemOffset,
      fallbackValue: value,
    );
  }

  void _moveGalleryToGlobalTarget(
    LibraryTimelineProjection globalProjection,
    int targetGlobalOffset, {
    double? fallbackValue,
  }) {
    final metrics = widget.layoutMetrics;
    final position = widget.scrollController.hasClients
        ? widget.scrollController.position
        : null;
    final geometry = widget.virtualGeometry;
    if (position == null || !position.hasContentDimensions) {
      return;
    }
    double? targetPixels;
    if (metrics != null &&
        metrics.containsGlobalItemIndex(targetGlobalOffset.toDouble())) {
      final localPixels = metrics.navigationOffsetForGlobalItemIndex(
        targetGlobalOffset,
      );
      if (localPixels != null) {
        targetPixels =
            (metrics.isQueryWide ? 0 : geometry?.leadingExtent ?? 0) +
            localPixels;
      }
    }
    targetPixels ??= geometry?.scrollOffsetForValue(
      fallbackValue ??
          globalProjection.valueForGlobalItemOffset(
            targetGlobalOffset.toDouble(),
          ),
    );
    if (targetPixels == null) {
      return;
    }
    final nextPixels = targetPixels
        .clamp(position.minScrollExtent, position.maxScrollExtent)
        .toDouble();
    if ((position.pixels - nextPixels).abs() < 0.5) {
      return;
    }
    position.jumpTo(nextPixels);
  }

  void _handleGalleryScroll() {
    if (!mounted || _isDragging) {
      return;
    }
    final projection = _projectionForCurrentTimeline();
    final metrics = widget.layoutMetrics;
    final position = widget.scrollController.hasClients
        ? widget.scrollController.position
        : null;
    if (position == null || !position.hasContentDimensions) {
      return;
    }
    if (_isUserControlledScroll(position)) {
      _recordUserScrollIntent();
    }
    if (projection == null || metrics == null) {
      return;
    }
    if (metrics.isQueryWide) {
      _cancelGalleryScrollSettle();
      return;
    }
    final value = _valueFromGallery(
      projection,
      position,
      metrics,
      geometry: widget.virtualGeometry,
      windowStartItemOffset: widget.windowStartItemOffset,
      loadedItemCount: widget.loadedItemCount,
    );
    if (_isTargetLoaded(projection, value)) {
      _cancelGalleryScrollSettle();
      if (!_isSeeking) {
        _seeks.clearPending();
      }
      return;
    }
    _pendingGalleryScrollValue = value;
    _galleryScrollSettleTimer?.cancel();
    final generation = _timelineGeneration;
    _galleryScrollSettleTimer = Timer(_galleryScrollSettleDelay, () {
      _galleryScrollSettleTimer = null;
      if (!mounted || _isDragging || generation != _timelineGeneration) {
        return;
      }
      final pendingValue = _pendingGalleryScrollValue;
      _pendingGalleryScrollValue = null;
      final currentProjection = _projectionForCurrentTimeline();
      if (pendingValue == null ||
          currentProjection == null ||
          _isTargetLoaded(currentProjection, pendingValue)) {
        return;
      }
      _queueSeek(
        currentProjection,
        pendingValue,
        immediate: true,
        intent: LibraryTimeSeekIntent.prefetch,
      );
    });
  }

  bool _isUserControlledScroll(ScrollPosition position) {
    final controller = widget.scrollController;
    return controller is AmeScrollController
        ? controller.isUserScrolling
        : position.userScrollDirection != ScrollDirection.idle;
  }

  void _recordUserScrollIntent() {
    if (!_hasUserScrollIntent) {
      _hasUserScrollIntent = true;
      _cancelTimelineIntent();
    }
    _userScrollIntentTimer?.cancel();
    _userScrollIntentTimer = Timer(_galleryScrollSettleDelay, () {
      _userScrollIntentTimer = null;
      _hasUserScrollIntent = false;
    });
  }

  void _cancelTimelineIntent() {
    _isDragging = false;
    _cancelSeek();
    _pendingFrameValue = null;
    if (_interactiveValue != null) {
      setState(() => _interactiveValue = null);
    }
  }

  void _cancelGalleryScrollSettle() {
    _galleryScrollSettleTimer?.cancel();
    _galleryScrollSettleTimer = null;
    _pendingGalleryScrollValue = null;
  }

  LibraryTimelineProjection? _projectionForCurrentTimeline() {
    final position = widget.scrollController.hasClients
        ? widget.scrollController.position
        : null;
    final hasScrollRange = position != null && position.hasContentDimensions;
    return _projectionCache.resolve(
      timeline: widget.timeline,
      metrics: widget.layoutMetrics ?? _stableLayoutMetrics,
      maximumScrollOffset: hasScrollRange ? position.maxScrollExtent : null,
      viewportExtent: hasScrollRange ? position.viewportDimension : null,
      useAspectRatioWeight:
          widget.layoutShape == GalleryLayoutShape.equalHeight,
    );
  }

  void _rememberStableLayout() {
    final metrics = widget.layoutMetrics;
    if (metrics == null) {
      return;
    }
    _stableLayoutMetrics = metrics;
    _stableVirtualGeometry = widget.virtualGeometry;
    _stableWindowStartItemOffset = widget.windowStartItemOffset;
    _stableLoadedItemCount = widget.loadedItemCount;
  }

  bool _isTargetLoaded(LibraryTimelineProjection projection, double value) {
    if (widget.layoutMetrics == null) {
      return false;
    }
    return _isGlobalTargetLoaded(
      projection.targetForValue(value).globalItemOffset,
    );
  }

  bool _isGlobalTargetLoaded(int targetGlobalOffset) {
    if (widget.loadedItemCount <= 0) {
      return false;
    }
    final metrics = widget.layoutMetrics ?? _stableLayoutMetrics;
    if (metrics?.isQueryWide ?? false) {
      final rowStart = metrics?.rowStartGlobalItemIndex(targetGlobalOffset);
      final rowEnd = metrics?.rowEndGlobalItemIndexExclusive(
        targetGlobalOffset,
      );
      if (rowStart == null || rowEnd == null) {
        return false;
      }
      return rowStart >= widget.windowStartItemOffset &&
          targetGlobalOffset >= widget.windowStartItemOffset &&
          rowEnd <= widget.windowStartItemOffset + widget.loadedItemCount;
    }
    return widget.virtualGeometry?.containsGlobalItemOffset(
          targetGlobalOffset.toDouble(),
        ) ??
        (targetGlobalOffset >= widget.windowStartItemOffset &&
            targetGlobalOffset <
                widget.windowStartItemOffset + widget.loadedItemCount);
  }

  LibraryTimelineTarget _detailLoadTarget(
    LibraryTimelineProjection projection,
    double value,
  ) {
    final target = projection.targetForValue(value);
    final metrics = widget.layoutMetrics ?? _stableLayoutMetrics;
    if (metrics == null || !metrics.isQueryWide) {
      return target;
    }
    final rowStart = metrics.rowStartGlobalItemIndex(target.globalItemOffset);
    return rowStart == null
        ? target
        : projection.targetForGlobalItemOffset(rowStart);
  }

  bool _hasTargetGeometry(LibraryTimelineProjection projection, double value) {
    final metrics = widget.layoutMetrics;
    if (metrics == null) {
      return false;
    }
    return metrics.containsGlobalItemIndex(
          projection.targetForValue(value).globalItemOffset.toDouble(),
        ) ||
        widget.virtualGeometry?.isVirtualized == true;
  }

  void _cancelSeek() {
    _navigationAlignment = null;
    _seeks.cancel();
    widget.onCancelSeek?.call();
  }

  void _moveOneRow(LibraryGalleryLayoutMetrics metrics, int direction) {
    if (!widget.scrollController.hasClients || metrics.photoRowHeight <= 0) {
      return;
    }
    _recordUserScrollIntent();
    _cancelTimelineIntent();
    final position = widget.scrollController.position;
    if (!position.hasContentDimensions) {
      return;
    }
    final target =
        position.pixels +
        (direction *
            (metrics.photoRowHeight + LibraryGalleryLayoutEntry.spacing));
    widget.scrollController.animateTo(
      target
          .clamp(position.minScrollExtent, position.maxScrollExtent)
          .toDouble(),
      duration: const Duration(milliseconds: 140),
      curve: Curves.easeOutCubic,
    );
  }
}
