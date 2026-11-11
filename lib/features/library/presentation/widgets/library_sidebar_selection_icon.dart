import "package:material_ui/material_ui.dart";
import "package:material_symbols_icons/symbols.dart";

import "../../../../app/presentation/ame_overlay_semantics.dart";
import "../../../../app/presentation/ame_flip_transition.dart";

class LibrarySidebarSelectionIcon extends StatelessWidget {
  const LibrarySidebarSelectionIcon({
    required this.icon,
    required this.name,
    required this.selected,
    required this.onPressed,
    super.key,
  });

  final Widget icon;
  final String name;
  final bool selected;
  final VoidCallback? onPressed;

  @override
  Widget build(BuildContext context) {
    final colors = Theme.of(context).colorScheme;
    return Semantics(
      container: true,
      child: AmeTooltip(
        message: selected ? "取消选择$name" : "选择$name",
        child: IconButton(
          isSelected: selected,
          onPressed: onPressed,
          style: IconButton.styleFrom(
            minimumSize: const Size.square(40),
            maximumSize: const Size.square(40),
            padding: const EdgeInsets.all(8),
          ),
          icon: SizedBox.square(
            dimension: 24,
            child: AmeFlipTransition(
              showBack: selected,
              front: IconTheme.merge(
                data: const IconThemeData(size: 24),
                child: icon,
              ),
              back: DecoratedBox(
                decoration: BoxDecoration(
                  color: colors.primary,
                  shape: BoxShape.circle,
                ),
                child: Icon(
                  Symbols.check_rounded,
                  size: 18,
                  color: colors.onPrimary,
                ),
              ),
            ),
          ),
        ),
      ),
    );
  }
}
