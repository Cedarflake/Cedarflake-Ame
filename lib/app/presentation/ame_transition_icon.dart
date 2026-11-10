import "package:material_ui/material_ui.dart";
import "package:material_symbols_icons/symbols.dart";

import "ame_motion.dart";

class AmeExpansionIcon extends StatelessWidget {
  const AmeExpansionIcon({
    required this.expanded,
    this.icon = Symbols.arrow_drop_down_rounded,
    super.key,
  });

  final bool expanded;
  final IconData icon;

  @override
  Widget build(BuildContext context) => AmeStateTransition(
    active: expanded,
    motion: AmeMotion.fastSpatial,
    child: Icon(icon),
    builder: (context, progress, child) => RotationTransition(
      turns: progress.drive(Tween<double>(begin: 0, end: 0.5)),
      child: child,
    ),
  );
}

class AmeCrossFadeIcon extends StatelessWidget {
  const AmeCrossFadeIcon({
    required this.active,
    required this.inactiveIcon,
    required this.activeIcon,
    super.key,
  });

  final bool active;
  final IconData inactiveIcon;
  final IconData activeIcon;

  @override
  Widget build(BuildContext context) => AmeStateTransition(
    active: active,
    motion: AmeMotion.fastEffects,
    builder: (context, progress, _) => Stack(
      alignment: Alignment.center,
      children: [
        FadeTransition(
          opacity: ReverseAnimation(progress),
          child: Icon(inactiveIcon),
        ),
        FadeTransition(opacity: progress, child: Icon(activeIcon)),
      ],
    ),
  );
}
