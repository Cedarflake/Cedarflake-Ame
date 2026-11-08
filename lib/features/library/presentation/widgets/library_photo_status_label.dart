import "package:material_ui/material_ui.dart";

class LibraryPhotoStatusLabel extends StatelessWidget {
  const LibraryPhotoStatusLabel({
    required this.label,
    required this.textKey,
    this.detail,
    super.key,
  });

  final String label;
  final Key textKey;
  final String? detail;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final labelWidget = Material(
      color: theme.colorScheme.surface.withValues(alpha: 0.92),
      borderRadius: BorderRadius.circular(8),
      child: Padding(
        padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 4),
        child: Text(
          label,
          key: textKey,
          maxLines: 1,
          overflow: TextOverflow.ellipsis,
          style: theme.textTheme.labelMedium?.copyWith(
            color: theme.colorScheme.onSurface,
          ),
        ),
      ),
    );
    return Align(
      alignment: Alignment.bottomLeft,
      child: Padding(
        padding: const EdgeInsets.all(8),
        child: detail == null
            ? labelWidget
            : Tooltip(message: detail, child: labelWidget),
      ),
    );
  }
}
