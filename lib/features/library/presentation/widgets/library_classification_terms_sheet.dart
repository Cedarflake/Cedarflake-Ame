import "package:material_ui/material_ui.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:material_symbols_icons/symbols.dart";

import "../../../../app/presentation/ame_dialog.dart";
import "../../../../app/presentation/ame_localizations.dart";
import "../../../../app/presentation/ame_menu.dart";
import "../../../../app/presentation/ame_progress.dart";
import "../../../../app/presentation/ame_scroll_motion.dart";
import "../../../../app/presentation/ame_tray.dart";
import "../../application/library_classification_terms.dart";
import "../../domain/library_classification_term.dart";
import "../library_category_label.dart";
import "library_classification_term_editor.dart";
import "library_classification_terms_reset_dialog.dart";

enum _TermFilter { all, builtIn, custom }

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
  final _scroll = AmeScrollController();
  List<LibraryClassificationTerm>? _terms;
  bool _loading = true;
  bool _failed = false;
  _TermFilter _filter = _TermFilter.all;

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
    if (original == null || !original.isConfigured) {
      await _load();
    }
  }

  Future<void> _resetAll() async {
    final targets = _terms?.where((term) => term.canRestoreDefaults).toList();
    if (_loading || targets == null || targets.isEmpty) return;
    await showDialog<bool>(
      context: context,
      builder: (_) => LibraryClassificationTermsResetDialog(
        terms: List.unmodifiable(targets),
      ),
    );
    if (mounted) await _load();
  }

  void _selectFilter(_TermFilter filter) {
    if (_filter == filter) return;
    if (_scroll.hasClients) _scroll.jumpTo(0);
    setState(() => _filter = filter);
  }

  String _filterLabel(BuildContext context, _TermFilter filter) =>
      switch (filter) {
        _TermFilter.all => context.l10n.allSmartAlbumTerms,
        _TermFilter.builtIn => context.l10n.builtInSmartAlbumTerm,
        _TermFilter.custom => context.l10n.customSmartAlbumTerm,
      };

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
    final visible = terms
        .where(
          (term) => switch (_filter) {
            _TermFilter.all => true,
            _TermFilter.builtIn => !term.isCustom,
            _TermFilter.custom => term.isCustom,
          },
        )
        .toList();
    if (visible.isEmpty) {
      return Center(
        child: Padding(
          padding: const EdgeInsets.symmetric(vertical: 24),
          child: Text(
            context.l10n.noMatchingSmartAlbumTerms,
            textAlign: TextAlign.center,
            style: Theme.of(context).textTheme.bodyMedium,
          ),
        ),
      );
    }
    return AmeScrollMotion(
      controller: _scroll,
      child: ListView.builder(
        controller: _scroll,
        shrinkWrap: true,
        itemCount: visible.length,
        itemBuilder: (context, index) {
          final term = visible[index];
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
            trailing: Row(
              mainAxisSize: MainAxisSize.min,
              spacing: 8,
              children: [
                Text(
                  term.isCustom
                      ? context.l10n.customSmartAlbumTerm
                      : context.l10n.builtInSmartAlbumTerm,
                  style: Theme.of(context).textTheme.labelSmall?.copyWith(
                    color: Theme.of(context).colorScheme.onSurfaceVariant,
                  ),
                ),
                const Icon(Symbols.chevron_right_rounded, size: 20),
              ],
            ),
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
        Row(
          children: [
            Expanded(
              child: Text(
                context.l10n.smartAlbumTerms,
                style: Theme.of(context).textTheme.titleLarge,
              ),
            ),
            const SizedBox(width: 16),
            TextButton.icon(
              key: const Key("classification-term-add"),
              onPressed: _loading || _terms == null ? null : () => _edit(),
              icon: const Icon(Symbols.add_rounded, size: 20),
              label: Text(context.l10n.addSmartAlbumTerm),
            ),
            AmePopupMenuButton<bool>(
              labels: [context.l10n.restoreAllSmartAlbumTerms],
              items: [
                PopupMenuItem(
                  key: const Key("classification-terms-reset-all"),
                  value: true,
                  enabled:
                      !_loading &&
                      (_terms?.any((term) => term.canRestoreDefaults) ?? false),
                  child: AmeMenuItemContent(
                    label: context.l10n.restoreAllSmartAlbumTerms,
                    icon: Symbols.restore_rounded,
                  ),
                ),
              ],
              onSelected: (_) => _resetAll(),
              builder: (context, openMenu) => IconButton(
                key: const Key("classification-terms-more"),
                tooltip: context.l10n.more,
                onPressed: openMenu,
                icon: const Icon(Symbols.more_vert_rounded, size: 20),
              ),
            ),
          ],
        ),
        const SizedBox(height: 8),
        Wrap(
          spacing: 8,
          runSpacing: 4,
          children: [
            for (final filter in _TermFilter.values)
              ChoiceChip(
                key: ValueKey("classification-terms-filter-${filter.name}"),
                padding: const EdgeInsets.all(4),
                labelPadding: const EdgeInsets.symmetric(horizontal: 4),
                visualDensity: VisualDensity.compact,
                materialTapTargetSize: MaterialTapTargetSize.shrinkWrap,
                showCheckmark: false,
                avatar: _filter == filter
                    ? const Icon(Symbols.check_rounded, size: 18, weight: 300)
                    : null,
                avatarBoxConstraints: const BoxConstraints.tightFor(
                  width: 18,
                  height: 18,
                ),
                label: Text(_filterLabel(context, filter)),
                selected: _filter == filter,
                onSelected: (_) => _selectFilter(filter),
              ),
          ],
        ),
        const SizedBox(height: 16),
        Flexible(child: SizedBox(height: 360, child: _buildTerms(context))),
        if (_failed && _terms != null)
          AmeDialogReadFailure(
            message: context.l10n.smartAlbumTermsLoadFailed,
            onRetry: _load,
          ),
      ],
    ),
  );
}
