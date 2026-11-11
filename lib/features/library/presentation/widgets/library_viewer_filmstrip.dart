import "dart:async";
import "dart:math" as math;

import "package:material_ui/material_ui.dart";
import "package:flutter/rendering.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";

import "../../../../app/presentation/ame_progress.dart";
import "../../../../app/presentation/ame_scroll_motion.dart";
import "../../application/library_browse_admission.dart";
import "../../application/library_controller.dart";
import "../../domain/library_models.dart";
import "../../domain/library_state.dart";
import "../coordination/library_filmstrip_paging.dart";
import "../coordination/library_filmstrip_position.dart";
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
    this.position,
    this.onScrollActivityChanged,
    super.key,
  });

  final LibraryAsset selection;
  final ValueChanged<LibraryAsset> onSelected;
  final Future<void> Function() loadPrevious;
  final LibraryFilmstripPosition? position;
  final ValueChanged<bool>? onScrollActivityChanged;

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
  late final LibraryFilmstripPaging _paging;
  late final _position = widget.position ?? LibraryFilmstripPosition();
  int? _windowStart;
  int _leadingItems = 0;
  String? _queryId;
  bool _centerSelection = true;
  bool _revealSelection = false;
  bool _frameScheduled = false;
  double _width = 0;
  bool _isActive = true;
  LibraryState? _displayState;

  @override
  void initState() {
    super.initState();
    _paging = LibraryFilmstripPaging(
      controller: _controller,
      readState: () => ref.read(libraryControllerProvider),
      readSelection: () => widget.selection,
      loadPrevious: () => widget.loadPrevious(),
    )..addListener(_pagingChanged);
    _scroll.addListener(_handleScroll);
  }

  void _handleScroll() {
    final state = _displayState;
    if (_isActive && state != null && _scroll.hasClients) {
      _position.capture(
        state,
        widget.selection,
        _scroll.offset,
        _extent,
        leadingItems: _leadingItems,
      );
    }
    _scheduleDemand();
  }

  void _pagingChanged() {
    if (mounted) setState(() {});
  }

  @override
  void didUpdateWidget(covariant LibraryViewerFilmstrip oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.selection.locationId != widget.selection.locationId) {
      _revealSelection = true;
      _handleScroll();
      _paging.selectionChanged();
    }
  }

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    final wasActive = _isActive;
    _isActive =
        LibraryViewerActivity.isActiveOf(context) && Visibility.of(context);
    if (_isActive && !wasActive) _centerSelection = true;
    if (!_isActive) {
      _position.clear();
      widget.onScrollActivityChanged?.call(false);
      _controller.releaseFilmstripPreviewDemand(_previewOwner);
      if (_scroll.hasClients) _scroll.jumpTo(_scroll.offset);
    }
  }

  @override
  void dispose() {
    _scroll.dispose();
    _paging.dispose();
    _position.clear();
    widget.onScrollActivityChanged?.call(false);
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
          final hasPaging = _paging.isLoading || _paging.hasFailure;
          final leadingItems =
              hasPaging && _paging.page == LibraryFilmstripPage.previous
              ? 1
              : 0;
          _width = constraints.maxWidth;
          _synchronizeWindow(state, leadingItems);
          _scheduleDemand();
          return LibraryViewerFilmstripScrollbar(
            controller: _scroll,
            step: _extent,
            child: NotificationListener<ScrollNotification>(
              onNotification: (notification) {
                if (notification.depth == 0 && _isActive) {
                  if (notification is ScrollStartNotification) {
                    widget.onScrollActivityChanged?.call(true);
                  } else if (notification is ScrollEndNotification) {
                    widget.onScrollActivityChanged?.call(false);
                  }
                }
                return false;
              },
              child: AmeScrollMotion(
                controller: _scroll,
                scrollDirection: Axis.horizontal,
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
                  itemCount: state.assets.length + (hasPaging ? 1 : 0),
                  findChildIndexCallback: (key) {
                    if (key == const Key("viewer-filmstrip-paging")) {
                      if (!hasPaging) return null;
                      return leadingItems == 1 ? 0 : state.assets.length;
                    }
                    if (key is! ValueKey<String>) return null;
                    final index = state.assets.indexWhere(
                      (asset) => asset.locationId == key.value,
                    );
                    return index < 0 ? null : index + leadingItems;
                  },
                  itemBuilder: (context, index) {
                    final assetIndex = index - leadingItems;
                    if (assetIndex < 0 || assetIndex >= state.assets.length) {
                      return _buildPagingCard(context);
                    }
                    final asset = state.assets[assetIndex];
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
          );
        },
      ),
    );
  }

  Widget _buildPagingCard(BuildContext context) => Container(
    key: const Key("viewer-filmstrip-paging"),
    margin: const EdgeInsets.symmetric(horizontal: 3),
    decoration: BoxDecoration(
      color: Theme.of(context).colorScheme.surfaceContainerHighest,
      borderRadius: BorderRadius.circular(6),
    ),
    alignment: Alignment.center,
    child: _paging.hasFailure
        ? FilledButton.tonal(
            onPressed: () {
              final page = _paging.page;
              if (page != null) unawaited(_loadPage(page, retry: true));
            },
            style: FilledButton.styleFrom(padding: const EdgeInsets.all(8)),
            child: const Text("重新加载缩略图"),
          )
        : const SizedBox.square(
            dimension: 20,
            child: AmeCircularProgress(strokeWidth: 2),
          ),
  );

  void _synchronizeWindow(LibraryState state, int leadingItems) {
    final retainedOffset = _position.offsetIn(
      state,
      widget.selection,
      _extent,
      leadingItems: leadingItems,
    );
    if (_queryId != state.queryId) {
      _centerSelection = retainedOffset == null;
      _paging.clearFailure();
    }
    final previousStart = _windowStart;
    final nextStart = state.windowStartItemOffset - leadingItems;
    if (!_centerSelection && previousStart != null && _scroll.hasClients) {
      // Correct during layout so retained tiles never paint at the old page
      // offset. Unlike jumpTo, this preserves the native scrollbar's drag.
      _scroll.position.correctBy(
        retainedOffset == null
            ? (previousStart - nextStart) * _extent
            : retainedOffset - _scroll.offset,
      );
    }
    _queryId = state.queryId;
    _windowStart = nextStart;
    _leadingItems = leadingItems;
  }

  void _scheduleDemand() {
    if (_frameScheduled || !_isActive) return;
    _frameScheduled = true;
    WidgetsBinding.instance.addPostFrameCallback((_) {
      _frameScheduled = false;
      if (!mounted || !_isActive || !_scroll.hasClients) return;
      final state = ref.read(libraryControllerProvider);
      if (_centerSelection) {
        final index = state.assets.indexWhere(
          (asset) => asset.locationId == widget.selection.locationId,
        );
        if (index >= 0) {
          _centerSelection = false;
          _jump((index + _leadingItems + 0.5) * _extent - _width / 2);
        }
      }
      if (_revealSelection) {
        final index = state.assets.indexWhere(
          (asset) => asset.locationId == widget.selection.locationId,
        );
        if (index >= 0) {
          _revealSelection = false;
          final leading = (index + _leadingItems) * _extent;
          final trailing = leading + _extent;
          if (leading < _scroll.offset) {
            _jump(leading);
          } else if (trailing > _scroll.offset + _width) {
            _jump(trailing - _width);
          }
        } else if (_paging.page == LibraryFilmstripPage.selection &&
            (_paging.isLoading || _paging.hasFailure)) {
          final end = math.max(
            0.0,
            (state.assets.length + 1) * _extent - _width,
          );
          // A lazy list may publish its new extent only after reaching the old end.
          _revealSelection = _scroll.position.maxScrollExtent < end;
          _centerSelection = true;
          _jump(end);
        }
      }
      final start = math.max(
        0,
        (_scroll.offset / _extent).floor() - _leadingItems - 1,
      );
      final end = math.min(
        state.assets.length,
        ((_scroll.offset + _width) / _extent).ceil() - _leadingItems + 1,
      );
      _position.capture(
        state,
        widget.selection,
        _scroll.offset,
        _extent,
        leadingItems: _leadingItems,
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
      if (!_paging.isLoading && !_paging.hasFailure) {
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
    final previous = start <= 1 && state.hasPreviousAssets;
    final next = end >= state.assets.length - 1 && state.hasMoreAssets;
    if (!locateSelection && !previous && !next) return;
    final LibraryFilmstripPage page;
    if (locateSelection) {
      page = LibraryFilmstripPage.selection;
    } else if (previous) {
      page = LibraryFilmstripPage.previous;
    } else {
      page = LibraryFilmstripPage.next;
    }
    await _loadPage(page);
  }

  Future<void> _loadPage(
    LibraryFilmstripPage page, {
    bool retry = false,
  }) async {
    final state = ref.read(libraryControllerProvider);
    final locateSelection = page == LibraryFilmstripPage.selection;
    if (locateSelection) _revealSelection = true;
    final located = await _paging.load(page, retry: retry);
    if (!mounted) return;
    if (locateSelection &&
        ref.read(libraryControllerProvider).query == state.query) {
      _queryId = ref.read(libraryControllerProvider).queryId;
    }
    if (located) {
      _centerSelection = true;
    }
    _scheduleDemand();
  }
}
