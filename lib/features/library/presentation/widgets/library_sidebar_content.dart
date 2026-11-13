import "package:material_ui/material_ui.dart";
import "package:flutter/rendering.dart";

class LibrarySidebarMotion extends InheritedWidget {
  const LibrarySidebarMotion({
    required this.progress,
    required this.sidebarWidth,
    required this.expanded,
    required super.child,
    super.key,
  });

  final Animation<double> progress;
  final double sidebarWidth;
  final bool expanded;

  double get leadingOffset =>
      sidebarWidth * (progress.value - (expanded ? 1 : 0));

  @override
  bool updateShouldNotify(LibrarySidebarMotion oldWidget) =>
      progress != oldWidget.progress ||
      sidebarWidth != oldWidget.sidebarWidth ||
      expanded != oldWidget.expanded;
}

class LibrarySidebarLeading extends StatelessWidget {
  const LibrarySidebarLeading({required this.child, super.key});

  final Widget child;

  @override
  Widget build(BuildContext context) {
    final motion = context
        .dependOnInheritedWidgetOfExactType<LibrarySidebarMotion>();
    return AnimatedBuilder(
      animation: motion?.progress ?? const AlwaysStoppedAnimation(1.0),
      child: child,
      builder: (context, child) => Transform.translate(
        offset: Offset(motion?.leadingOffset ?? 0, 0),
        child: child,
      ),
    );
  }
}

/// Moves content with the sidebar while retaining its destination layout width.
class LibrarySidebarContent extends StatelessWidget {
  const LibrarySidebarContent({
    required this.child,
    this.alignment = Alignment.topLeft,
    super.key,
  });

  final Widget child;
  final Alignment alignment;

  @override
  Widget build(BuildContext context) {
    final motion = context
        .dependOnInheritedWidgetOfExactType<LibrarySidebarMotion>();
    return SizedBox(
      width: double.infinity,
      child: _SidebarContentViewport(
        motion: motion,
        alignment: alignment,
        child: child,
      ),
    );
  }
}

class _SidebarContentViewport extends SingleChildRenderObjectWidget {
  const _SidebarContentViewport({
    required this.motion,
    required this.alignment,
    required super.child,
  });

  final LibrarySidebarMotion? motion;
  final Alignment alignment;

  @override
  RenderObject createRenderObject(BuildContext context) =>
      _RenderSidebarContentViewport(motion, alignment);

  @override
  void updateRenderObject(
    BuildContext context,
    covariant _RenderSidebarContentViewport renderObject,
  ) => renderObject.update(motion, alignment);
}

class _RenderSidebarContentViewport extends RenderProxyBox {
  _RenderSidebarContentViewport(this._motion, this._alignment);

  LibrarySidebarMotion? _motion;
  Alignment _alignment;

  double get _leading =>
      (_motion?.leadingOffset ?? 0).clamp(double.negativeInfinity, size.width);
  Offset get _offset => Offset(_leading * (1 - _alignment.x) / 2, 0);
  Rect get _clip => Rect.fromLTRB(_leading, 0, size.width, size.height);

  void update(LibrarySidebarMotion? motion, Alignment alignment) {
    if (_motion == motion && _alignment == alignment) return;
    if (attached) _motion?.progress.removeListener(_repaint);
    _motion = motion;
    _alignment = alignment;
    if (attached) _motion?.progress.addListener(_repaint);
    _repaint();
  }

  void _repaint() {
    markNeedsPaint();
    markNeedsSemanticsUpdate();
  }

  @override
  void attach(PipelineOwner owner) {
    super.attach(owner);
    _motion?.progress.addListener(_repaint);
  }

  @override
  void detach() {
    _motion?.progress.removeListener(_repaint);
    super.detach();
  }

  @override
  void paint(PaintingContext context, Offset offset) {
    layer = context.pushClipRect(
      needsCompositing,
      offset,
      _clip,
      (context, offset) => super.paint(context, offset + _offset),
      oldLayer: layer as ClipRectLayer?,
    );
  }

  @override
  bool hitTestChildren(BoxHitTestResult result, {required Offset position}) =>
      _clip.contains(position) &&
      result.addWithPaintOffset(
        offset: _offset,
        position: position,
        hitTest: (result, position) =>
            super.hitTestChildren(result, position: position),
      );

  @override
  void applyPaintTransform(RenderBox child, Matrix4 transform) {
    transform.translateByDouble(_offset.dx, _offset.dy, 0, 1);
  }

  @override
  Rect? describeApproximatePaintClip(RenderObject child) => _clip;
}
