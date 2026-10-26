import "dart:async";

import "package:flutter/material.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:material_symbols_icons/symbols.dart";

import "../../../library/application/library_cleanup_controller.dart";
import "../../../library/application/library_cleanup_state.dart";
import "../organization_record_actions.dart";
import "settings_section.dart";

class OrganizationCleanupRow extends ConsumerWidget {
  const OrganizationCleanupRow({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final state = ref.watch(libraryCleanupControllerProvider);
    final busy = state is LibraryCleanupWorking;
    return SettingsRow(
      icon: Symbols.cleaning_services_rounded,
      title: "清理未关联记录",
      subtitle: const Text("检查无法关联图片的相册、收藏和整理记录"),
      trailing: FilledButton.tonal(
        key: const Key("settings-cleanup-inspect"),
        onPressed: busy
            ? null
            : () => unawaited(inspectOrganizationCleanup(context)),
        child: const Text("查看并清理"),
      ),
    );
  }
}
