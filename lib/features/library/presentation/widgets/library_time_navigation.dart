import "dart:async";

import "package:flutter/material.dart";
import "package:flutter/scheduler.dart";

import "../../domain/library_models.dart";
import "../gallery_view_options.dart";
import "annotated_time_rail.dart";
import "library_gallery_layout.dart";
import "library_timeline_projection.dart";
import "library_virtual_gallery_geometry.dart";

typedef LibraryTimelineSeekCallback =
    Future<bool> Function(LibraryTimeBucket bucket, int itemOffset);

enum _LibraryTimelineSeekIntent { navigation, prefetch }

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
    this.virtualGeometry,
    super.key,
  });

  final bool isLoading;
  final ScrollController scrollController;
  final LibraryGalleryLayoutMetrics? layoutMetrics;
  final LibraryTimeline? timeline;
  final GalleryLayoutShape layoutShape;
  final LibraryVirtualGalleryGeometry? virtualGeometry;
  final int windowStartItemOffset;
  final int loadedItemCount;
  final LibraryTimelineSeekCallback onSeek;

  @override
  State<LibraryTimeNavigation> createState() => _LibraryTimeNavigationState();
}

class _LibraryTimeNavigationState extends State<LibraryTimeNavigation> {
  static const Duration _windowSeekInterval = Duration(milliseconds: 120);
  static const Duration _galleryScrollSettleDelay = Duration(milliseconds: 180);

  double? _interactiveValue;
  double? _pendingFrameValue;
  double? _pendingSeekValue;
  _LibraryTimelineSeekIntent? _pendingSeekIntent;
  bool _isPendingSeekFinal = false;
  bool _isDragging = false;
  bool _isFrameScheduled = false;
  final Set<int> _activeSeekGenerations = <int>{};
  int _timelineGeneration = 0;
  int _seekIntentGeneration = 0;
  DateTime? _lastSeekStartedAt;
  Timer? _seekTimer;
  Timer? _galleryScrollSettleTimer;
  double? _pendingGalleryScrollValue;
  LibraryTimelineProjection? _cachedProjection;
  LibraryGalleryLayoutMetrics? _stableLayoutMetrics;
  LibraryVirtualGalleryGeometry? _stableVirtualGeometry;
  var _stableWindowStartItemOffset = 0;
  var _stableLoadedItemCount = 0;

  bool get _isSeeking => _activeSeekGenerations.isNotEmpty;

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
    final didChangeTimeline =
        oldWidget.timeline?.revision != widget.timeline?.revision ||
        oldWidget.timeline?.queryId != widget.timeline?.queryId ||
        oldWidget.layoutShape != widget.layoutShape;
    if (didChangeTimeline) {
      _timelineGeneration += 1;
      _seekIntentGeneration += 1;
      _seekTimer?.cancel();
      _galleryScrollSettleTimer?.cancel();
      _interactiveValue = null;
      _pendingFrameValue = null;
      _pendingSeekValue = null;
      _pendingSeekIntent = null;
      _pendingGalleryScrollValue = null;
      _isPendingSeekFinal = false;
      _isDragging = false;
      _isFrameScheduled = false;
      _activeSeekGenerations.clear();
      _lastSeekStartedAt = null;
      _cachedProjection = null;
      _stableLayoutMetrics = null;
      _stableVirtualGeometry = null;
      _stableWindowStartItemOffset = 0;
      _stableLoadedItemCount = 0;
    }
    _rememberStableLayout();
  }

  @override
  void dispose() {
    _timelineGeneration += 1;
    _seekIntentGeneration += 1;
    _seekTimer?.cancel();
    _galleryScrollSettleTimer?.cancel();
    _activeSeekGenerations.clear();
    widget.scrollController.removeListener(_handleGalleryScroll);
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final timeline = widget.timeline;
    if (widget.isLoading && timeline == null) {
      return const SizedBox(
        width: AnnotatedTimeRail.width,
        child: Center(
          child: SizedBox.square(
            dimension: 24,
            child: CircularProgressIndicator(strokeWidth: 3),
          ),
        ),
      );
    }
    _rememberStableLayout();
    final metrics = widget.layoutMetrics ?? _stableLayoutMetrics;
    if (timeline == null ||
        timeline.buckets.isEmpty ||
        metrics == null ||
        metrics.dateAnchors.isEmpty) {
      return const SizedBox(width: AnnotatedTimeRail.width);
    }
    final globalProjection = _projectionForCurrentTimeline();
    if (globalProjection == null) {
      return const SizedBox(width: AnnotatedTimeRail.width);
    }
    return AnimatedBuilder(
      animation: widget.scrollController,
      builder: (context, child) {
        final position = widget.scrollController.hasClients
            ? widget.scrollController.position
            : null;
        final derivedValue = _valueFromGallery(
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
        );
        return AnnotatedTimeRail(
          key: const Key("library-time-rail"),
          value: _interactiveValue ?? derivedValue,
          maximumScrollOffset: globalProjection.maximumOffset,
          buckets: globalProjection.railBuckets,
          projection: globalProjection.projection,
          onChangeStart: _beginInteraction,
          onChanged: (value) => _handleChanged(globalProjection, value),
          onChangeEnd: (value) => _finishInteraction(globalProjection, value),
          onStep: (direction) => _moveOneRow(metrics, direction),
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
    setState(() {
      _isDragging = true;
      _interactiveValue = value;
    });
  }

  void _handleChanged(
    LibraryTimelineProjection globalProjection,
    double value,
  ) {
    _seekIntentGeneration += 1;
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
    if (_isSeeking || !_isTargetLoaded(globalProjection, value)) {
      if (_hasTargetGeometry(globalProjection, value)) {
        _moveGalleryToValue(globalProjection, value);
      }
      _queueSeek(globalProjection, value, isFinal: true, immediate: true);
    } else {
      _moveGalleryToValue(globalProjection, value);
      _clearPendingSeek();
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
      final isTargetLoaded = _isTargetLoaded(globalProjection, value);
      if (isTargetLoaded || _hasTargetGeometry(globalProjection, value)) {
        _moveGalleryToValue(globalProjection, value);
      }
      if (_isDragging) {
        if (!isTargetLoaded &&
            widget.layoutShape == GalleryLayoutShape.equalHeight) {
          _queueSeek(globalProjection, value);
        }
        return;
      }
      if (_isSeeking) {
        _queueSeek(globalProjection, value);
        return;
      }
      if (isTargetLoaded) {
        _clearPendingSeek();
      } else {
        _queueSeek(globalProjection, value);
      }
    });
  }

  void _queueSeek(
    LibraryTimelineProjection globalProjection,
    double value, {
    bool isFinal = false,
    bool immediate = false,
    _LibraryTimelineSeekIntent intent = _LibraryTimelineSeekIntent.navigation,
  }) {
    if (intent == _LibraryTimelineSeekIntent.navigation) {
      _cancelGalleryScrollSettle();
    }
    _seekIntentGeneration += 1;
    _pendingSeekValue = value;
    _pendingSeekIntent = isFinal
        ? _LibraryTimelineSeekIntent.navigation
        : intent;
    _isPendingSeekFinal = _isPendingSeekFinal || isFinal;
    final lastStartedAt = _lastSeekStartedAt;
    final elapsed = lastStartedAt == null
        ? _windowSeekInterval
        : DateTime.now().difference(lastStartedAt);
    final delay = immediate || elapsed >= _windowSeekInterval
        ? Duration.zero
        : _windowSeekInterval - elapsed;
    if (delay == Duration.zero) {
      _seekTimer?.cancel();
      _seekTimer = null;
      unawaited(_startPendingSeek(globalProjection, _timelineGeneration));
      return;
    }
    _seekTimer ??= Timer(delay, () {
      _seekTimer = null;
      unawaited(_startPendingSeek(globalProjection, _timelineGeneration));
    });
  }

  Future<void> _startPendingSeek(
    LibraryTimelineProjection globalProjection,
    int generation,
  ) async {
    if (!mounted || generation != _timelineGeneration) {
      return;
    }
    final value = _pendingSeekValue;
    final intent = _pendingSeekIntent ?? _LibraryTimelineSeekIntent.navigation;
    final seekIntentGeneration = _seekIntentGeneration;
    _pendingSeekValue = null;
    _pendingSeekIntent = null;
    _isPendingSeekFinal = false;
    if (value == null) {
      return;
    }
    if (_isTargetLoaded(globalProjection, value)) {
      if (!_isDragging && !_isSeeking) {
        setState(() => _interactiveValue = null);
      }
      return;
    }
    _activeSeekGenerations.add(seekIntentGeneration);
    _lastSeekStartedAt = DateTime.now();
    final target = _detailLoadTarget(globalProjection, value);
    final didSeek = await widget.onSeek(target.bucket, target.itemOffset);
    _activeSeekGenerations.remove(seekIntentGeneration);
    if (!mounted || generation != _timelineGeneration) {
      return;
    }
    if (didSeek &&
        intent == _LibraryTimelineSeekIntent.navigation &&
        seekIntentGeneration == _seekIntentGeneration) {
      await _alignLoadedSeekTarget(
        value,
        globalProjection,
        generation,
        seekIntentGeneration,
      );
    }
    if (!mounted || generation != _timelineGeneration) {
      return;
    }
    if (_pendingSeekValue case final pendingValue?) {
      if (_seekTimer != null) {
        return;
      }
      final pendingIntent =
          _pendingSeekIntent ?? _LibraryTimelineSeekIntent.navigation;
      final isPendingFinal = _isPendingSeekFinal;
      _queueSeek(
        _projectionForCurrentTimeline() ?? globalProjection,
        pendingValue,
        isFinal: isPendingFinal,
        immediate: isPendingFinal,
        intent: pendingIntent,
      );
      return;
    }
    if (!_isDragging && _pendingFrameValue == null) {
      setState(() => _interactiveValue = null);
    }
  }

  Future<void> _alignLoadedSeekTarget(
    double value,
    LibraryTimelineProjection fallbackProjection,
    int generation,
    int seekIntentGeneration,
  ) async {
    for (var attempt = 0; attempt < 8; attempt++) {
      WidgetsBinding.instance.scheduleFrame();
      await WidgetsBinding.instance.endOfFrame;
      if (!mounted ||
          generation != _timelineGeneration ||
          seekIntentGeneration != _seekIntentGeneration) {
        return;
      }
      final projection = _projectionForCurrentTimeline() ?? fallbackProjection;
      if (widget.layoutMetrics == null || !_isTargetLoaded(projection, value)) {
        continue;
      }
      _moveGalleryToValue(projection, value);
      return;
    }
  }

  void _moveGalleryToValue(
    LibraryTimelineProjection globalProjection,
    double value,
  ) {
    final metrics = widget.layoutMetrics;
    final position = widget.scrollController.hasClients
        ? widget.scrollController.position
        : null;
    final geometry = widget.virtualGeometry;
    if (position == null || !position.hasContentDimensions) {
      return;
    }
    final targetGlobalOffset = globalProjection
        .targetForValue(value)
        .globalItemOffset;
    double? targetPixels;
    if (metrics != null &&
        metrics.containsGlobalItemIndex(targetGlobalOffset.toDouble())) {
      final localPixels = metrics.offsetForGlobalItemIndex(targetGlobalOffset);
      if (localPixels != null) {
        targetPixels =
            (metrics.isQueryWide ? 0 : geometry?.leadingExtent ?? 0) +
            localPixels;
      }
    }
    targetPixels ??= geometry?.scrollOffsetForValue(value);
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
    if (_isSeeking) {
      _seekIntentGeneration += 1;
    }
    final projection = _projectionForCurrentTimeline();
    final metrics = widget.layoutMetrics;
    final position = widget.scrollController.hasClients
        ? widget.scrollController.position
        : null;
    if (projection == null ||
        metrics == null ||
        position == null ||
        !position.hasContentDimensions) {
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
        _clearPendingSeek();
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
        intent: _LibraryTimelineSeekIntent.prefetch,
      );
    });
  }

  void _cancelGalleryScrollSettle() {
    _galleryScrollSettleTimer?.cancel();
    _galleryScrollSettleTimer = null;
    _pendingGalleryScrollValue = null;
  }

  LibraryTimelineProjection? _projectionForCurrentTimeline() {
    final cachedProjection = _cachedProjection;
    if (cachedProjection != null) {
      return cachedProjection;
    }
    final timeline = widget.timeline;
    if (timeline == null || timeline.buckets.isEmpty) {
      return null;
    }
    return _cachedProjection = LibraryTimelineProjection(
      timeline: timeline,
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
    if (widget.loadedItemCount <= 0) {
      return false;
    }
    final targetGlobalOffset = projection
        .targetForValue(value)
        .globalItemOffset;
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
    );
  }

  void _clearPendingSeek() {
    _seekTimer?.cancel();
    _seekTimer = null;
    _pendingSeekValue = null;
    _pendingSeekIntent = null;
    _isPendingSeekFinal = false;
  }

  void _moveOneRow(LibraryGalleryLayoutMetrics metrics, int direction) {
    if (!widget.scrollController.hasClients || metrics.photoRowHeight <= 0) {
      return;
    }
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
