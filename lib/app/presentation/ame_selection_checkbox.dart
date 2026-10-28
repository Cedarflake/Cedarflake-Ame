import "package:flutter/material.dart";
import "package:material_symbols_icons/symbols.dart";

class AmeSelectionCheckbox extends StatelessWidget {
  const AmeSelectionCheckbox({
    required this.value,
    required this.onChanged,
    required this.markExtent,
    this.tapTargetSize = 40,
    this.tristate = false,
    this.isOutlined = false,
    this.shape = const CircleBorder(),
    this.semanticLabel,
    super.key,
  });

  final bool? value;
  final bool tristate;
  final bool isOutlined;
  final OutlinedBorder shape;
  final ValueChanged<bool?>? onChanged;
  final double markExtent;
  final double tapTargetSize;
  final String? semanticLabel;

  @override
  Widget build(BuildContext context) {
    final colors = Theme.of(context).colorScheme;
    var markColor = onChanged == null ? colors.surface : colors.onPrimary;
    if (isOutlined) markColor = colors.onSurfaceVariant;
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
                  tristate: tristate,
                  onChanged: onChanged,
                  semanticLabel: semanticLabel,
                  shape: shape,
                  materialTapTargetSize: MaterialTapTargetSize.shrinkWrap,
                  checkColor: Colors.transparent,
                  splashRadius: tapTargetSize / scale / 2,
                  fillColor: WidgetStateProperty.resolveWith((states) {
                    if (isOutlined) return Colors.transparent;
                    if (states.contains(WidgetState.selected) ||
                        states.contains(WidgetState.disabled)) {
                      return null;
                    }
                    return colors.surface.withValues(alpha: 0.92);
                  }),
                  side: WidgetStateBorderSide.resolveWith((states) {
                    if (!isOutlined && states.contains(WidgetState.selected)) {
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
            if (value != false)
              IgnorePointer(
                child: Icon(
                  value == null
                      ? Symbols.remove_rounded
                      : Symbols.check_rounded,
                  size: markExtent * 0.75,
                  color: markColor,
                ),
              ),
          ],
        ),
      ),
    );
  }
}
