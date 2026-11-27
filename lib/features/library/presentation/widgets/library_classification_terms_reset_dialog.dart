import "package:material_ui/material_ui.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";

import "../../../../app/presentation/ame_dialog.dart";
import "../../../../app/presentation/ame_localizations.dart";
import "../../application/library_classification_terms.dart";
import "../../domain/library_classification_review.dart";
import "../../domain/library_classification_term.dart";

class LibraryClassificationTermsResetDialog extends ConsumerStatefulWidget {
  const LibraryClassificationTermsResetDialog({required this.terms, super.key});

  final List<LibraryClassificationTerm> terms;

  @override
  ConsumerState<LibraryClassificationTermsResetDialog> createState() =>
      _LibraryClassificationTermsResetDialogState();
}

class _LibraryClassificationTermsResetDialogState
    extends ConsumerState<LibraryClassificationTermsResetDialog> {
  final _operationId = newClassificationTermOperationId();
  bool _saving = false;
  String? _errorCode;

  Future<void> _restore() async {
    if (_saving) return;
    setState(() {
      _saving = true;
      _errorCode = null;
    });
    try {
      final editor = ref.read(libraryClassificationTermsProvider);
      if (editor == null) throw StateError("Term editing is unavailable");
      await editor.resetAll(operationId: _operationId, originals: widget.terms);
      if (mounted) Navigator.of(context).pop(true);
    } on Object catch (error) {
      if (!mounted) return;
      setState(() {
        _saving = false;
        _errorCode = error is LibraryClassificationFailure
            ? error.code
            : "classification_failed";
      });
    }
  }

  @override
  Widget build(BuildContext context) => PopScope(
    canPop: !_saving,
    child: AmeDialog(
      key: const Key("classification-terms-reset-dialog"),
      title: context.l10n.restoreAllSmartAlbumTerms,
      content: AmeDialogBody(
        children: [
          Text(context.l10n.restoreAllSmartAlbumTermsImpact),
          if (_errorCode case final code?)
            AmeDialogError(
              code == "classification_term_changed"
                  ? context.l10n.smartAlbumTermsChanged
                  : context.l10n.smartAlbumTermsRestoreFailed,
            ),
        ],
      ),
      actions: [
        TextButton(
          key: const Key("classification-terms-reset-cancel"),
          onPressed: _saving ? null : () => Navigator.of(context).pop(false),
          child: Text(context.l10n.cancel),
        ),
        FilledButton(
          key: const Key("classification-terms-reset-confirm"),
          onPressed: _saving ? null : _restore,
          child: Text(context.l10n.restore),
        ),
      ],
    ),
  );
}
