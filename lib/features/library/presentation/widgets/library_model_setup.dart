import "dart:async";

import "package:material_ui/material_ui.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";

import "../../../../app/presentation/ame_localizations.dart";
import "../../application/library_classification_configuration.dart";
import "../../application/library_model_installation.dart";
import "../../application/library_model_activation.dart";

Future<void> showLibraryModelSetup(BuildContext context, WidgetRef ref) async {
  final configuration = ref.read(libraryClassificationConfigurationProvider);
  final installation = ref.read(libraryModelInstallationProvider);
  if (configuration == null || installation == null || !installation.canStart) {
    return;
  }

  final confirmed = await showDialog<bool>(
    context: context,
    builder: (context) => AlertDialog(
      key: const Key("classification-model-download"),
      title: Text(context.l10n.downloadSmartModel),
      content: SizedBox(
        width: 480,
        child: Text(context.l10n.smartModelDownloadDetail),
      ),
      actions: [
        TextButton(
          onPressed: () => Navigator.of(context).pop(false),
          child: Text(context.l10n.cancel),
        ),
        FilledButton(
          onPressed: () => Navigator.of(context).pop(true),
          child: Text(context.l10n.downloadAndEnable),
        ),
      ],
    ),
  );
  if (confirmed != true || !context.mounted || !installation.canStart) return;

  unawaited(installation.start(activateLibraryModel(configuration)));
}
