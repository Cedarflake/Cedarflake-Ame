import "dart:async";
import "dart:io";

import "package:flutter/material.dart";
import "package:material_symbols_icons/symbols.dart";

import "../../application/library_source_read_scheduler.dart";
import "../../domain/library_models.dart";
import "../library_strings.dart";
import "library_image_reveal.dart";
import "library_loading_indicator.dart";
import "library_source_image.dart";
import "library_viewer_notice.dart";
import "library_viewer_source_scope.dart";
import "library_viewer_transition.dart";
import "library_viewer_zoom_geometry.dart";

typedef LibraryViewerImageBuilder =
    Widget Function(BuildContext context, Widget image);

class LibraryViewerImage extends StatefulWidget {
  const LibraryViewerImage({
    required this.asset,
    this.sourceReadScheduler,
    this.sourceBufferLoader,
    this.imageBuilder,
    this.zoomScale = 1,
    this.notice,
    this.operationNotice,
    super.key,
  });

  final LibraryAsset asset;
  final LibrarySourceReadScheduler? sourceReadScheduler;
  final LibrarySourceBufferLoader? sourceBufferLoader;
  final LibraryViewerImageBuilder? imageBuilder;
  final double zoomScale;
  final Widget? notice;
  final Widget? operationNotice;

  @override
  State<LibraryViewerImage> createState() => _LibraryViewerImageState();
}

class _LibraryViewerImageState extends State<LibraryViewerImage> {
  late LibrarySourceImage _sourceImage;
  LibraryViewerSourceScope? _sourceScope;
  int _retryGeneration = 0;
  bool _isActive = true;

  @override
  void initState() {
    super.initState();
    _sourceScope = context
        .getInheritedWidgetOfExactType<LibraryViewerSourceScope>();
    _sourceImage = _createSourceImage();
  }

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    _sourceScope = LibraryViewerSourceScope.maybeOf(context);
    final isActive = LibraryViewerActivity.isActiveOf(context);
    if (!isActive) {
      _isActive = false;
      _sourceImage.cancel();
      unawaited(_sourceImage.evict());
      return;
    }
    if (!_isActive) _retryGeneration += 1;
    _isActive = true;
    _updateSourceImage();
  }

  @override
  void didUpdateWidget(covariant LibraryViewerImage oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (_isActive) _updateSourceImage();
  }

  void _updateSourceImage() {
    final nextImage = _createSourceImage();
    if (_sourceImage != nextImage) {
      _sourceImage.cancel();
      unawaited(_sourceImage.evict());
      _sourceImage = nextImage;
    }
  }

  @override
  void dispose() {
    _sourceImage.cancel();
    unawaited(_sourceImage.evict());
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return LayoutBuilder(
      builder: (context, constraints) {
        final geometry = LibraryViewerZoomGeometry(
          imageSize: Size(
            widget.asset.width.toDouble(),
            widget.asset.height.toDouble(),
          ),
          viewportSize: constraints.biggest,
        );
        final quality = geometry.filterQualityForScale(
          widget.zoomScale,
          MediaQuery.devicePixelRatioOf(context),
        );
        return Stack(
          fit: StackFit.expand,
          children: [_buildSource(context, quality), ?widget.notice],
        );
      },
    );
  }

  Widget _buildSource(BuildContext context, FilterQuality quality) =>
      LibraryImageReveal(
        key: ValueKey(_sourceImage),
        image: _sourceImage,
        revealIdentity: _sourceImage,
        duration: const Duration(milliseconds: 100),
        placeholder: _hasPreview ? _buildPreview() : null,
        semanticLabel: widget.asset.relativePath,
        fit: BoxFit.contain,
        filterQuality: quality,
        frameBuilder: (context, child, frame, wasSynchronouslyLoaded) {
          return Stack(
            fit: StackFit.expand,
            children: [
              _buildImage(context, child),
              if (widget.notice == null &&
                  (frame != null || wasSynchronouslyLoaded))
                ?widget.operationNotice,
              if (frame == null && !wasSynchronouslyLoaded)
                const IgnorePointer(
                  child: LibraryLoadingIndicator(
                    key: Key("viewer-image-loading"),
                    maximumDimension: 36,
                    minimumInset: 0,
                    strokeWidth: 4,
                  ),
                ),
            ],
          );
        },
        errorBuilder: (context, error, stackTrace) => _buildFailure(context),
      );

  Widget _buildImage(BuildContext context, Widget image) =>
      widget.imageBuilder?.call(context, image) ?? image;

  Widget _buildFailure(BuildContext context) {
    if (!_hasPreview) {
      return _buildUnavailableSource();
    }
    return Image.file(
      File(widget.asset.previewPath),
      fit: BoxFit.contain,
      filterQuality: FilterQuality.medium,
      frameBuilder: (context, image, frame, wasSynchronouslyLoaded) {
        if (frame == null && !wasSynchronouslyLoaded) {
          return _buildUnavailableSource();
        }
        return _buildPreviewFallback(context, image);
      },
      errorBuilder: (context, error, stackTrace) => _buildUnavailableSource(),
    );
  }

  Widget _buildPreviewFallback(BuildContext context, Widget image) {
    if (widget.notice != null) return _buildImage(context, image);
    return Stack(
      fit: StackFit.expand,
      children: [
        _buildImage(context, image),
        LibraryViewerNotice(
          icon: Symbols.broken_image_rounded,
          message: LibraryStrings.originalImagePreviewFallback,
          actionLabel: LibraryStrings.retryOriginalImage,
          onAction: _retry,
          actionKey: const Key("viewer-original-retry"),
        ),
      ],
    );
  }

  Widget _buildUnavailableSource() {
    return Center(
      child: Column(
        mainAxisSize: MainAxisSize.min,
        children: [
          const Icon(Symbols.broken_image_rounded, size: 64),
          const SizedBox(height: 12),
          const Text(LibraryStrings.originalImageUnavailable),
          if (widget.notice == null) ...[
            const SizedBox(height: 8),
            OutlinedButton.icon(
              key: const Key("viewer-original-retry"),
              onPressed: _retry,
              icon: const Icon(Symbols.refresh_rounded),
              label: const Text(LibraryStrings.retryOriginalImage),
            ),
          ],
        ],
      ),
    );
  }

  Widget _buildPreview() {
    return Image.file(
      File(widget.asset.previewPath),
      fit: BoxFit.contain,
      filterQuality: FilterQuality.medium,
      errorBuilder: (context, error, stackTrace) => const SizedBox.shrink(),
    );
  }

  LibrarySourceImage _createSourceImage() => LibrarySourceImage(
    widget.asset,
    retryGeneration: _retryGeneration,
    scheduler: widget.sourceReadScheduler ?? _sourceScope?.scheduler,
    bufferLoader: widget.sourceBufferLoader ?? _sourceScope?.bufferLoader,
  );

  void _retry() {
    _sourceImage.cancel();
    unawaited(_sourceImage.evict());
    setState(() {
      _retryGeneration += 1;
      _sourceImage = _createSourceImage();
    });
  }

  bool get _hasPreview =>
      widget.asset.previewStatus == LibraryPreviewStatus.ready &&
      widget.asset.previewPath.isNotEmpty;
}
