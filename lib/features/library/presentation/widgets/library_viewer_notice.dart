import "package:flutter/material.dart";

import "../../../../app/presentation/ame_theme.dart";

class LibraryViewerNotice extends StatelessWidget {
  const LibraryViewerNotice({
    required this.icon,
    required this.message,
    required this.actionLabel,
    required this.onAction,
    this.actionKey,
    super.key,
  });

  final IconData icon;
  final String message;
  final String actionLabel;
  final VoidCallback? onAction;
  final Key? actionKey;

  @override
  Widget build(BuildContext context) => Positioned(
    left: 16,
    right: 16,
    top: 16,
    child: Center(
      child: ConstrainedBox(
        constraints: const BoxConstraints(maxWidth: ameNotificationWidth),
        child: Semantics(
          liveRegion: true,
          child: Material(
            color: Theme.of(context).colorScheme.surfaceContainerHigh,
            elevation: ameNotificationElevation,
            borderRadius: BorderRadius.circular(ameNotificationRadius),
            child: Padding(
              padding: const EdgeInsets.only(left: 14, right: 6),
              child: Row(
                mainAxisSize: MainAxisSize.min,
                children: [
                  Icon(icon, size: 20),
                  const SizedBox(width: 8),
                  Flexible(child: Text(message)),
                  TextButton(
                    key: actionKey,
                    onPressed: onAction,
                    child: Text(actionLabel),
                  ),
                ],
              ),
            ),
          ),
        ),
      ),
    ),
  );
}
