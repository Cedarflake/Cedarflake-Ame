import "dart:math" as math;

import "package:flutter/material.dart";

import "../../settings/application/ame_preferences.dart";
import "widgets/library_viewer_zoom_geometry.dart";

/// Owns zoom intent as well as its transform, so resizing preserves actual pixel scale.
class LibraryViewerZoom extends ChangeNotifier {
  LibraryViewerZoom({required TickerProvider vsync}) {
    _animation = AnimationController(
      vsync: vsync,
      duration: const Duration(milliseconds: 180),
    )..addListener(_animate);
    transform.addListener(_transformed);
  }

  final transform = TransformationController();
  late final AnimationController _animation;
  Matrix4? _animationStart;
  Matrix4? _animationEnd;
  Matrix4? _interactionStart;
  Object? _image;
  Size _imageSize = Size.zero;
  Size _viewport = Size.zero;
  ImageViewerOpenBehavior? _behavior = ImageViewerOpenBehavior.automatic;
  double _scale = 1;
  bool _isConstraining = false;
  bool _isUpdatingLayout = false;

  LibraryViewerZoomGeometry get _geometry =>
      LibraryViewerZoomGeometry(imageSize: _imageSize, viewportSize: _viewport);
  Size get viewport => _viewport;
  double get minimumScale => _geometry.minimumScale;
  double get maximumScale => _geometry.maximumScale;
  bool get hasDimensions => _geometry.hasImageDimensions;
  bool get isActualSize => _geometry.isActualSize(_scale);
  bool get canZoomOut => _scale > minimumScale + 0.001;
  bool get canZoomIn => _scale < maximumScale - 0.001;
  double get sliderValue => _geometry.sliderValueForScale(_scale);
  int get percent => _geometry.percentageForScale(_scale).round();

  String sliderLabel(double value) =>
      "${_geometry.percentageForScale(_geometry.scaleForSliderValue(value)).round()}%";

  void layout({
    required Object image,
    required Size imageSize,
    required Size viewport,
    required ImageViewerOpenBehavior opening,
  }) {
    if (_image == image && _imageSize == imageSize && _viewport == viewport) {
      return;
    }
    final previousPixelScale = _geometry.fitImageScale * _scale;
    final changedImage = _image != image;
    _image = image;
    _imageSize = imageSize;
    _viewport = viewport;
    if (changedImage) _behavior = opening;
    _isUpdatingLayout = true;
    try {
      final behavior = _behavior;
      if (behavior != null) {
        _setScale(
          _geometry.openingScale(behavior),
          animate: false,
          center: true,
        );
      } else {
        _setScale(previousPixelScale / _geometry.fitImageScale, animate: false);
      }
      _constrain();
    } finally {
      _isUpdatingLayout = false;
    }
  }

  void fit() {
    _behavior = ImageViewerOpenBehavior.fitWindow;
    _setScale(1, center: true);
  }

  void actualSize() {
    if (!hasDimensions) return;
    _behavior = ImageViewerOpenBehavior.actualSize;
    _setScale(_geometry.actualSizeScale, center: true);
  }

  void toggleMode() {
    if (isActualSize) {
      fit();
    } else {
      actualSize();
    }
  }

  void zoomIn() => _manualScale(_scale * 1.25);
  void zoomOut() => _manualScale(_scale / 1.25);
  void sliderChanged(double value) =>
      _manualScale(_geometry.scaleForSliderValue(value), animate: false);

  void _manualScale(double value, {bool animate = true}) {
    _behavior = null;
    _setScale(value, animate: animate);
  }

  void interactionStarted(ScaleStartDetails _) {
    _animation.stop();
    _behavior = null;
    _interactionStart = transform.value.clone();
  }

  void animatePointerZoom() {
    final start = _interactionStart;
    _interactionStart = null;
    if (start == null) return;
    final end = transform.value.clone();
    transform.value = start;
    _setTransform(end, animate: true);
  }

  void _setScale(double value, {bool animate = true, bool center = false}) {
    if (_viewport.isEmpty) return;
    final next = value.clamp(minimumScale, maximumScale).toDouble();
    final middle = Offset(_viewport.width / 2, _viewport.height / 2);
    final scene = center ? middle : transform.toScene(middle);
    _setTransform(
      Matrix4.identity()
        ..translateByDouble(middle.dx, middle.dy, 0, 1)
        ..scaleByDouble(next, next, next, 1)
        ..translateByDouble(-scene.dx, -scene.dy, 0, 1),
      animate: animate,
    );
  }

  void _setTransform(Matrix4 value, {required bool animate}) {
    _animation.stop();
    if (!animate) {
      transform.value = value;
      return;
    }
    _animationStart = transform.value.clone();
    _animationEnd = value;
    _animation.forward(from: 0);
  }

  void _animate() {
    final start = _animationStart;
    final end = _animationEnd;
    if (start == null || end == null) return;
    final progress = Curves.easeOutCubic.transform(_animation.value);
    final startPosition = start.getTranslation();
    final endPosition = end.getTranslation();
    double blend(double a, double b) => a + (b - a) * progress;
    final scale = blend(start.getMaxScaleOnAxis(), end.getMaxScaleOnAxis());
    transform.value = Matrix4.identity()
      ..translateByDouble(
        blend(startPosition.x, endPosition.x),
        blend(startPosition.y, endPosition.y),
        0,
        1,
      )
      ..scaleByDouble(scale, scale, scale, 1);
  }

  void _transformed() {
    _constrain();
    final next = transform.value
        .getMaxScaleOnAxis()
        .clamp(minimumScale, maximumScale)
        .toDouble();
    if ((next - _scale).abs() > 0.001) {
      _scale = next;
      if (!_isUpdatingLayout) notifyListeners();
    }
  }

  void _constrain() {
    if (_isConstraining || !hasDimensions || _viewport.isEmpty) return;
    final matrix = transform.value;
    final scale = matrix.getMaxScaleOnAxis();
    final position = matrix.getTranslation();
    final fitted = _imageSize * _geometry.fitImageScale;
    final left = position.x + (_viewport.width - fitted.width) / 2 * scale;
    final top = position.y + (_viewport.height - fitted.height) / 2 * scale;
    final width = fitted.width * scale;
    final height = fitted.height * scale;
    double correction(double start, double size, double available) {
      if (size <= available) return (available - size) / 2 - start;
      if (start > 0) return -start;
      return math.max(0, available - start - size);
    }

    final x = position.x + correction(left, width, _viewport.width);
    final y = position.y + correction(top, height, _viewport.height);
    if ((x - position.x).abs() < 0.01 && (y - position.y).abs() < 0.01) return;
    _isConstraining = true;
    transform.value = matrix.clone()..setTranslationRaw(x, y, 0);
    _isConstraining = false;
  }

  @override
  void dispose() {
    _animation.dispose();
    transform.removeListener(_transformed);
    transform.dispose();
    super.dispose();
  }
}
