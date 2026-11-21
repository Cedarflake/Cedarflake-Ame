import "package:material_ui/material_ui.dart";

import "../../../../app/presentation/ame_disclosure.dart";
import "../../../../app/presentation/ame_localizations.dart";
import "../../../storage/domain/storage_models.dart";

class StorageSettingsError extends StatefulWidget {
  const StorageSettingsError({required this.error, super.key});

  final Object error;

  @override
  State<StorageSettingsError> createState() => _StorageSettingsErrorState();
}

class _StorageSettingsErrorState extends State<StorageSettingsError> {
  bool _detailsVisible = false;

  @override
  Widget build(BuildContext context) {
    final message = switch (widget.error) {
      StorageSettingsFailure(code: "storage_path_overlaps_source") =>
        context.l10n.storageOverlapsSource,
      _ => null,
    };
    final style = TextStyle(color: Theme.of(context).colorScheme.error);
    if (message == null) {
      return Text(switch (widget.error) {
        StorageSettingsFailure(:final message) => message,
        final error => error.toString(),
      }, style: style);
    }
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Text(message, style: style),
        Semantics(
          expanded: _detailsVisible,
          child: TextButton(
            onPressed: () => setState(() => _detailsVisible = !_detailsVisible),
            child: Text(
              _detailsVisible
                  ? context.l10n.storageHideErrorDetails
                  : context.l10n.storageErrorDetails,
            ),
          ),
        ),
        AmeDisclosure(
          child: _detailsVisible
              ? SelectableText(widget.error.toString())
              : null,
        ),
      ],
    );
  }
}
