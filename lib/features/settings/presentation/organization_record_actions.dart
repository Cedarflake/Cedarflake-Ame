import "package:flutter/material.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";

import "../../library/application/library_archive_controller.dart";
import "../../library/application/library_archive_state.dart";
import "../../library/application/library_cleanup_controller.dart";
import "../../library/application/library_cleanup_state.dart";
import "widgets/organization_archive_confirmation.dart";
import "widgets/organization_cleanup_confirmation.dart";

Future<void> confirmOrganizationArchive(BuildContext context) async {
  final container = ProviderScope.containerOf(context, listen: false);
  final ready = container.read(libraryArchiveControllerProvider);
  if (ready is! LibraryArchiveReady) return;
  final confirmed = await showOrganizationArchiveConfirmation(context, ready);
  if (!context.mounted) return;
  final controller = container.read(libraryArchiveControllerProvider.notifier);
  if (confirmed) {
    await controller.restore(ready);
  } else if (identical(
    container.read(libraryArchiveControllerProvider),
    ready,
  )) {
    controller.dismiss();
  }
}

Future<void> retryOrganizationArchive(BuildContext context) async {
  final container = ProviderScope.containerOf(context, listen: false);
  await container.read(libraryArchiveControllerProvider.notifier).retry();
  if (context.mounted) await confirmOrganizationArchive(context);
}

Future<void> inspectOrganizationCleanup(BuildContext context) async {
  final container = ProviderScope.containerOf(context, listen: false);
  final state = container.read(libraryCleanupControllerProvider);
  final minimumAgeDays = switch (state) {
    LibraryCleanupReady(:final minimumAgeDays) => minimumAgeDays ?? 0,
    LibraryCleanupFailed(:final request) => request.minimumAgeDays ?? 0,
    _ => 90,
  };
  final confirmed = await showOrganizationCleanupConfirmation(
    context,
    minimumAgeDays: minimumAgeDays,
  );
  if (context.mounted && confirmed != null) {
    await container
        .read(libraryCleanupControllerProvider.notifier)
        .apply(confirmed);
  }
}
