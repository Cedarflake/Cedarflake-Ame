import "package:flutter/material.dart";
import "package:material_symbols_icons/symbols.dart";

import "../../../../app/presentation/ame_overlay_semantics.dart";

class LibrarySidebarSelectionIcon extends StatefulWidget {
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
  State<LibrarySidebarSelectionIcon> createState() =>
      _LibrarySidebarSelectionIconState();
}

class _LibrarySidebarSelectionIconState
    extends State<LibrarySidebarSelectionIcon> {
  final _focus = FocusNode(debugLabel: "Sidebar selection");

  @override
  void dispose() {
    _focus.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final colors = Theme.of(context).colorScheme;
    return AmeTooltip(
      message: widget.selected ? "取消选择${widget.name}" : "选择${widget.name}",
      child: IconButton(
        focusNode: _focus,
        isSelected: widget.selected,
        onPressed: widget.onPressed == null
            ? null
            : () {
                _focus.requestFocus();
                widget.onPressed!();
              },
        style: IconButton.styleFrom(
          minimumSize: const Size.square(40),
          maximumSize: const Size.square(40),
          padding: const EdgeInsets.all(8),
        ),
        icon: Icon(widget.icon, size: 24),
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
