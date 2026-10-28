import "package:flutter/material.dart";

import "../../../../app/presentation/ame_feedback_style.dart";

class LibraryViewerNotice extends StatelessWidget {
  const LibraryViewerNotice({
    required this.icon,
    required this.message,
    this.actionLabel,
    this.onAction,
    this.actionKey,
    super.key,
  });

  final IconData icon;
  final String message;
  final String? actionLabel;
  final VoidCallback? onAction;
  final Key? actionKey;

  @override
  Widget build(BuildContext context) => Positioned(
    left: ameViewerNoticeInset,
    right: ameViewerNoticeInset,
    top: ameViewerNoticeInset,
    child: Center(
      child: ConstrainedBox(
        constraints: const BoxConstraints(
          maxWidth: ameNotificationWidth,
          minHeight: kMinInteractiveDimension,
        ),
        child: Semantics(
          liveRegion: true,
          child: AmeFeedbackSurface(
            child: Padding(
              padding: const EdgeInsets.only(left: 14, right: 6),
              child: Row(
                mainAxisSize: MainAxisSize.min,
                children: [
                  Icon(icon, size: ameNotificationIconSize),
                  const SizedBox(width: 8),
                  Flexible(
                    child: Text(
                      message,
                      style: Theme.of(context).textTheme.bodyMedium,
                    ),
                  ),
                  if (actionLabel case final label?)
                    AmeFeedbackAction(
                      key: actionKey,
                      onPressed: onAction,
                      label: label,
                    ).build(),
                ],
              ),
            ),
          ),
        ),
      ),
    ),
  );
}
