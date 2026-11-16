import "package:material_ui/material_ui.dart";
import "package:material_symbols_icons/symbols.dart";

import "../../../../app/presentation/ame_overlay_semantics.dart";
import "../../../../app/presentation/ame_typography.dart";

class LibrarySidebarPrimaryAction extends StatelessWidget {
  const LibrarySidebarPrimaryAction({
    required this.label,
    required this.buttonKey,
    required this.onPressed,
    this.disabledReason,
    super.key,
  });

  final String label;
  final Key buttonKey;
  final VoidCallback? onPressed;
  final String? disabledReason;

  @override
  Widget build(BuildContext context) {
    final colors = Theme.of(context).colorScheme;
    return Padding(
      padding: const EdgeInsets.only(top: 4, bottom: 32),
      child: Align(
        alignment: AlignmentDirectional.centerStart,
        child: AmeTooltip(
          message: disabledReason ?? label,
          child: FilledButton.icon(
            key: buttonKey,
            onPressed: onPressed,
            style: FilledButton.styleFrom(
              backgroundColor: colors.secondaryContainer,
              foregroundColor: colors.onSecondaryContainer,
              textStyle: Theme.of(context).textTheme.labelLarge?.copyWith(
                fontWeight: ameFontWeightSemibold,
              ),
              minimumSize: const Size(0, 56),
              padding: const EdgeInsetsDirectional.only(start: 16, end: 20),
              shape: RoundedRectangleBorder(
                borderRadius: BorderRadius.circular(12),
              ),
            ),
            icon: const Icon(Symbols.add_rounded, size: 24, weight: 700),
            label: Text(label),
          ),
        ),
      ),
    );
  }
}
