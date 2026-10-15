import "dart:async";
import "dart:math" as math;

import "package:flutter/material.dart";
import "package:flutter/rendering.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";

import "../../../../app/presentation/ame_scroll_motion.dart";
import "../../application/library_browse_admission.dart";
import "../../application/library_controller.dart";
import "../../domain/library_models.dart";
import "../../domain/library_state.dart";
import "library_photo_tile.dart" show libraryPreviewDecodeWidth;
import "library_viewer_filmstrip_scrollbar.dart";
import "library_viewer_filmstrip_thumbnail.dart";
import "library_viewer_transition.dart";

/// Browses the retained catalog window without instantiating offscreen thumbnails.
class LibraryViewerFilmstrip extends ConsumerStatefulWidget {
  const LibraryViewerFilmstrip({
    required this.selection,
    required this.onSelected,
    required this.loadPrevious,
    super.key,
  });

  final LibraryAsset selection;
  final ValueChanged<LibraryAsset> onSelected;
  final Future<void> Function() loadPrevious;

  @override
  ConsumerState<LibraryViewerFilmstrip> createState() =>
      _LibraryViewerFilmstripState();
}

class _LibraryViewerFilmstripState
    extends ConsumerState<LibraryViewerFilmstrip> {
  static const _extent = 100.0;
  late final _controller = ref.read(libraryControllerProvider.notifier);
  final _scroll = AmeScrollController();
  final _previewOwner = Object();
  int? _windowStart;
  String? _queryId;
  bool _centerSelection = true;
  bool _revealSelection = false;
  bool _frameScheduled = false;
  bool _isPaging = false;
  bool _pagingFailed = false;
  double _width = 0;
  bool _isActive = true;
  LibraryState? _displayState;

  @override
  void initState() {
    super.initState();
    _scroll.addListener(_scheduleDemand);
  }

  @override
  void didUpdateWidget(covariant LibraryViewerFilmstrip oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.selection.locationId != widget.selection.locationId) {
      _revealSelection = true;
      _pagingFailed = false;
    }
  }

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    _isActive =
        LibraryViewerActivity.isActiveOf(context) && Visibility.of(context);
    if (!_isActive) {
      _controller.releaseFilmstripPreviewDemand(_previewOwner);
      if (_scroll.hasClients) _scroll.jumpTo(_scroll.offset);
    }
  }

  @override
  void dispose() {
    _scroll.dispose();
    _controller.releaseFilmstripPreviewDemand(_previewOwner);
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final current = ref.watch(libraryControllerProvider);
    if (_isActive) _displayState = current;
    final state = _displayState ?? current;
    return Material(
      color: Theme.of(context).colorScheme.surfaceContainerLow,
      child: LayoutBuilder(
        builder: (context, constraints) {
          _width = constraints.maxWidth;
          _scheduleDemand();
          return Stack(
            children: [
              AmeScrollMotion(
                controller: _scroll,
                scrollDirection: Axis.horizontal,
                child: LibraryViewerFilmstripScrollbar(
                  controller: _scroll,
                  step: _extent,
                  child: ListView.builder(
                    key: const Key("viewer-filmstrip"),
                    controller: _scroll,
                    scrollDirection: Axis.horizontal,
                    itemExtent: _extent,
                    scrollCacheExtent: const ScrollCacheExtent.pixels(_extent),
                    padding: const EdgeInsets.only(
                      top: 4,
                      bottom: LibraryViewerFilmstripScrollbar.controlExtent,
                    ),
                    itemCount: state.assets.length,
                    findChildIndexCallback: (key) {
                      if (key is! ValueKey<String>) return null;
                      final index = state.assets.indexWhere(
                        (asset) => asset.locationId == key.value,
                      );
                      return index < 0 ? null : index;
                    },
                    itemBuilder: (context, index) {
                      final asset = state.assets[index];
                      return LibraryViewerFilmstripThumbnail(
                        key: ValueKey(asset.locationId),
                        asset: asset,
                        isSelected:
                            asset.locationId == widget.selection.locationId,
                        onPressed: () => widget.onSelected(asset),
                      );
                    },
                  ),
                ),
              ),
              if (_isPaging || _pagingFailed)
                Positioned(
                  right: 8,
                  top: 8,
                  child: _pagingFailed
                      ? FilledButton.tonal(
                          onPressed: () {
                            setState(() => _pagingFailed = false);
                            _scheduleDemand();
                          },
                          child: const Text("重新加载缩略图"),
                        )
                      : const SizedBox.square(
                          dimension: 20,
                          child: CircularProgressIndicator(strokeWidth: 2),
                        ),
                ),
            ],
          );
        },
      ),
    );
  }

  void _scheduleDemand() {
    if (_frameScheduled || !_isActive) return;
    _frameScheduled = true;
    WidgetsBinding.instance.addPostFrameCallback((_) {
      _frameScheduled = false;
      if (!mounted || !_isActive || !_scroll.hasClients) return;
      final state = ref.read(libraryControllerProvider);
      if (_queryId != state.queryId) {
        _centerSelection = true;
        _pagingFailed = false;
      }
      _queryId = state.queryId;
      final previousStart = _windowStart;
      _windowStart = state.windowStartItemOffset;
      if (_centerSelection) {
        final index = state.assets.indexWhere(
          (asset) => asset.locationId == widget.selection.locationId,
        );
        if (index >= 0) {
          _centerSelection = false;
          _jump((index + 0.5) * _extent - _width / 2);
        }
      } else if (previousStart != null && previousStart != _windowStart) {
        // Prepending or retiring a page preserves the same global strip position.
        _jump(_scroll.offset + (previousStart - _windowStart!) * _extent);
      }
      if (_revealSelection) {
        final index = state.assets.indexWhere(
          (asset) => asset.locationId == widget.selection.locationId,
        );
        if (index >= 0) {
          _revealSelection = false;
          final leading = index * _extent;
          final trailing = leading + _extent;
          if (leading < _scroll.offset) {
            _jump(leading);
          } else if (trailing > _scroll.offset + _width) {
            _jump(trailing - _width);
          }
        }
      }
      final start = math.max(0, (_scroll.offset / _extent).floor() - 1);
      final end = math.min(
        state.assets.length,
        ((_scroll.offset + _width) / _extent).ceil() + 1,
      );
      if (_selectionNeedsPage(state)) {
        _controller.releaseFilmstripPreviewDemand(_previewOwner);
      } else {
        _controller.updateFilmstripPreviewDemand(
          _previewOwner,
          state.assets.skip(start).take(math.max(0, end - start)),
          libraryPreviewDecodeWidth(
            _extent,
            MediaQuery.devicePixelRatioOf(context),
          ),
        );
      }
      if (!_isPaging && !_pagingFailed) {
        unawaited(_loadAtEdge(state, start, end));
      }
    });
  }

  void _jump(double offset) =>
      _scroll.jumpTo(offset.clamp(0.0, _scroll.position.maxScrollExtent));

  bool _selectionNeedsPage(LibraryState state) =>
      (_centerSelection || _revealSelection) &&
      !state.assets.any(
        (asset) => asset.locationId == widget.selection.locationId,
      );

  Future<void> _loadAtEdge(LibraryState state, int start, int end) async {
    if (!LibraryBrowseAdmission(state).canReadPage) return;
    final locateSelection = _selectionNeedsPage(state);
    final selectedLocation = widget.selection.locationId;
    final previous = start <= 1 && state.hasPreviousAssets;
    final next = end >= state.assets.length - 1 && state.hasMoreAssets;
    if (!locateSelection && !previous && !next) return;
    setState(() => _isPaging = true);
    try {
      if (locateSelection) {
        await _controller.updateQuery(
          state.query,
          anchorLocationId: selectedLocation,
          anchorAssetId: widget.selection.assetId,
          forceRefresh: true,
          showRefreshingStatus: false,
        );
      } else if (previous) {
        await widget.loadPrevious();
      } else {
        await _controller.loadNextPage();
      }
      if (!mounted) return;
      final current = ref.read(libraryControllerProvider);
      if (current.query != state.query ||
          widget.selection.locationId != selectedLocation) {
        return;
      }
      final bool failed;
      if (locateSelection) {
        _queryId = current.queryId;
        _centerSelection = true;
        failed = !current.assets.any(
          (asset) => asset.locationId == selectedLocation,
        );
      } else if (previous) {
        failed = current.previousPageErrorMessage != null;
      } else {
        failed = current.pageErrorMessage != null;
      }
      setState(() => _pagingFailed = failed);
    } on Object {
      if (mounted &&
          ref.read(libraryControllerProvider).query == state.query &&
          widget.selection.locationId == selectedLocation) {
        setState(() => _pagingFailed = true);
      }
    } finally {
      if (mounted) {
        setState(() => _isPaging = false);
        _scheduleDemand();
      }
    }
  }
}
