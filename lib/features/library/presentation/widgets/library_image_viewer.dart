import "dart:io";
import "dart:math" as math;

import "package:flutter/material.dart";

import "../../domain/library_models.dart";
import "../library_strings.dart";

class LibraryImageViewer extends StatefulWidget {
  const LibraryImageViewer({
    required this.asset,
    required this.onBack,
    required this.onInformation,
    super.key,
  });

  final LibraryAsset asset;
  final VoidCallback onBack;
  final VoidCallback onInformation;

  @override
  State<LibraryImageViewer> createState() => _LibraryImageViewerState();
}

class _LibraryImageViewerState extends State<LibraryImageViewer> {
  static const _minimumScale = 0.25;
  static const _maximumScale = 8.0;
  static const _zoomStep = 1.25;

  final TransformationController _transformationController =
      TransformationController();
  Size _viewportSize = Size.zero;
  double _scale = 1;

  @override
  void initState() {
    super.initState();
    _transformationController.addListener(_handleTransformationChanged);
  }

  @override
  void didUpdateWidget(covariant LibraryImageViewer oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.asset.locationId != widget.asset.locationId) {
      _resetZoom();
    }
  }

  @override
  void dispose() {
    _transformationController
      ..removeListener(_handleTransformationChanged)
      ..dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return Column(
      children: [
        SizedBox(
          height: 72,
          child: Padding(
            padding: const EdgeInsets.symmetric(horizontal: 16),
            child: Row(
              children: [
                IconButton(
                  tooltip: LibraryStrings.backToLibrary,
                  onPressed: widget.onBack,
                  icon: const Icon(Icons.arrow_back),
                ),
                const SizedBox(width: 8),
                Expanded(
                  child: Text(
                    widget.asset.relativePath,
                    maxLines: 1,
                    overflow: TextOverflow.ellipsis,
                    style: Theme.of(context).textTheme.titleMedium,
                  ),
                ),
                IconButton(
                  tooltip: LibraryStrings.viewInformation,
                  onPressed: widget.onInformation,
                  icon: const Icon(Icons.info_outline),
                ),
              ],
            ),
          ),
        ),
        const Divider(height: 1),
        Expanded(
          child: ColoredBox(
            color: Theme.of(context).colorScheme.surfaceContainerLowest,
            child: LayoutBuilder(
              builder: (context, constraints) {
                _viewportSize = constraints.biggest;
                return Stack(
                  fit: StackFit.expand,
                  children: [
                    InteractiveViewer(
                      transformationController: _transformationController,
                      alignment: Alignment.topLeft,
                      boundaryMargin: const EdgeInsets.all(double.infinity),
                      minScale: _minimumScale,
                      maxScale: _maximumScale,
                      scaleFactor: 180,
                      trackpadScrollCausesScale: true,
                      child: SizedBox(
                        width: constraints.maxWidth,
                        height: constraints.maxHeight,
                        child: _ViewerImage(asset: widget.asset),
                      ),
                    ),
                    Positioned(
                      left: 16,
                      right: 16,
                      bottom: 20,
                      child: Center(child: _buildZoomControls(context)),
                    ),
                  ],
                );
              },
            ),
          ),
        ),
      ],
    );
  }

  Widget _buildZoomControls(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    return Material(
      color: colorScheme.surfaceContainerHigh.withValues(alpha: 0.94),
      elevation: 2,
      borderRadius: BorderRadius.circular(28),
      child: SizedBox(
        height: 52,
        child: Row(
          mainAxisSize: MainAxisSize.min,
          children: [
            IconButton(
              tooltip: "缩小",
              onPressed: _scale <= _minimumScale + 0.001
                  ? null
                  : () => _setScale(_scale / _zoomStep),
              icon: const Icon(Icons.remove),
            ),
            SizedBox(
              width: 180,
              child: Slider(
                value: _sliderValueForScale(_scale),
                onChanged: (value) => _setScale(_scaleForSliderValue(value)),
                semanticFormatterCallback: (value) =>
                    "${(_scaleForSliderValue(value) * 100).round()}%",
              ),
            ),
            Tooltip(
              message: "适合窗口",
              child: TextButton(
                onPressed: _resetZoom,
                child: Text("${(_scale * 100).round()}%"),
              ),
            ),
            IconButton(
              tooltip: "放大",
              onPressed: _scale >= _maximumScale - 0.001
                  ? null
                  : () => _setScale(_scale * _zoomStep),
              icon: const Icon(Icons.add),
            ),
          ],
        ),
      ),
    );
  }

  void _handleTransformationChanged() {
    final nextScale = _transformationController.value
        .getMaxScaleOnAxis()
        .clamp(_minimumScale, _maximumScale)
        .toDouble();
    if (mounted && (nextScale - _scale).abs() > 0.001) {
      setState(() => _scale = nextScale);
    }
  }

  void _resetZoom() {
    _transformationController.value = Matrix4.identity();
  }

  void _setScale(double value) {
    if (_viewportSize.isEmpty) {
      return;
    }
    final nextScale = value.clamp(_minimumScale, _maximumScale).toDouble();
    final viewportCenter = Offset(
      _viewportSize.width / 2,
      _viewportSize.height / 2,
    );
    final sceneCenter = _transformationController.toScene(viewportCenter);
    _transformationController.value = Matrix4.identity()
      ..translate(viewportCenter.dx, viewportCenter.dy)
      ..scale(nextScale)
      ..translate(-sceneCenter.dx, -sceneCenter.dy);
  }

  double _sliderValueForScale(double scale) {
    return (math.log(scale) - math.log(_minimumScale)) /
        (math.log(_maximumScale) - math.log(_minimumScale));
  }

  double _scaleForSliderValue(double value) {
    return math.exp(
      math.log(_minimumScale) +
          value * (math.log(_maximumScale) - math.log(_minimumScale)),
    );
  }
}

class _ViewerImage extends StatefulWidget {
  const _ViewerImage({required this.asset});

  final LibraryAsset asset;

  @override
  State<_ViewerImage> createState() => _ViewerImageState();
}

class _ViewerImageState extends State<_ViewerImage> {
  late FileImage _sourceImage;

  @override
  void initState() {
    super.initState();
    _sourceImage = FileImage(File(widget.asset.sourcePath));
  }

  @override
  void didUpdateWidget(covariant _ViewerImage oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.asset.sourcePath != widget.asset.sourcePath) {
      _sourceImage.evict();
      _sourceImage = FileImage(File(widget.asset.sourcePath));
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
            if (_hasPreview)
              Image.file(
                File(widget.asset.previewPath),
                fit: BoxFit.contain,
                filterQuality: FilterQuality.medium,
              ),
            const Center(child: CircularProgressIndicator()),
          ],
        );
      },
      errorBuilder: (context, error, stackTrace) {
        if (_hasPreview) {
          return Stack(
            fit: StackFit.expand,
            children: [
              Image.file(
                File(widget.asset.previewPath),
                fit: BoxFit.contain,
                filterQuality: FilterQuality.high,
              ),
              const Positioned(
                left: 16,
                right: 16,
                top: 16,
                child: Center(
                  child: Material(
                    borderRadius: BorderRadius.all(Radius.circular(16)),
                    child: Padding(
                      padding: EdgeInsets.symmetric(
                        horizontal: 12,
                        vertical: 6,
                      ),
                      child: Text("原图暂不可用，当前显示缩略图"),
                    ),
                  ),
                ),
              ),
            ],
          );
        }
        return const Center(child: Icon(Icons.broken_image_outlined, size: 64));
      },
    );
  }

  bool get _hasPreview =>
      widget.asset.previewStatus == LibraryPreviewStatus.ready &&
      widget.asset.previewPath.isNotEmpty;
}

Future<void> showLibraryAssetInformation(
  BuildContext context,
  LibraryAsset asset,
) {
  final captureTime = asset.captureTime?.localTime ?? "未知";
  final size = _formatBytes(asset.fileSize);
  return showModalBottomSheet<void>(
    context: context,
    showDragHandle: true,
    builder: (context) => SafeArea(
      child: Padding(
        padding: const EdgeInsets.fromLTRB(24, 8, 24, 24),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            Text(
              LibraryStrings.viewInformation,
              style: Theme.of(context).textTheme.titleLarge,
            ),
            const SizedBox(height: 16),
            _InformationRow(label: "文件", value: asset.relativePath),
            _InformationRow(label: "路径", value: asset.sourcePath),
            _InformationRow(
              label: "尺寸",
              value: "${asset.width} × ${asset.height}",
            ),
            _InformationRow(label: "大小", value: size),
            _InformationRow(label: "拍摄时间", value: captureTime),
          ],
        ),
      ),
    ),
  );
}

class _InformationRow extends StatelessWidget {
  const _InformationRow({required this.label, required this.value});

  final String label;
  final String value;

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 8),
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          SizedBox(
            width: 88,
            child: Text(
              label,
              style: TextStyle(
                color: Theme.of(context).colorScheme.onSurfaceVariant,
              ),
            ),
          ),
          Expanded(child: SelectableText(value)),
        ],
      ),
    );
  }
}

String _formatBytes(BigInt value) {
  final bytes = value.toDouble();
  if (bytes < 1024) {
    return "${value.toInt()} B";
  }
  if (bytes < 1024 * 1024) {
    return "${(bytes / 1024).toStringAsFixed(1)} KB";
  }
  return "${(bytes / (1024 * 1024)).toStringAsFixed(1)} MB";
}
