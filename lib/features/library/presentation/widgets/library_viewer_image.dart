import "dart:io";

import "package:flutter/material.dart";
import "package:material_symbols_icons/symbols.dart";

import "../../domain/library_models.dart";
import "library_loading_indicator.dart";

class LibraryViewerImage extends StatefulWidget {
  const LibraryViewerImage({required this.asset, super.key});

  final LibraryAsset asset;

  @override
  State<LibraryViewerImage> createState() => _LibraryViewerImageState();
}

class _LibraryViewerImageState extends State<LibraryViewerImage> {
  late FileImage _sourceImage;

  @override
  void initState() {
    super.initState();
    _sourceImage = _createSourceImage();
  }

  @override
  void didUpdateWidget(covariant LibraryViewerImage oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.asset.sourcePath != widget.asset.sourcePath) {
      _sourceImage.evict();
      _sourceImage = _createSourceImage();
    }
  }

  @override
  void dispose() {
    _sourceImage.evict();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return Image(
      key: ValueKey("viewer-source-${widget.asset.locationId}"),
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

  FileImage _createSourceImage() => FileImage(File(widget.asset.sourcePath));

  void _retry() {
    _sourceImage.evict();
    setState(() => _sourceImage = _createSourceImage());
  }

  bool get _hasPreview =>
      widget.asset.previewStatus == LibraryPreviewStatus.ready &&
      widget.asset.previewPath.isNotEmpty;
}
