import "dart:async";

import "package:material_ui/material_ui.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";

import "../../../../app/presentation/ame_choice.dart";
import "../../../../app/presentation/ame_dialog.dart";
import "../../../../app/presentation/ame_localizations.dart";
import "../../../library/application/library_cleanup_controller.dart";
import "../../../library/application/library_cleanup_state.dart";
import "../../../library/domain/library_organization_cleanup.dart";

String cleanupAgeDescription(
  AppLocalizations strings,
  LibraryCleanupInspection inspection,
) {
  final oldest = inspection.oldestUnassociatedUnixMs;
  final newest = inspection.newestUnassociatedUnixMs;
  if (oldest == null || newest == null) return strings.noUnlinkedRecords;
  final longest = Duration(
    milliseconds: inspection.inspectedUnixMs - oldest,
  ).inDays;
  final shortest = Duration(
    milliseconds: inspection.inspectedUnixMs - newest,
  ).inDays;
  if (longest < 1) return strings.unlinkedLessThanDay;
  if (shortest == longest) return strings.unlinkedDays(longest);
  return strings.unlinkedRange(
    (shortest < 1 ? strings.lessThanOne : shortest).toString(),
    longest,
  );
}

Future<LibraryCleanupReady?> showOrganizationCleanupConfirmation(
  BuildContext context, {
  required int minimumAgeDays,
}) async {
  final container = ProviderScope.containerOf(context, listen: false);
  final controller = container.read(libraryCleanupControllerProvider.notifier);
  unawaited(controller.inspect(minimumAgeDays == 0 ? null : minimumAgeDays));
  LibraryCleanupReady? confirmed;
  try {
    confirmed = await showDialog<LibraryCleanupReady>(
      context: context,
      builder: (context) => _OrganizationCleanupConfirmation(
        initialMinimumAgeDays: minimumAgeDays,
      ),
    );
    return confirmed;
  } finally {
    if (confirmed == null) controller.cancel();
  }
}

class _OrganizationCleanupConfirmation extends ConsumerWidget {
  const _OrganizationCleanupConfirmation({required this.initialMinimumAgeDays});

  final int initialMinimumAgeDays;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final state = ref.watch(libraryCleanupControllerProvider);
    final controller = ref.read(libraryCleanupControllerProvider.notifier);
    final minimumAgeDays = switch (state) {
      LibraryCleanupReady(:final minimumAgeDays) => minimumAgeDays ?? 0,
      LibraryCleanupWorking(:final request) ||
      LibraryCleanupFailed(:final request) => request.minimumAgeDays ?? 0,
      _ => initialMinimumAgeDays,
    };
    final ready = state is LibraryCleanupReady ? state : null;
    return AlertDialog(
      key: const Key("organization-cleanup-confirmation"),
      title: Text(context.l10n.clearUnlinkedRecords),
      scrollable: true,
      content: SizedBox(
        width: 420,
        child: AmeDialogContent(
          isLoading: state is LibraryCleanupWorking,
          loadingLabel: state is LibraryCleanupWorking && state.stopping
              ? context.l10n.stopping
              : context.l10n.checkingRecords,
          child: Column(
            mainAxisSize: MainAxisSize.min,
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              AmeDialogField(
                label: context.l10n.cleanupScope,
                child: Align(
                  alignment: AlignmentDirectional.centerStart,
                  child: AmeChoice<int>.menu(
                    key: const Key("settings-cleanup-age"),
                    value: minimumAgeDays,
                    enabled: state is! LibraryCleanupWorking,
                    entries: [
                      AmeChoiceEntry(
                        value: 90,
                        label: context.l10n.unlinkedNinetyDays,
                      ),
                      AmeChoiceEntry(
                        value: 30,
                        label: context.l10n.unlinkedThirtyDays,
                      ),
                      AmeChoiceEntry(
                        value: 0,
                        label: context.l10n.allUnlinkedRecords,
                      ),
                    ],
                    onSelected: (value) {
                      if (value == null || value == minimumAgeDays) return;
                      unawaited(controller.inspect(value == 0 ? null : value));
                    },
                  ),
                ),
              ),
              const SizedBox(height: 16),
              switch (state) {
                LibraryCleanupReady(:final inspection) => _inspection(
                  context.l10n,
                  inspection,
                ),
                LibraryCleanupWorking() => const SizedBox.shrink(),
                LibraryCleanupFailed() => AmeDialogError(
                  context.l10n.checkFailedRetry,
                ),
                LibraryCleanupFinished() => Text(context.l10n.checkCancelled),
                LibraryCleanupIdle() => const SizedBox.shrink(),
              },
            ],
          ),
        ),
      ),
      actions: [
        TextButton(
          autofocus: true,
          onPressed: () => Navigator.of(context).pop(),
          child: Text(context.l10n.cancel),
        ),
        if (state is LibraryCleanupFailed)
          FilledButton(
            key: const Key("organization-cleanup-retry"),
            onPressed: () => unawaited(controller.retry()),
            child: Text(context.l10n.retry),
          )
        else
          FilledButton(
            key: const Key("organization-cleanup-confirm"),
            onPressed: ready != null && ready.inspection.total > 0
                ? () => Navigator.of(context).pop(ready)
                : null,
            child: Text(context.l10n.clearRecords),
          ),
      ],
    );
  }

  Widget _inspection(
    AppLocalizations strings,
    LibraryCleanupInspection inspection,
  ) {
    if (inspection.total == 0) return Text(strings.noUnlinkedRecords);
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Text(
          strings.cleanupCounts(inspection.groups, inspection.duplicateChoices),
        ),
        Text(cleanupAgeDescription(strings, inspection)),
        const SizedBox(height: 16),
        Text(strings.cleanupAlbumGroups(inspection.albumGroups)),
        Text(strings.cleanupFavoriteGroups(inspection.favoriteGroups)),
        Text(strings.cleanupProgressGroups(inspection.progressGroups)),
        if (inspection.classificationGroups > 0)
          Text(
            strings.cleanupClassificationGroups(
              inspection.classificationGroups,
            ),
          ),
        const SizedBox(height: 16),
        Text(strings.cleanupRecordImpact),
        const SizedBox(height: 8),
        Text(strings.cleanupUndoWarning),
      ],
    );
  }
}
