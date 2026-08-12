import "package:flutter/material.dart";

// Flutter 3.44 can merge sibling OverlayPortal traversal parents and serialize
// an orphan node on Windows. Keep each tooltip or menu anchor in its own
// semantics container until flutter/flutter#190344 and #190431 reach the
// pinned SDK.
class AmeOverlayTraversalBoundary extends StatelessWidget {
  const AmeOverlayTraversalBoundary({required this.child, super.key});

  final Widget child;

  @override
  Widget build(BuildContext context) {
    return Semantics(container: true, explicitChildNodes: true, child: child);
  }
}

class AmeTooltip extends StatelessWidget {
  const AmeTooltip({
    required this.message,
    required this.child,
    this.waitDuration,
    super.key,
  });

  final String message;
  final Widget child;
  final Duration? waitDuration;

  @override
  Widget build(BuildContext context) {
    return AmeOverlayTraversalBoundary(
      child: Tooltip(
        message: message,
        waitDuration: waitDuration,
        child: child,
      ),
    );
  }
}
