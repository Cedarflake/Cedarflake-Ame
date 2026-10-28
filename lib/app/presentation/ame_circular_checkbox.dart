import "package:flutter/material.dart";

class AmeCircularCheckbox extends StatelessWidget {
  const AmeCircularCheckbox({
    required this.value,
    required this.onChanged,
    required this.diameter,
    this.semanticLabel,
    super.key,
  });

  final bool value;
  final ValueChanged<bool?>? onChanged;
  final double diameter;
  final String? semanticLabel;

  @override
  Widget build(BuildContext context) {
    final colors = Theme.of(context).colorScheme;
    // Checkbox paints an 18-pixel mark independently of its tap target.
    // Scale that mark and its framework input/semantics together to the diameter.
    return SizedBox.square(
      dimension: diameter,
      child: ClipOval(
        child: FittedBox(
          child: SizedBox.square(
            dimension: Checkbox.width,
            child: Checkbox(
              value: value,
              onChanged: onChanged,
              semanticLabel: semanticLabel,
              shape: const CircleBorder(),
              materialTapTargetSize: MaterialTapTargetSize.shrinkWrap,
              splashRadius: Checkbox.width / 2,
              fillColor: WidgetStateProperty.resolveWith((states) {
                if (states.contains(WidgetState.selected) ||
                    states.contains(WidgetState.disabled)) {
                  return null;
                }
                return colors.surface.withValues(alpha: 0.92);
              }),
              side: WidgetStateBorderSide.resolveWith((states) {
                if (states.contains(WidgetState.selected)) {
                  return BorderSide.none;
                }
                return BorderSide(
                  color: colors.outline,
                  width: 1.5 * Checkbox.width / diameter,
                );
              }),
            ),
          ),
        ),
      ),
    );
  }
}
