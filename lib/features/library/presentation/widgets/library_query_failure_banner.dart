import "package:material_ui/material_ui.dart";

import "../../../../app/presentation/ame_localizations.dart";

class LibraryQueryFailureBanner extends StatelessWidget {
  const LibraryQueryFailureBanner({required this.onRetry, super.key});

  final VoidCallback? onRetry;

  @override
  Widget build(BuildContext context) => MaterialBanner(
    key: const Key("library-query-failure"),
    content: Text(context.l10n.photoListFailed),
    actions: [
      TextButton(
        key: const Key("library-query-retry"),
        onPressed: onRetry,
        child: Text(context.l10n.retryLoading),
      ),
    ],
  );
}
