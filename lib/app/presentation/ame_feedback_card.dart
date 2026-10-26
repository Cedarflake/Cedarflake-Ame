import "package:flutter/material.dart";

import "ame_theme.dart";

class AmeFeedbackCard extends StatelessWidget {
  const AmeFeedbackCard({
    required this.leading,
    required this.content,
    required this.actions,
    this.progress,
    super.key,
  });

  final Widget leading;
  final Widget content;
  final List<Widget> actions;
  final Widget? progress;

  @override
  Widget build(BuildContext context) => Material(
    elevation: ameNotificationElevation,
    color: Theme.of(context).colorScheme.surfaceContainerHigh,
    borderRadius: BorderRadius.circular(ameNotificationRadius),
    child: ConstrainedBox(
      constraints: const BoxConstraints.tightFor(width: ameNotificationWidth),
      child: Padding(
        padding: const EdgeInsets.fromLTRB(20, 14, 12, 12),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            Row(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Padding(padding: const EdgeInsets.only(top: 2), child: leading),
                const SizedBox(width: 12),
                Expanded(child: content),
                ...actions,
              ],
            ),
            if (progress case final indicator?) ...[
              const SizedBox(height: 10),
              indicator,
            ],
          ],
        ),
      ),
    ),
  );
}
