import "dart:async";
import "dart:math" as math;

import "package:flutter/material.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";

import "../../../../app/presentation/ame_scroll_motion.dart";
import "../../application/library_controller.dart";
import "../../domain/library_models.dart";
import "library_duplicate_review_metrics.dart";
import "library_photo_tile.dart" show libraryPreviewDecodeWidth;

/// Owns the visible grid's bounded preview demand and hidden workspace retirement.
class LibraryDuplicateReviewGrid extends ConsumerStatefulWidget {
  const LibraryDuplicateReviewGrid({
    required this.itemCount,
    required this.assetAt,
    required this.itemBuilder,
    this.onVisibleRange,
    this.singleColumn = false,
    super.key,
  });
  final int itemCount;
  final LibraryAsset? Function(int index) assetAt;
  final IndexedWidgetBuilder itemBuilder;
  final Future<void> Function(int first, int end)? onVisibleRange;
  final bool singleColumn;

  @override
  ConsumerState<LibraryDuplicateReviewGrid> createState() =>
      _LibraryDuplicateReviewGridState();
}

class _LibraryDuplicateReviewGridState
    extends ConsumerState<LibraryDuplicateReviewGrid> {
  final _scroll = AmeScrollController();
  final _previewOwner = Object();
  late final _library = ref.read(libraryControllerProvider.notifier);
  double _height = 0;
  double _rowStride = 176;
  int _columns = 1;
  bool _active = true;
  bool _scheduled = false;

  @override
  void initState() {
    super.initState();
    _scroll.addListener(_scheduleDemand);
  }

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    _active = TickerMode.valuesOf(context).enabled && Visibility.of(context);
    if (_active) {
      _scheduleDemand();
    } else {
      _library.releaseFilmstripPreviewDemand(_previewOwner);
    }
  }

  @override
  void didUpdateWidget(covariant LibraryDuplicateReviewGrid oldWidget) {
    super.didUpdateWidget(oldWidget);
    _scheduleDemand();
  }

  void _scheduleDemand() {
    if (_scheduled || !_active) return;
    _scheduled = true;
    WidgetsBinding.instance.addPostFrameCallback((_) {
      _scheduled = false;
      if (!mounted || !_active || !_scroll.hasClients) return;
      final first =
          math.max(0, (_scroll.offset / _rowStride).floor()) * _columns;
      final last = math.min(
        widget.itemCount,
        (((_scroll.offset + _height) / _rowStride).ceil() + 1) * _columns,
      );
      _library.updateFilmstripPreviewDemand(
        _previewOwner,
        [for (var index = first; index < last; index++) ?widget.assetAt(index)],
        libraryPreviewDecodeWidth(144, MediaQuery.devicePixelRatioOf(context)),
      );
      unawaited(widget.onVisibleRange?.call(first, last));
    });
  }

  @override
  void dispose() {
    _library.releaseFilmstripPreviewDemand(_previewOwner);
    _scroll.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) => LayoutBuilder(
    builder: (context, constraints) {
      _height = constraints.maxHeight;
      final textSize = MediaQuery.textScalerOf(context).scale(16);
      final minimumWidth = 340 * (textSize / 16).clamp(1.0, 1.5);
      const spacing = 12.0;
      _columns = widget.singleColumn
          ? 1
          : math.max(
              1,
              ((constraints.maxWidth + spacing) / (minimumWidth + spacing))
                  .floor(),
            );
      final cardWidth =
          (constraints.maxWidth - (_columns - 1) * spacing) / _columns;
      final cardHeight = LibraryDuplicateReviewMetrics.height(
        context,
        cardWidth,
        member: widget.singleColumn,
      );
      _rowStride = cardHeight + spacing;
      _scheduleDemand();
      return AmeScrollMotion(
        controller: _scroll,
        child: GridView.builder(
          key: PageStorageKey(widget.key),
          controller: _scroll,
          padding: EdgeInsets.zero,
          gridDelegate: SliverGridDelegateWithFixedCrossAxisCount(
            crossAxisCount: _columns,
            mainAxisExtent: cardHeight,
            crossAxisSpacing: spacing,
            mainAxisSpacing: spacing,
          ),
          itemCount: widget.itemCount,
          itemBuilder: widget.itemBuilder,
        ),
      );
    },
  );
}
