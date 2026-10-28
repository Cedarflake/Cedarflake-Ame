import "package:flutter/gestures.dart";
import "package:flutter/material.dart";
import "package:flutter/services.dart";

class LibrarySidebarSelectionRegion extends StatefulWidget {
  const LibrarySidebarSelectionRegion({
    required this.child,
    required this.onClear,
    super.key,
  });

  final Widget child;
  final VoidCallback? onClear;

  @override
  State<LibrarySidebarSelectionRegion> createState() =>
      _LibrarySidebarSelectionRegionState();
}

class _LibrarySidebarSelectionRegionState
    extends State<LibrarySidebarSelectionRegion> {
  final _focus = FocusNode(debugLabel: "Sidebar selection region");

  @override
  void dispose() {
    _focus.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) => CallbackShortcuts(
    bindings: {
      const SingleActivator(LogicalKeyboardKey.escape): ?widget.onClear,
    },
    child: Focus(
      focusNode: _focus,
      skipTraversal: true,
      child: Listener(
        onPointerDown: (event) {
          // Pointer selection keeps shortcuts in the sidebar without giving a
          // row persistent keyboard-focus decoration. Tab still focuses controls.
          if (event.buttons == kPrimaryMouseButton) _focus.requestFocus();
        },
        child: widget.child,
      ),
    ),
  );
}
