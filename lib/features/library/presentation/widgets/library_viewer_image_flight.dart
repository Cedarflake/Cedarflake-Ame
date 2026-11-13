import "dart:math" as math;

import "package:flutter/rendering.dart";
import "package:material_ui/material_ui.dart";

import "library_viewer_launch.dart";

/// Paints the existing image from its thumbnail bounds without another decode.
class LibraryViewerImageFlight extends SingleChildRenderObjectWidget {
  const LibraryViewerImageFlight({
    required this.locationId,
    this.entryId,
    required this.imageSize,
    required super.child,
    this.quarterTurns = 0,
    super.key,
  });

  final String locationId;
  final String? entryId;
  final Size imageSize;
  final int quarterTurns;

  bool _canFly(BuildContext context) =>
      !MediaQuery.disableAnimationsOf(context) && quarterTurns == 0;

  @override
  RenderObject createRenderObject(BuildContext context) =>
      RenderLibraryViewerImageFlight(
        LibraryViewerFlightScope.of(context)?.origin,
        LibraryViewerFlightScope.of(context)?.animation,
        imageSize,
        entryId ?? locationId,
        _canFly(context),
      );

  @override
  void updateRenderObject(
    BuildContext context,
    covariant RenderLibraryViewerImageFlight renderObject,
  ) => renderObject.update(
    LibraryViewerFlightScope.of(context)?.origin,
    LibraryViewerFlightScope.of(context)?.animation,
    imageSize,
    entryId ?? locationId,
    _canFly(context),
  );
}

class RenderLibraryViewerImageFlight extends RenderProxyBox {
  RenderLibraryViewerImageFlight(
    this._origin,
    this._animation,
    this._imageSize,
    this._locationId,
    this._canFly,
  );

  LibraryViewerOrigin? _origin;
  Animation<double>? _animation;
  Size _imageSize;
  String _locationId;
  bool _canFly;
  bool _returning = false;
  LibraryViewerOrigin? _returnOrigin;
  final _roundedClipLayer = LayerHandle<ClipRRectLayer>();
  final _transformLayer = LayerHandle<TransformLayer>();

  @override
  bool get alwaysNeedsCompositing => child != null;

  void update(
    LibraryViewerOrigin? origin,
    Animation<double>? animation,
    Size imageSize,
    String locationId,
    bool canFly,
  ) {
    if (_animation != animation) {
      if (attached) _animation?.removeListener(markNeedsPaint);
      _animation = animation;
      if (attached) _animation?.addListener(markNeedsPaint);
    }
    if (_origin != origin || _locationId != locationId) {
      _origin = origin;
      _locationId = locationId;
      _returning = false;
      _returnOrigin = null;
    }
    _imageSize = imageSize;
    _canFly = canFly;
    markNeedsPaint();
  }

  @override
  void attach(PipelineOwner owner) {
    super.attach(owner);
    _animation?.addListener(markNeedsPaint);
  }

  @override
  void detach() {
    _animation?.removeListener(markNeedsPaint);
    super.detach();
  }

  LibraryViewerOrigin? _targetOrigin() {
    final origin = _origin;
    final animation = _animation;
    if (origin == null || animation == null || !_canFly) {
      return null;
    }
    final returning = animation.status == AnimationStatus.reverse;
    if (returning != _returning) {
      _returning = returning;
      _returnOrigin = returning ? origin.findTarget?.call(_locationId) : null;
      if (returning && _returnOrigin == null && origin.entryId == _locationId) {
        _returnOrigin = origin.resolve();
      }
    }
    final targetOrigin = returning ? _returnOrigin : origin;
    if (targetOrigin == null || targetOrigin.entryId != _locationId) {
      return null;
    }
    return targetOrigin;
  }

  ({Matrix4 transform, RRect clip, Rect visibleClip})? _frame(
    LibraryViewerOrigin? targetOrigin,
  ) {
    final animation = _animation;
    if (targetOrigin == null ||
        animation == null ||
        _imageSize.isEmpty ||
        !hasSize) {
      return null;
    }
    final geometry = targetOrigin.geometry;
    final progress = animation.value.clamp(0.0, 1.0);
    if (progress == 1 || size.isEmpty) return null;
    final inverse = Matrix4.tryInvert(getTransformTo(null));
    if (inverse == null) return null;
    final sourceBox = MatrixUtils.transformRect(inverse, geometry.bounds);
    final visibleSource = MatrixUtils.transformRect(
      inverse,
      geometry.visibleBounds,
    );
    final source = Alignment.center.inscribe(
      applyBoxFit(targetOrigin.fit, _imageSize, sourceBox.size).destination,
      sourceBox,
    );
    final target = Alignment.center.inscribe(
      applyBoxFit(BoxFit.contain, _imageSize, size).destination,
      Offset.zero & size,
    );
    if (source.isEmpty || target.isEmpty) return null;
    final initialScale = math.max(
      source.width / target.width,
      source.height / target.height,
    );
    final scale = initialScale + (1 - initialScale) * progress;
    final center = Offset.lerp(source.center, target.center, progress)!;
    final transform = Matrix4.identity()
      ..translateByDouble(center.dx, center.dy, 0, 1)
      ..scaleByDouble(scale, scale, 1, 1)
      ..translateByDouble(-target.center.dx, -target.center.dy, 0, 1);
    return (
      transform: transform,
      visibleClip: Rect.lerp(visibleSource, target, progress)!,
      clip: RRect.lerp(
        RRect.fromRectAndRadius(
          sourceBox,
          Radius.elliptical(
            targetOrigin.radius * sourceBox.width / geometry.bounds.width,
            targetOrigin.radius * sourceBox.height / geometry.bounds.height,
          ),
        ),
        RRect.fromRectAndRadius(target, Radius.zero),
        progress,
      )!,
    );
  }

  @override
  void paint(PaintingContext context, Offset offset) {
    final target = _targetOrigin();
    final frame = _frame(target);
    if (frame == null) {
      _roundedClipLayer.layer = null;
      _transformLayer.layer = null;
      if (_origin != null && target == null && _animation != null) {
        layer = context.pushOpacity(
          offset,
          (255 * _animation!.value.clamp(0.0, 1.0)).round(),
          super.paint,
          oldLayer: layer is OpacityLayer ? layer! as OpacityLayer : null,
        );
      } else {
        layer = null;
        super.paint(context, offset);
      }
      return;
    }
    layer = context.pushClipRect(needsCompositing, offset, frame.visibleClip, (
      context,
      offset,
    ) {
      _roundedClipLayer.layer = context.pushClipRRect(
        needsCompositing,
        offset,
        frame.clip.outerRect,
        frame.clip,
        (context, offset) {
          _transformLayer.layer = context.pushTransform(
            needsCompositing,
            offset,
            frame.transform,
            super.paint,
            oldLayer: _transformLayer.layer,
          );
        },
        oldLayer: _roundedClipLayer.layer,
      );
    }, oldLayer: layer is ClipRectLayer ? layer! as ClipRectLayer : null);
  }

  @override
  void dispose() {
    _roundedClipLayer.layer = null;
    _transformLayer.layer = null;
    super.dispose();
  }

  @override
  void applyPaintTransform(RenderBox child, Matrix4 transform) {
    final frame = _frame(_targetOrigin());
    if (frame != null) transform.multiply(frame.transform);
  }
}
