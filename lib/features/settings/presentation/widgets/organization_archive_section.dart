import "dart:async";

import "package:material_ui/material_ui.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:material_symbols_icons/symbols.dart";

import "../../../../app/messages/ame_message.dart";
import "../../../../app/notifications/ame_notification_controller.dart";
import "../../../../app/presentation/ame_localizations.dart";
import "../../../library/application/library_archive_controller.dart";
import "../../../library/application/library_archive_state.dart";
import "../../application/organization_archive_picker.dart";
import "../organization_record_actions.dart";
import "organization_cleanup_row.dart";
import "settings_section.dart";

class OrganizationArchiveSection extends ConsumerStatefulWidget {
  const OrganizationArchiveSection({super.key});

  @override
  ConsumerState<OrganizationArchiveSection> createState() =>
      _OrganizationArchiveSectionState();
}

class _OrganizationArchiveSectionState
    extends ConsumerState<OrganizationArchiveSection> {
  bool _picking = false;

  Future<void> _choose({required bool exporting}) async {
    if (_picking ||
        ref.read(libraryArchiveControllerProvider) is LibraryArchiveWorking) {
      return;
    }
    setState(() => _picking = true);
    String? path;
    try {
      final picker = ref.read(organizationArchivePickerProvider);
      path = await (exporting ? picker.destination() : picker.backup());
    } on Object {
      if (mounted) {
        ref
            .read(ameNotificationControllerProvider.notifier)
            .publish(
              const AmeNotificationDraft(
                title: AmeLabel.filePickerFailed,
                message: AmeLabel.tryAgain,
                severity: AmeNotificationSeverity.error,
                dedupeKey: "organization-archive-picker",
              ),
            );
      }
    } finally {
      if (mounted) setState(() => _picking = false);
    }
    if (!mounted || path == null) return;
    final controller = ref.read(libraryArchiveControllerProvider.notifier);
    if (exporting) {
      await controller.export(path);
    } else {
      await controller.inspect(path);
      if (mounted) await confirmOrganizationArchive(context);
    }
  }

  @override
  Widget build(BuildContext context) {
    final state = ref.watch(libraryArchiveControllerProvider);
    final busy = _picking || state is LibraryArchiveWorking;
    return SettingsSection(
      title: context.l10n.organizationRecords,
      children: [
        SettingsRow(
          icon: Symbols.file_upload_rounded,
          title: context.l10n.exportOrganizationRecords,
          subtitle: Text(context.l10n.exportArchiveHelp),
          trailing: FilledButton.tonal(
            key: const Key("settings-archive-export"),
            onPressed: busy ? null : () => unawaited(_choose(exporting: true)),
            child: Text(context.l10n.exportAction),
          ),
        ),
        SettingsRow(
          icon: Symbols.restore_page_rounded,
          title: context.l10n.restoreOrganizationRecords,
          subtitle: Text(context.l10n.restoreArchiveHelp),
          trailing: FilledButton.tonal(
            key: const Key("settings-archive-inspect"),
            onPressed: busy
                ? null
                : () => unawaited(
                    state is LibraryArchiveReady
                        ? confirmOrganizationArchive(context)
                        : _choose(exporting: false),
                  ),
            child: Text(
              state is LibraryArchiveReady
                  ? context.l10n.viewBackup
                  : context.l10n.chooseFile,
            ),
          ),
        ),
        const OrganizationCleanupRow(),
      ],
    );
  }
}
