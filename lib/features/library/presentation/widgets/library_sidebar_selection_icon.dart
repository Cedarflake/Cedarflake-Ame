import "package:flutter/material.dart";
import "package:material_symbols_icons/symbols.dart";

import "../../../../app/presentation/ame_overlay_semantics.dart";

class LibrarySidebarSelectionIcon extends StatelessWidget {
  const LibrarySidebarSelectionIcon({
    required this.icon,
    required this.name,
    required this.selected,
    required this.onPressed,
    super.key,
  });

  final IconData icon;
  final String name;
  final bool selected;
  final VoidCallback? onPressed;

  @override
  Widget build(BuildContext context) {
    final colors = Theme.of(context).colorScheme;
    return AmeTooltip(
      message: selected ? "取消选择$name" : "选择$name",
      child: IconButton(
        isSelected: selected,
        onPressed: onPressed,
        style: IconButton.styleFrom(
          minimumSize: const Size.square(40),
          maximumSize: const Size.square(40),
          padding: const EdgeInsets.all(8),
        ),
        icon: Icon(icon, size: 24),
        selectedIcon: DecoratedBox(
          decoration: BoxDecoration(
            color: colors.primary,
            shape: BoxShape.circle,
          ),
          child: SizedBox.square(
            dimension: 24,
            child: Icon(
              Symbols.check_rounded,
              size: 18,
              color: colors.onPrimary,
            ),
          ),
        ),
      ),
    );
  }
}
