import "package:material_ui/material_ui.dart";

import "ame_progress.dart";

class AmeDialog extends StatelessWidget {
  const AmeDialog({
    required this.title,
    required this.content,
    required this.actions,
    this.loadingLabel,
    this.isLoading = false,
    super.key,
  });

  final String title;
  final Widget content;
  final List<Widget> actions;
  final String? loadingLabel;
  final bool isLoading;

  @override
  Widget build(BuildContext context) => AlertDialog(
    title: Text(title),
    scrollable: true,
    content: SizedBox(
      width: 440,
      child: loadingLabel == null
          ? content
          : AmeDialogContent(
              isLoading: isLoading,
              loadingLabel: loadingLabel!,
              child: content,
            ),
    ),
    actions: actions,
  );
}

class AmeDialogContent extends StatelessWidget {
  const AmeDialogContent({
    required this.isLoading,
    required this.loadingLabel,
    required this.child,
    super.key,
  });

  final bool isLoading;
  final String loadingLabel;
  final Widget child;

  @override
  Widget build(BuildContext context) => ConstrainedBox(
    constraints: const BoxConstraints(minHeight: 48),
    child: Stack(
      alignment: Alignment.center,
      children: [
        Visibility(
          visible: !isLoading,
          maintainSize: true,
          maintainState: true,
          maintainAnimation: true,
          child: child,
        ),
        Positioned.fill(
          child: Center(
            child: Semantics(
              liveRegion: true,
              child: AmeLoadingProgress(
                active: isLoading,
                semanticsLabel: loadingLabel,
              ),
            ),
          ),
        ),
      ],
    ),
  );
}

class AmeDialogField extends StatelessWidget {
  const AmeDialogField({required this.label, required this.child, super.key});

  final String label;
  final Widget child;

  @override
  Widget build(BuildContext context) => Column(
    mainAxisSize: MainAxisSize.min,
    crossAxisAlignment: CrossAxisAlignment.start,
    children: [
      Text(label, style: Theme.of(context).textTheme.labelLarge),
      const SizedBox(height: 8),
      child,
    ],
  );
}

class AmeDialogItem extends StatelessWidget {
  const AmeDialogItem({
    required this.icon,
    required this.title,
    this.subtitle,
    this.trailing,
    super.key,
  });

  final IconData icon;
  final String title;
  final String? subtitle;
  final Widget? trailing;

  @override
  Widget build(BuildContext context) => Row(
    children: [
      Icon(icon),
      const SizedBox(width: 16),
      Expanded(
        child: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Text(title, style: Theme.of(context).textTheme.titleMedium),
            if (subtitle case final text?) ...[
              const SizedBox(height: 4),
              Text(text, style: Theme.of(context).textTheme.bodySmall),
            ],
          ],
        ),
      ),
      if (trailing case final action?) ...[const SizedBox(width: 12), action],
    ],
  );
}
