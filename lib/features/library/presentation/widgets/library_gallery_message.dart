import "package:flutter/material.dart";

class LibraryGalleryMessage extends StatelessWidget {
  const LibraryGalleryMessage({
    required this.icon,
    required this.title,
    this.detail,
    this.action,
    super.key,
  });

  final IconData icon;
  final String title;
  final String? detail;
  final Widget? action;

  @override
  Widget build(BuildContext context) => Center(
    child: ConstrainedBox(
      constraints: const BoxConstraints(maxWidth: 520),
      child: Padding(
        padding: const EdgeInsets.all(32),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            Icon(icon, size: 48),
            const SizedBox(height: 16),
            Text(
              title,
              style: Theme.of(context).textTheme.titleLarge,
              textAlign: TextAlign.center,
            ),
            if (detail case final message?) ...[
              const SizedBox(height: 8),
              Text(message, textAlign: TextAlign.center),
            ],
            if (action case final button?) ...[
              const SizedBox(height: 16),
              button,
            ],
          ],
        ),
      ),
    ),
  );
}
