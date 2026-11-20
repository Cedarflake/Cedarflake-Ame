import "package:material_ui/material_ui.dart";
import "package:material_symbols_icons/symbols.dart";

import "../../../../app/presentation/ame_dialog.dart";
import "../../../../app/presentation/ame_localizations.dart";
import "../../../../app/presentation/ame_notification_time.dart";
import "../../../library/application/library_archive_state.dart";
import "../organization_archive_messages.dart";

Future<bool> showOrganizationArchiveConfirmation(
  BuildContext context,
  LibraryArchiveReady ready,
) async {
  final data = ready.inspection;
  final created =
      data.createdUnixMs >= BigInt.zero &&
          data.createdUnixMs <= BigInt.from(8640000000000000)
      ? DateTime.fromMillisecondsSinceEpoch(data.createdUnixMs.toInt())
      : null;
  return await showDialog<bool>(
        context: context,
        builder: (context) => AmeDialog(
          key: const Key("organization-archive-confirmation"),
          title: context.l10n.restoreOrganizationRecords,
          content: Column(
            mainAxisSize: MainAxisSize.min,
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              AmeDialogItem(
                icon: Symbols.settings_backup_restore_rounded,
                title: archiveFilename(ready.path),
                subtitle: created == null
                    ? null
                    : context.l10n.createdAt(
                        formatAmeNotificationOccurredAt(context.l10n, created),
                      ),
              ),
              const SizedBox(height: 24),
              for (final row in [
                if (data.albums > 0)
                  (
                    Symbols.photo_album_rounded,
                    context.l10n.albumCount(data.albums),
                  ),
                if (data.groups > 0)
                  (
                    Symbols.photo_library_rounded,
                    context.l10n.photoGroupCount(data.groups),
                  ),
                if (data.favorites > 0)
                  (
                    Symbols.favorite_rounded,
                    context.l10n.favoriteGroupCount(data.favorites),
                  ),
                if (data.duplicateChoices > 0)
                  (
                    Symbols.layers_rounded,
                    context.l10n.duplicateChoiceCount(data.duplicateChoices),
                  ),
                if (data.matchedGroups > 0)
                  (
                    Symbols.link_rounded,
                    context.l10n.matchedGroups(data.matchedGroups),
                  ),
                if (data.unmatchedGroups > 0)
                  (
                    Symbols.link_off_rounded,
                    context.l10n.unmatchedGroups(data.unmatchedGroups),
                  ),
                if (data.conflicts > 0)
                  (
                    Symbols.warning_rounded,
                    context.l10n.albumConflictCount(data.conflicts),
                  ),
                if (data.unresolvedPreferredCopies > 0)
                  (
                    Symbols.image_search_rounded,
                    context.l10n.unresolvedCopies(
                      data.unresolvedPreferredCopies,
                    ),
                  ),
                if (data.hasContinuation)
                  (
                    Symbols.bookmark_rounded,
                    context.l10n.includesOrganizationPosition,
                  ),
              ])
                Padding(
                  padding: const EdgeInsets.only(bottom: 12),
                  child: AmeDialogItem(icon: row.$1, title: row.$2),
                ),
              const SizedBox(height: 8),
              Text(context.l10n.restoreKeepsChoices),
              if (data.unmatchedGroups > 0 ||
                  data.unresolvedPreferredCopies > 0) ...[
                const SizedBox(height: 8),
                Text(context.l10n.unmatchedAwaitPhotos),
              ],
            ],
          ),
          actions: [
            TextButton(
              onPressed: () => Navigator.of(context).pop(false),
              child: Text(context.l10n.cancel),
            ),
            FilledButton(
              key: const Key("organization-archive-restore-confirm"),
              onPressed: () => Navigator.of(context).pop(true),
              child: Text(context.l10n.restore),
            ),
          ],
        ),
      ) ??
      false;
}
