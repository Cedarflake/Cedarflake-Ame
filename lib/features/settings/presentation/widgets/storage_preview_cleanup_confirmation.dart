import "package:material_ui/material_ui.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";

import "../../../../app/presentation/ame_dialog.dart";
import "../../../../app/presentation/ame_dialog_location.dart";
import "../../../../app/presentation/ame_localizations.dart";
import "../../../library/application/library_platform_actions.dart";

class StoragePreviewCleanupConfirmation extends ConsumerWidget {
  const StoragePreviewCleanupConfirmation({
    this.retiredDirectory,
    this.retiredDisplayPath,
    super.key,
  });

  final String? retiredDirectory;
  final String? retiredDisplayPath;

  @override
  Widget build(BuildContext context, WidgetRef ref) => AmeDialog(
    title: retiredDirectory == null
        ? context.l10n.confirmClearThumbnails
        : context.l10n.confirmClearOldThumbnails,
    content: Column(
      mainAxisSize: MainAxisSize.min,
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        if (retiredDirectory case final path?) ...[
          AmeDialogLocation(
            label: context.l10n.oldThumbnailFolder,
            path: retiredDisplayPath ?? path,
            onOpen: () =>
                ref.read(libraryPlatformActionsProvider).revealDirectory(path),
          ),
          const SizedBox(height: 20),
        ],
        Text(
          retiredDirectory == null
              ? context.l10n.clearThumbnailsImpact
              : context.l10n.clearOldThumbnailsImpact,
        ),
      ],
    ),
    actions: [
      TextButton(
        onPressed: () => Navigator.pop(context, false),
        child: Text(context.l10n.cancel),
      ),
      FilledButton(
        onPressed: () => Navigator.pop(context, true),
        child: Text(context.l10n.startCleanup),
      ),
    ],
  );
}
