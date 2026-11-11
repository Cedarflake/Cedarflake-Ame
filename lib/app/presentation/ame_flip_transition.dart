import "dart:math" as math;

import "package:material_ui/material_ui.dart";

import "ame_motion.dart";

/// A decorative, two-sided flip inside a bounded slot. The parent owns input.
class AmeFlipTransition extends StatelessWidget {
  const AmeFlipTransition({
    required this.showBack,
    required this.front,
    required this.back,
    super.key,
  });

  final bool showBack;
  final Widget front;
  final Widget back;

  @override
  Widget build(BuildContext context) => ExcludeFocus(
    child: ExcludeSemantics(
      child: IgnorePointer(
        child: LayoutBuilder(
          builder: (context, constraints) {
            assert(constraints.hasBoundedWidth && constraints.hasBoundedHeight);
            final cameraDistance =
                math.max(constraints.maxWidth, constraints.maxHeight) * 8;
            return AmeStateTransition(
              active: showBack,
              motion: AmeMotion.fastSpatial,
              builder: (context, progress, _) => AnimatedBuilder(
                animation: progress,
                builder: (context, _) {
                  final value = progress.value.clamp(0.0, 1.0);
                  final backVisible = value >= 0.5;
                  final angle = math.pi * (backVisible ? value - 1 : value);
                  return Transform(
                    alignment: Alignment.center,
                    transformHitTests: false,
                    transform: Matrix4.identity()
                      ..setEntry(
                        3,
                        2,
                        cameraDistance > 0 ? 1 / cameraDistance : 0,
                      )
                      ..rotateY(angle),
                    child: IndexedStack(
                      index: backVisible ? 1 : 0,
                      alignment: Alignment.center,
                      sizing: StackFit.expand,
                      children: [front, back],
                    ),
                  );
                },
              ),
            );
          },
        ),
      ),
    ),
  );
}
