import "dart:async";
import "dart:io";
import "dart:math" as math;

import "package:flutter/material.dart";
import "package:flutter/rendering.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:material_symbols_icons/symbols.dart";

import "../../../../app/presentation/ame_scroll_motion.dart";
import "../../application/library_browse_admission.dart";
import "../../application/library_controller.dart";
import "library_photo_tile.dart" show libraryPreviewDecodeWidth;
import "../../domain/library_models.dart";
import "../../domain/library_state.dart";
import "library_path_text.dart";
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
  bool _frameScheduled = false;
  bool _isPaging = false;
  bool _pagingFailed = false;
  double _width = 0;
  bool _isActive = true;

  @override
  void initState() {
    super.initState();
    _scroll.addListener(_scheduleDemand);
  }

  @override
  void didUpdateWidget(covariant LibraryViewerFilmstrip oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.selection.locationId != widget.selection.locationId) {
      _centerSelection = true;
      _pagingFailed = false;
    }
  }

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    _isActive = LibraryViewerActivity.isActiveOf(context);
    if (!_isActive) _controller.releaseFilmstripPreviewDemand(_previewOwner);
  }

  @override
  void dispose() {
    _scroll.dispose();
    _controller.releaseFilmstripPreviewDemand(_previewOwner);
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final state = ref.watch(libraryControllerProvider);
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
                child: Scrollbar(
                  controller: _scroll,
                  thumbVisibility: true,
                  child: ListView.builder(
                    key: const Key("viewer-filmstrip"),
                    controller: _scroll,
                    scrollDirection: Axis.horizontal,
                    itemExtent: _extent,
                    scrollCacheExtent: const ScrollCacheExtent.pixels(_extent),
                    padding: const EdgeInsets.only(top: 4, bottom: 12),
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
                      return _FilmstripThumbnail(
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
      if (_queryId != state.queryId) _centerSelection = true;
      _queryId = state.queryId;
      final previousStart = _windowStart;
      _windowStart = state.windowStartItemOffset;
      if (_centerSelection) {
        _centerSelection = false;
        final index = state.assets.indexWhere(
          (asset) => asset.locationId == widget.selection.locationId,
        );
        if (index >= 0) _jump((index + 0.5) * _extent - _width / 2);
      } else if (previousStart != null && previousStart != _windowStart) {
        // Prepending or retiring a page preserves the same global strip position.
        _jump(_scroll.offset + (previousStart - _windowStart!) * _extent);
      }
      final start = math.max(0, (_scroll.offset / _extent).floor() - 1);
      final end = math.min(
        state.assets.length,
        ((_scroll.offset + _width) / _extent).ceil() + 1,
      );
      _controller.updateFilmstripPreviewDemand(
        _previewOwner,
        state.assets.skip(start).take(math.max(0, end - start)),
        libraryPreviewDecodeWidth(
          _extent,
          MediaQuery.devicePixelRatioOf(context),
        ),
      );
      if (!_isPaging && !_pagingFailed) {
        unawaited(_loadAtEdge(state, start, end));
      }
    });
  }

  void _jump(double offset) =>
      _scroll.jumpTo(offset.clamp(0.0, _scroll.position.maxScrollExtent));

  Future<void> _loadAtEdge(LibraryState state, int start, int end) async {
    if (!LibraryBrowseAdmission(state).canReadPage) return;
    final previous = start <= 1 && state.hasPreviousAssets;
    final next = end >= state.assets.length - 1 && state.hasMoreAssets;
    if (!previous && !next) return;
    setState(() => _isPaging = true);
    try {
      if (previous) {
        await widget.loadPrevious();
      } else {
        await _controller.loadNextPage();
      }
      if (!mounted) return;
      final current = ref.read(libraryControllerProvider);
      final failed = previous
          ? current.previousPageErrorMessage != null
          : current.pageErrorMessage != null;
      setState(() => _pagingFailed = failed);
    } on Object {
      if (mounted) setState(() => _pagingFailed = true);
    } finally {
      if (mounted) {
        setState(() => _isPaging = false);
        _scheduleDemand();
      }
    }
  }
}

class _FilmstripThumbnail extends ConsumerWidget {
  const _FilmstripThumbnail({
    required this.asset,
    required this.isSelected,
    required this.onPressed,
    super.key,
  });

  final LibraryAsset asset;
  final bool isSelected;
  final VoidCallback onPressed;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final controller = ref.read(libraryControllerProvider.notifier);
    final colors = Theme.of(context).colorScheme;
    return Padding(
      padding: const EdgeInsets.symmetric(horizontal: 3),
      child: Semantics(
        button: true,
        selected: isSelected,
        label: displayLibraryFileName(asset.displayPath),
        child: Material(
          clipBehavior: Clip.antiAlias,
          color: colors.surfaceContainerHigh,
          shape: RoundedRectangleBorder(
            borderRadius: BorderRadius.circular(6),
            side: BorderSide(
              color: isSelected ? colors.primary : Colors.transparent,
              width: 2,
            ),
          ),
          child: InkWell(
            onTap: onPressed,
            child: Padding(
              padding: const EdgeInsets.all(3),
              child: ExcludeSemantics(
                child: StreamBuilder<void>(
                  stream: controller.watchPreview(asset.locationId),
                  builder: (context, _) {
                    final resolved = controller.resolvePreview(asset);
                    if (resolved.previewStatus != LibraryPreviewStatus.ready) {
                      return const Icon(Symbols.image_rounded, size: 22);
                    }
                    return Image.file(
                      File(resolved.previewPath),
                      fit: BoxFit.contain,
                      cacheWidth: libraryPreviewDecodeWidth(
                        94,
                        MediaQuery.devicePixelRatioOf(context),
                      ),
                      errorBuilder: (context, error, stack) =>
                          const Icon(Symbols.broken_image_rounded, size: 22),
                    );
                  },
                ),
              ),
            ),
          ),
        ),
      ),
    );
  }
}
