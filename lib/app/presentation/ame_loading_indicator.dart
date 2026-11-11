// Copyright 2013 The Flutter Authors
// BSD-3-Clause license: assets/licenses/material_loading_indicator.txt.
// Adapted from flutter/packages PR 13429, commit 9eaa17df4e0ba4b067a7203e857953e3d4bc04bd.
// Retains its indeterminate shapes, morph timing and painting; Ame owns visibility
// and adds reduced-motion and inactive-surface retirement.

import "dart:math" as math;
import "dart:ui" show SemanticsRole;

import "package:flutter/foundation.dart";
import "package:flutter/physics.dart";
import "package:flutter/scheduler.dart";
import "package:material_ui/material_ui.dart";

/// The temporary renderer behind Ame's loading feedback; it owns no request state.
class AmeLoadingIndicator extends StatefulWidget {
  const AmeLoadingIndicator({
    this.size = 48,
    this.semanticsLabel = "正在加载",
    super.key,
  });

  final double size;
  final String semanticsLabel;

  @override
  State<AmeLoadingIndicator> createState() => _AmeLoadingIndicatorState();
}

class _AmeLoadingIndicatorState extends State<AmeLoadingIndicator>
    with SingleTickerProviderStateMixin {
  final _elapsed = ValueNotifier(Duration.zero);
  Duration _startedAt = Duration.zero;
  late final Ticker _ticker = createTicker(
    (elapsed) => _elapsed.value = _startedAt + elapsed,
  );

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    final animate =
        TickerMode.valuesOf(context).enabled &&
        !MediaQuery.disableAnimationsOf(context);
    if (animate && !_ticker.isActive) {
      _startedAt = _elapsed.value;
      _ticker.start();
    } else if (!animate && _ticker.isActive) {
      _ticker.stop();
    }
  }

  @override
  Widget build(BuildContext context) => Semantics(
    label: widget.semanticsLabel,
    role: SemanticsRole.loadingSpinner,
    child: SizedBox.square(
      dimension: widget.size,
      child: RepaintBoundary(
        child: CustomPaint(
          painter: _LoadingIndicatorPainter(
            elapsed: _elapsed,
            color: Theme.of(context).colorScheme.primary,
          ),
        ),
      ),
    ),
  );

  @override
  void dispose() {
    _ticker.dispose();
    _elapsed.dispose();
    super.dispose();
  }
}

class _LoadingIndicatorPainter extends CustomPainter {
  _LoadingIndicatorPainter({required this.elapsed, required this.color})
    : super(repaint: elapsed);

  final ValueListenable<Duration> elapsed;
  final Color color;

  static const _morphInterval = Duration(milliseconds: 650);
  static const _rotationDuration = Duration(milliseconds: 4666);
  static final _spring = SpringSimulation(
    SpringDescription.withDampingRatio(mass: 1, stiffness: 200, ratio: 0.6),
    0,
    1,
    0,
    snapToEnd: true,
  );
  static final _polygons = [
    MaterialShapes.softBurst,
    MaterialShapes.cookie9Sided,
    MaterialShapes.pentagon,
    MaterialShapes.pill,
    MaterialShapes.sunny,
    MaterialShapes.cookie4Sided,
    MaterialShapes.oval,
  ];

  // The public shape-border API owns and caches the package's internal morphs.
  static final _shapes = [
    for (final polygon in _polygons)
      MaterialShapeBorder(shape: polygon.normalized()),
  ];

  @override
  void paint(Canvas canvas, Size size) {
    final elapsedUs = elapsed.value.inMicroseconds;
    final intervalUs = _morphInterval.inMicroseconds;
    final rotationUs = _rotationDuration.inMicroseconds;
    final completed = elapsedUs ~/ intervalUs;
    final progress = _spring.x(
      (elapsedUs % intervalUs) / Duration.microsecondsPerSecond,
    );
    final turns =
        (elapsedUs % rotationUs) / rotationUs + (completed % 4 + progress) / 4;
    final shape = ShapeBorder.lerp(
      _shapes[completed % _shapes.length],
      _shapes[(completed + 1) % _shapes.length],
      progress,
    )!;
    final path = shape.getOuterPath(const Rect.fromLTWH(0, 0, 1, 1));
    // Center each painted path to keep asymmetric shapes from wobbling.
    final pathCenter = path.getBounds().center;
    final center = size.center(Offset.zero);
    canvas
      ..save()
      ..translate(center.dx, center.dy)
      ..rotate(2 * math.pi * turns)
      ..scale(size.shortestSide * 38 / 48)
      ..translate(-pathCenter.dx, -pathCenter.dy)
      ..drawPath(path, Paint()..color = color)
      ..restore();
  }

  @override
  bool shouldRepaint(_LoadingIndicatorPainter oldDelegate) =>
      oldDelegate.elapsed != elapsed || oldDelegate.color != color;
}
