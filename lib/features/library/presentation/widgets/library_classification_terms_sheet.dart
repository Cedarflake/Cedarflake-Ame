import "package:material_ui/material_ui.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:material_symbols_icons/symbols.dart";

import "../../../../app/presentation/ame_dialog.dart";
import "../../../../app/presentation/ame_localizations.dart";
import "../../../../app/presentation/ame_progress.dart";
import "../../../../app/presentation/ame_scroll_motion.dart";
import "../../../../app/presentation/ame_tray.dart";
import "../../application/library_classification_terms.dart";
import "../../domain/library_classification_term.dart";
import "../library_category_label.dart";
import "library_classification_term_editor.dart";

Future<void> showLibraryClassificationTerms(BuildContext context) =>
    showAmeTray<void>(
      context: context,
      builder: (_) => const LibraryClassificationTermsSheet(),
    );

class LibraryClassificationTermsSheet extends ConsumerStatefulWidget {
  const LibraryClassificationTermsSheet({super.key});

  @override
  ConsumerState<LibraryClassificationTermsSheet> createState() =>
      _LibraryClassificationTermsSheetState();
}

class _LibraryClassificationTermsSheetState
    extends ConsumerState<LibraryClassificationTermsSheet> {
  final _scroll = ScrollController();
  List<LibraryClassificationTerm>? _terms;
  bool _loading = true;
  bool _failed = false;

  @override
  void initState() {
    super.initState();
    _load();
  }

  Future<void> _load() async {
    setState(() {
      _loading = true;
      _failed = false;
    });
    try {
      final editor = ref.read(libraryClassificationTermsProvider);
      if (editor == null) throw StateError("Term editing is unavailable");
      final terms = await editor.load();
      if (mounted) {
        setState(
          () => _terms = terms
              .where((term) => term.category.id != "other")
              .toList(),
        );
      }
    } on Object {
      if (mounted) setState(() => _failed = true);
    } finally {
      if (mounted) setState(() => _loading = false);
    }
  }

  Future<void> _edit([LibraryClassificationTerm? original]) async {
    final change = await showDialog<LibraryClassificationTermChange>(
      context: context,
      builder: (_) => LibraryClassificationTermEditor(original: original),
    );
    if (!mounted) return;
    if (change == null) {
      await _load();
      return;
    }
    setState(() {
      final terms = _terms!;
      final index = terms.indexWhere((term) => term.category.id == change.id);
      final updated = change.term;
      if (updated == null) {
        if (index >= 0) terms.removeAt(index);
      } else if (index >= 0) {
        terms[index] = updated;
      } else {
        terms.add(updated);
      }
    });
  }

  Widget _buildTerms(BuildContext context) {
    final terms = _terms;
    if (terms == null) {
      if (_failed) {
        return AmeDialogReadFailure(
          message: context.l10n.smartAlbumTermsLoadFailed,
          onRetry: _load,
        );
      }
      return Padding(
        padding: const EdgeInsets.symmetric(vertical: 24),
        child: Center(
          child: AmeLoadingProgress(
            semanticsLabel: context.l10n.loadingSmartAlbums,
          ),
        ),
      );
    }
    return AmeScrollMotion(
      child: ListView.builder(
        controller: _scroll,
        shrinkWrap: true,
        itemCount: terms.length,
        itemBuilder: (context, index) {
          final term = terms[index];
          return ListTile(
            key: ValueKey("classification-term-${term.category.id}"),
            shape: RoundedRectangleBorder(
              borderRadius: BorderRadius.circular(12),
            ),
            title: Text(libraryCategoryLabel(context.l10n, term.category)),
            subtitle: Text(
              term.category.descriptions.first,
              maxLines: 1,
              overflow: TextOverflow.ellipsis,
            ),
            trailing: const Icon(Symbols.chevron_right_rounded, size: 20),
            onTap: _loading ? null : () => _edit(term),
          );
        },
      ),
    );
  }

  @override
  void dispose() {
    _scroll.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) => AmeTray(
    key: const Key("library-classification-terms"),
    child: Column(
      mainAxisSize: MainAxisSize.min,
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        Text(
          context.l10n.smartAlbumTerms,
          style: Theme.of(context).textTheme.titleLarge,
        ),
        const SizedBox(height: 16),
        Flexible(
          child: ConstrainedBox(
            constraints: const BoxConstraints(maxHeight: 360),
            child: _buildTerms(context),
          ),
        ),
        if (_failed && _terms != null)
          AmeDialogReadFailure(
            message: context.l10n.smartAlbumTermsLoadFailed,
            onRetry: _load,
          ),
        const SizedBox(height: 16),
        Wrap(
          alignment: WrapAlignment.end,
          spacing: 8,
          runSpacing: 8,
          children: [
            TextButton.icon(
              key: const Key("classification-term-add"),
              onPressed: _loading || _terms == null ? null : () => _edit(),
              icon: const Icon(Symbols.add_rounded, size: 20),
              label: Text(context.l10n.addSmartAlbumTerm),
            ),
            FilledButton(
              onPressed: () => Navigator.of(context).pop(),
              child: Text(context.l10n.close),
            ),
          ],
        ),
      ],
    ),
  );
}
