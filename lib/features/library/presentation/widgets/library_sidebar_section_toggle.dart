import "package:material_ui/material_ui.dart";
import "package:material_symbols_icons/symbols.dart";

import "../../../../app/presentation/ame_localizations.dart";
import "../../../../app/presentation/ame_transition_icon.dart";

class LibrarySidebarSectionToggle extends StatelessWidget {
  const LibrarySidebarSectionToggle({
    required this.label,
    required this.expanded,
    required this.onPressed,
    super.key,
  });

  final String label;
  final bool expanded;
  final VoidCallback onPressed;

  @override
  Widget build(BuildContext context) => Semantics(
    expanded: expanded,
    child: IconButton(
      tooltip: expanded
          ? context.l10n.collapseSection(label)
          : context.l10n.expandSection(label),
      onPressed: onPressed,
      style: IconButton.styleFrom(
        fixedSize: const Size.square(36),
        iconSize: 18,
        padding: const EdgeInsets.all(8),
        tapTargetSize: MaterialTapTargetSize.shrinkWrap,
      ),
      icon: AmeCrossFadeIcon(
        active: expanded,
        inactiveIcon: Symbols.unfold_more_rounded,
        activeIcon: Symbols.unfold_less_rounded,
      ),
    ),
  );
}
