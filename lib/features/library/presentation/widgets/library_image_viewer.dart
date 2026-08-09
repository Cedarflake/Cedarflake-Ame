import "dart:math" as math;

import "package:flutter/gestures.dart";
import "package:flutter/material.dart";
import "package:flutter/services.dart";

import "../../../settings/application/ame_preferences.dart";
import "../../domain/library_models.dart";
import "library_viewer_controls.dart";
import "library_viewer_image.dart";

class LibraryImageViewer extends StatefulWidget {
  const LibraryImageViewer({
    required this.asset,
    required this.onBack,
    required this.onInformation,
    required this.onCopyPath,
    required this.onRevealFile,
    this.wheelBehavior = ImageViewerWheelBehavior.zoom,
    this.openBehavior = ImageViewerOpenBehavior.fitWindow,
    this.onPrevious,
    this.onNext,
    this.position,
    this.totalItems,
    super.key,
  });

  final LibraryAsset asset;
  final VoidCallback onBack;
  final VoidCallback onInformation;
  final VoidCallback onCopyPath;
  final VoidCallback onRevealFile;
  final ImageViewerWheelBehavior wheelBehavior;
  final ImageViewerOpenBehavior openBehavior;
  final VoidCallback? onPrevious;
  final VoidCallback? onNext;
  final int? position;
  final int? totalItems;

  @override
  State<LibraryImageViewer> createState() => _LibraryImageViewerState();
}

class _LibraryImageViewerState extends State<LibraryImageViewer>
    with SingleTickerProviderStateMixin {
  static const _defaultMinimumScale = 0.25;
  static const _defaultMaximumScale = 8.0;
  static const _zoomStep = 1.25;
  static const _zoomAnimationDuration = Duration(milliseconds: 180);

  final FocusNode _viewerFocusNode = FocusNode(
    debugLabel: "library-image-viewer",
  );
  final TransformationController _transformationController =
      TransformationController();
  late final AnimationController _zoomAnimationController;
  Matrix4? _zoomAnimationStart;
  Matrix4? _zoomAnimationEnd;
  Matrix4? _interactionStartTransform;
  Size _viewportSize = Size.zero;
  double _scale = 1;
  bool _isConstrainingTransform = false;
  bool _shouldApplyOpeningBehavior = true;
  DateTime? _lastWheelNavigationAt;

  @override
  void initState() {
    super.initState();
    _zoomAnimationController = AnimationController(
      vsync: this,
      duration: _zoomAnimationDuration,
    )..addListener(_handleZoomAnimation);
    _transformationController.addListener(_handleTransformationChanged);
    _requestViewerFocus();
  }

  @override
  void didUpdateWidget(covariant LibraryImageViewer oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.asset.locationId != widget.asset.locationId) {
      _fitToWindow(animate: false);
      _shouldApplyOpeningBehavior = true;
      _requestViewerFocus();
      WidgetsBinding.instance.addPostFrameCallback((_) {
        if (mounted) {
          _applyOpeningBehaviorIfNeeded();
        }
      });
    }
  }

  @override
  void dispose() {
    _viewerFocusNode.dispose();
    _zoomAnimationController.dispose();
    _transformationController
      ..removeListener(_handleTransformationChanged)
      ..dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return CallbackShortcuts(
      bindings: {
        const SingleActivator(LogicalKeyboardKey.escape): widget.onBack,
        const SingleActivator(LogicalKeyboardKey.backspace): widget.onBack,
        const SingleActivator(LogicalKeyboardKey.browserBack): widget.onBack,
        const SingleActivator(LogicalKeyboardKey.arrowLeft):
            _showPreviousFromKeyboard,
        const SingleActivator(LogicalKeyboardKey.arrowRight):
            _showNextFromKeyboard,
        const SingleActivator(LogicalKeyboardKey.add): _zoomIn,
        const SingleActivator(LogicalKeyboardKey.equal, shift: true): _zoomIn,
        const SingleActivator(LogicalKeyboardKey.minus): _zoomOut,
        const SingleActivator(LogicalKeyboardKey.numpadAdd): _zoomIn,
        const SingleActivator(LogicalKeyboardKey.numpadSubtract): _zoomOut,
        const SingleActivator(LogicalKeyboardKey.equal, control: true): _zoomIn,
        const SingleActivator(
          LogicalKeyboardKey.equal,
          control: true,
          shift: true,
        ): _zoomIn,
        const SingleActivator(LogicalKeyboardKey.minus, control: true):
            _zoomOut,
        const SingleActivator(LogicalKeyboardKey.digit0): _fitToWindow,
        const SingleActivator(LogicalKeyboardKey.digit0, control: true):
            _fitToWindow,
        const SingleActivator(LogicalKeyboardKey.digit1): _showActualSize,
        const SingleActivator(LogicalKeyboardKey.digit1, control: true):
            _showActualSize,
      },
      child: Focus(
        focusNode: _viewerFocusNode,
        autofocus: true,
        child: Column(
          children: [
            LibraryViewerTopBar(
              displayPath: widget.asset.displayPath,
              positionLabel: _positionLabel,
              onBack: widget.onBack,
              onInformation: widget.onInformation,
              onCopyPath: widget.onCopyPath,
              onRevealFile: widget.onRevealFile,
            ),
            const Divider(height: 1),
            Expanded(child: _buildCanvas(context)),
          ],
        ),
      ),
    );
  }

  Widget _buildCanvas(BuildContext context) {
    return ColoredBox(
      color: Theme.of(context).colorScheme.surfaceContainerLowest,
      child: LayoutBuilder(
        builder: (context, constraints) {
          final nextViewportSize = Size(
            constraints.maxWidth,
            math.max(
              0,
              constraints.maxHeight -
                  LibraryViewerZoomControls.commandBarHeight,
            ),
          );
          if (nextViewportSize != _viewportSize) {
            _viewportSize = nextViewportSize;
            WidgetsBinding.instance.addPostFrameCallback((_) {
              if (mounted) {
                _applyOpeningBehaviorIfNeeded();
                _constrainTransform();
              }
            });
          }
          return Column(
            children: [
              Expanded(child: _buildImageViewport()),
              LibraryViewerZoomControls(
                sliderValue: _sliderValueForScale(_scale),
                zoomPercent: _zoomPercent.round(),
                canZoomOut: _scale > _minimumScale + 0.001,
                canZoomIn: _scale < _maximumScale - 0.001,
                canShowActualSize: _hasImageDimensions,
                onSliderChanged: (value) =>
                    _setScale(_scaleForSliderValue(value), animate: false),
                sliderSemanticFormatter: (value) =>
                    "${_zoomPercentForScale(_scaleForSliderValue(value)).round()}%",
                onZoomOut: _zoomOut,
                onZoomIn: _zoomIn,
                onFitToWindow: _fitToWindow,
                onShowActualSize: _showActualSize,
              ),
            ],
          );
        },
      ),
    );
  }

  Widget _buildImageViewport() {
    return Stack(
      fit: StackFit.expand,
      children: [
        Listener(
          onPointerSignal: _handlePointerSignal,
          child: GestureDetector(
            behavior: HitTestBehavior.opaque,
            onDoubleTap: _toggleActualSize,
            child: InteractiveViewer(
              key: const Key("library-image-interactive-viewer"),
              transformationController: _transformationController,
              alignment: Alignment.topLeft,
              boundaryMargin: const EdgeInsets.all(double.infinity),
              minScale: _minimumScale,
              maxScale: _maximumScale,
              scaleFactor: 180,
              scaleEnabled:
                  widget.wheelBehavior == ImageViewerWheelBehavior.zoom,
              trackpadScrollCausesScale: true,
              onInteractionStart: _handleInteractionStart,
              child: SizedBox(
                width: _viewportSize.width,
                height: _viewportSize.height,
                child: LibraryViewerImage(asset: widget.asset),
              ),
            ),
          ),
        ),
        LibraryViewerNavigationButton.previous(onPressed: widget.onPrevious),
        LibraryViewerNavigationButton.next(onPressed: widget.onNext),
      ],
    );
  }

  void _handleTransformationChanged() {
    _constrainTransform();
    final nextScale = _transformationController.value
        .getMaxScaleOnAxis()
        .clamp(_minimumScale, _maximumScale)
        .toDouble();
    if (mounted && (nextScale - _scale).abs() > 0.001) {
      setState(() => _scale = nextScale);
    }
  }

  void _requestViewerFocus() {
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (mounted) {
        _viewerFocusNode.requestFocus();
      }
    });
  }

  void _constrainTransform() {
    if (_isConstrainingTransform ||
        !_hasImageDimensions ||
        _viewportSize.isEmpty) {
      return;
    }
    final matrix = _transformationController.value;
    final scale = matrix.getMaxScaleOnAxis();
    final translation = matrix.getTranslation();
    final fittedSize = Size(
      widget.asset.width * _fitImageScale,
      widget.asset.height * _fitImageScale,
    );
    final fittedLeft = (_viewportSize.width - fittedSize.width) / 2;
    final fittedTop = (_viewportSize.height - fittedSize.height) / 2;
    final imageWidth = fittedSize.width * scale;
    final imageHeight = fittedSize.height * scale;
    final imageLeft = translation.x + fittedLeft * scale;
    final imageTop = translation.y + fittedTop * scale;
    var correctedX = translation.x;
    var correctedY = translation.y;

    if (imageWidth <= _viewportSize.width) {
      correctedX += (_viewportSize.width - imageWidth) / 2 - imageLeft;
    } else if (imageLeft > 0) {
      correctedX -= imageLeft;
    } else if (imageLeft + imageWidth < _viewportSize.width) {
      correctedX += _viewportSize.width - imageLeft - imageWidth;
    }
    if (imageHeight <= _viewportSize.height) {
      correctedY += (_viewportSize.height - imageHeight) / 2 - imageTop;
    } else if (imageTop > 0) {
      correctedY -= imageTop;
    } else if (imageTop + imageHeight < _viewportSize.height) {
      correctedY += _viewportSize.height - imageTop - imageHeight;
    }

    if ((correctedX - translation.x).abs() < 0.01 &&
        (correctedY - translation.y).abs() < 0.01) {
      return;
    }
    _isConstrainingTransform = true;
    _transformationController.value = matrix.clone()
      ..setTranslationRaw(correctedX, correctedY, 0);
    _isConstrainingTransform = false;
  }

  void _fitToWindow({bool animate = true}) {
    _setTransform(Matrix4.identity(), animate: animate);
  }

  void _applyOpeningBehaviorIfNeeded() {
    if (!_shouldApplyOpeningBehavior || !_hasImageDimensions) {
      return;
    }
    _shouldApplyOpeningBehavior = false;
    _fitToWindow(animate: false);
    if (widget.openBehavior == ImageViewerOpenBehavior.actualSize) {
      _showActualSize(animate: false);
    }
  }

  void _handlePointerSignal(PointerSignalEvent event) {
    if (event is! PointerScrollEvent || event.scrollDelta.dy == 0) {
      return;
    }
    if (widget.wheelBehavior == ImageViewerWheelBehavior.zoom) {
      final animationStart = _interactionStartTransform;
      _interactionStartTransform = null;
      if (animationStart == null) {
        return;
      }
      final animationEnd = _transformationController.value.clone();
      _transformationController.value = animationStart;
      _setTransform(animationEnd, animate: true);
      return;
    }
    final now = DateTime.now();
    final lastNavigation = _lastWheelNavigationAt;
    if (lastNavigation != null &&
        now.difference(lastNavigation) < const Duration(milliseconds: 240)) {
      return;
    }
    _lastWheelNavigationAt = now;
    if (event.scrollDelta.dy < 0) {
      widget.onPrevious?.call();
    } else {
      widget.onNext?.call();
    }
  }

  void _handleInteractionStart(ScaleStartDetails _) {
    _zoomAnimationController.stop();
    _interactionStartTransform = _transformationController.value.clone();
  }

  void _showActualSize({bool animate = true}) {
    if (_hasImageDimensions) {
      _setScale(_actualSizeScale, animate: animate);
    }
  }

  void _toggleActualSize() {
    if (!_hasImageDimensions || (_zoomPercent - 100).abs() < 1) {
      _fitToWindow();
    } else {
      _showActualSize();
    }
  }

  void _zoomIn() => _setScale(_scale * _zoomStep);

  void _zoomOut() => _setScale(_scale / _zoomStep);

  void _setScale(double value, {bool animate = true}) {
    if (_viewportSize.isEmpty) {
      return;
    }
    final nextScale = value.clamp(_minimumScale, _maximumScale).toDouble();
    final viewportCenter = Offset(
      _viewportSize.width / 2,
      _viewportSize.height / 2,
    );
    final sceneCenter = _transformationController.toScene(viewportCenter);
    final target = Matrix4.identity()
      ..translateByDouble(viewportCenter.dx, viewportCenter.dy, 0, 1)
      ..scaleByDouble(nextScale, nextScale, nextScale, 1)
      ..translateByDouble(-sceneCenter.dx, -sceneCenter.dy, 0, 1);
    _setTransform(target, animate: animate);
  }

  void _setTransform(Matrix4 target, {required bool animate}) {
    _zoomAnimationController.stop();
    if (!animate) {
      _transformationController.value = target;
      return;
    }
    _zoomAnimationStart = _transformationController.value.clone();
    _zoomAnimationEnd = target;
    _zoomAnimationController.forward(from: 0);
  }

  void _handleZoomAnimation() {
    final start = _zoomAnimationStart;
    final end = _zoomAnimationEnd;
    if (start == null || end == null) {
      return;
    }
    final progress = Curves.easeOutCubic.transform(
      _zoomAnimationController.value,
    );
    final startTranslation = start.getTranslation();
    final endTranslation = end.getTranslation();
    final startScale = start.getMaxScaleOnAxis();
    final endScale = end.getMaxScaleOnAxis();
    final scale = _lerp(startScale, endScale, progress);
    _transformationController.value = Matrix4.identity()
      ..translateByDouble(
        _lerp(startTranslation.x, endTranslation.x, progress),
        _lerp(startTranslation.y, endTranslation.y, progress),
        0,
        1,
      )
      ..scaleByDouble(scale, scale, scale, 1);
  }

  double _lerp(double start, double end, double progress) =>
      start + (end - start) * progress;

  double _sliderValueForScale(double scale) {
    final bounded = scale.clamp(_minimumScale, _maximumScale).toDouble();
    return (math.log(bounded) - math.log(_minimumScale)) /
        (math.log(_maximumScale) - math.log(_minimumScale));
  }

  double _scaleForSliderValue(double value) {
    return math.exp(
      math.log(_minimumScale) +
          value * (math.log(_maximumScale) - math.log(_minimumScale)),
    );
  }

  double _zoomPercentForScale(double scale) => scale * _fitImageScale * 100;

  void _showPreviousFromKeyboard() => widget.onPrevious?.call();

  void _showNextFromKeyboard() => widget.onNext?.call();

  bool get _hasImageDimensions =>
      widget.asset.width > 0 &&
      widget.asset.height > 0 &&
      !_viewportSize.isEmpty;

  double get _fitImageScale {
    if (!_hasImageDimensions) {
      return 1;
    }
    return math.min(
      _viewportSize.width / widget.asset.width,
      _viewportSize.height / widget.asset.height,
    );
  }

  double get _actualSizeScale => 1 / _fitImageScale;

  double get _minimumScale => !_hasImageDimensions
      ? _defaultMinimumScale
      : math
            .min(_defaultMinimumScale, _actualSizeScale / 4)
            .clamp(0.01, _defaultMinimumScale);

  double get _maximumScale => !_hasImageDimensions
      ? _defaultMaximumScale
      : math
            .max(_defaultMaximumScale, _actualSizeScale * 4)
            .clamp(_defaultMaximumScale, 1024);

  double get _zoomPercent => _zoomPercentForScale(_scale);

  String? get _positionLabel {
    final position = widget.position;
    final totalItems = widget.totalItems;
    if (position == null || totalItems == null || totalItems <= 0) {
      return null;
    }
    return "$position / $totalItems";
  }
}
