import "dart:math" as math;

import "package:flutter/material.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";

import "../../../../app/presentation/ame_scroll_motion.dart";
import "../../application/library_controller.dart";
import "../../domain/library_models.dart";
import "library_photo_tile.dart" show libraryPreviewDecodeWidth;

/// Owns the bounded preview demand for the visible review rows, including hidden workspace retirement.
class LibraryDuplicateReviewList extends ConsumerStatefulWidget {
  const LibraryDuplicateReviewList({
    required this.assets,
    required this.itemBuilder,
    super.key,
  });
  final List<LibraryAsset> assets;
  final IndexedWidgetBuilder itemBuilder;

  @override
  ConsumerState<LibraryDuplicateReviewList> createState() =>
      _LibraryDuplicateReviewListState();
}

class _LibraryDuplicateReviewListState
    extends ConsumerState<LibraryDuplicateReviewList> {
  final _scroll = ScrollController();
  final _previewOwner = Object();
  late final _library = ref.read(libraryControllerProvider.notifier);
  double _height = 0;
  double _extent = 176;
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
  void didUpdateWidget(covariant LibraryDuplicateReviewList oldWidget) {
    super.didUpdateWidget(oldWidget);
    _scheduleDemand();
  }

  void _scheduleDemand() {
    if (_scheduled || !_active) return;
    _scheduled = true;
    WidgetsBinding.instance.addPostFrameCallback((_) {
      _scheduled = false;
      if (!mounted || !_active || !_scroll.hasClients) return;
      final first = math.max(0, (_scroll.offset / _extent).floor());
      final last = math.min(
        widget.assets.length,
        ((_scroll.offset + _height) / _extent).ceil() + 1,
      );
      _library.updateFilmstripPreviewDemand(
        _previewOwner,
        widget.assets.skip(first).take(math.max(0, last - first)),
        libraryPreviewDecodeWidth(144, MediaQuery.devicePixelRatioOf(context)),
      );
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
      _extent = math.max(
        176,
        MediaQuery.textScalerOf(context).scale(16) * 8 + 40,
      );
      _scheduleDemand();
      return AmeScrollMotion(
        child: ListView.builder(
          controller: _scroll,
          itemExtent: _extent,
          itemCount: widget.assets.length,
          itemBuilder: widget.itemBuilder,
        ),
      );
    },
  );
}
