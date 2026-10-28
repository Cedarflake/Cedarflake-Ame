import "package:flutter/material.dart";
import "package:material_symbols_icons/symbols.dart";

class AmeSelectionCheckbox extends StatelessWidget {
  const AmeSelectionCheckbox({
    required this.value,
    required this.onChanged,
    required this.markExtent,
    this.tapTargetSize = 40,
    this.semanticLabel,
    super.key,
  });

  final bool value;
  final ValueChanged<bool?>? onChanged;
  final double markExtent;
  final double tapTargetSize;
  final String? semanticLabel;

  @override
  Widget build(BuildContext context) {
    final colors = Theme.of(context).colorScheme;
    final scale = markExtent / Checkbox.width;
    // Retain Checkbox's input, focus and checked semantics while using the
    // shared symbol for its mark. The larger tap target has no background.
    return SizedBox.square(
      dimension: tapTargetSize,
      child: ClipRect(
        child: Stack(
          fit: StackFit.expand,
          alignment: Alignment.center,
          children: [
            FittedBox(
              child: SizedBox.square(
                dimension: tapTargetSize / scale,
                child: Checkbox(
                  value: value,
                  onChanged: onChanged,
                  semanticLabel: semanticLabel,
                  shape: RoundedRectangleBorder(
                    borderRadius: BorderRadius.circular(4 / scale),
                  ),
                  materialTapTargetSize: MaterialTapTargetSize.shrinkWrap,
                  checkColor: Colors.transparent,
                  splashRadius: tapTargetSize / scale / 2,
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
                      width: 1.5 / scale,
                    );
                  }),
                ),
              ),
            ),
            if (value)
              IgnorePointer(
                child: Icon(
                  Symbols.check_rounded,
                  size: markExtent * 0.7,
                  color: onChanged == null ? colors.surface : colors.onPrimary,
                ),
              ),
          ],
        ),
      ),
    );
  }
}
