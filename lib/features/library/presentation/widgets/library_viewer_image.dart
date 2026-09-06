import "dart:async";
import "dart:io";

import "package:flutter/material.dart";
import "package:material_symbols_icons/symbols.dart";

import "../../application/library_source_read_scheduler.dart";
import "../../domain/library_models.dart";
import "library_loading_indicator.dart";
import "library_source_image.dart";

class LibraryViewerImage extends StatefulWidget {
  const LibraryViewerImage({
    required this.asset,
    this.sourceReadScheduler,
    this.sourceBufferLoader,
    super.key,
  });

  final LibraryAsset asset;
  final LibrarySourceReadScheduler? sourceReadScheduler;
  final LibrarySourceBufferLoader? sourceBufferLoader;

  @override
  State<LibraryViewerImage> createState() => _LibraryViewerImageState();
}

class _LibraryViewerImageState extends State<LibraryViewerImage> {
  late LibrarySourceImage _sourceImage;
  int _retryGeneration = 0;

  @override
  void initState() {
    super.initState();
    _sourceImage = _createSourceImage();
  }

  @override
  void didUpdateWidget(covariant LibraryViewerImage oldWidget) {
    super.didUpdateWidget(oldWidget);
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
    return Image(
      key: ValueKey(_sourceImage),
      image: _sourceImage,
      semanticLabel: widget.asset.relativePath,
      fit: BoxFit.contain,
      filterQuality: FilterQuality.high,
      gaplessPlayback: true,
      frameBuilder: (context, child, frame, wasSynchronouslyLoaded) {
        if (frame != null || wasSynchronouslyLoaded) {
          return child;
        }
        return Stack(
          fit: StackFit.expand,
          children: [
            if (_hasPreview) _buildPreview(),
            const LibraryLoadingIndicator(
              maximumDimension: 36,
              minimumInset: 0,
              strokeWidth: 4,
            ),
          ],
        );
      },
      errorBuilder: (context, error, stackTrace) => _buildFailure(context),
    );
  }

  Widget _buildFailure(BuildContext context) {
    if (_hasPreview) {
      return Stack(
        fit: StackFit.expand,
        children: [
          _buildPreview(),
          Positioned(
            left: 16,
            right: 16,
            top: 16,
            child: Center(
              child: Material(
                color: Theme.of(context).colorScheme.surfaceContainerHigh,
                elevation: 2,
                borderRadius: BorderRadius.circular(20),
                child: Padding(
                  padding: const EdgeInsets.only(left: 14, right: 6),
                  child: Row(
                    mainAxisSize: MainAxisSize.min,
                    children: [
                      const Flexible(child: Text("原图暂不可用，当前显示缩略图")),
                      TextButton(onPressed: _retry, child: const Text("重试")),
                    ],
                  ),
                ),
              ),
            ),
          ),
        ],
      );
    }
    return Center(
      child: Column(
        mainAxisSize: MainAxisSize.min,
        children: [
          const Icon(Symbols.broken_image_rounded, size: 64),
          const SizedBox(height: 12),
          const Text("无法打开原图"),
          const SizedBox(height: 8),
          OutlinedButton.icon(
            onPressed: _retry,
            icon: const Icon(Symbols.refresh_rounded),
            label: const Text("重试"),
          ),
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
    scheduler: widget.sourceReadScheduler,
    bufferLoader: widget.sourceBufferLoader,
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
