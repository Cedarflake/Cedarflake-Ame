import "dart:async";

import "package:flutter/material.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:material_symbols_icons/symbols.dart";

import "../../../../app/presentation/ame_menu.dart";
import "../../application/library_classification_actions.dart";
import "../../application/library_classification_review.dart";
import "../../application/library_classification_selection.dart";
import "../../application/library_classification_settings.dart";
import "../../domain/library_classification_review.dart";
import "../../domain/library_organization_batch.dart";
import "library_selection_progress.dart";
import "library_classification_category_menu.dart";

class LibraryClassificationControls extends ConsumerStatefulWidget {
  const LibraryClassificationControls({
    required this.scope,
    required this.enabled,
    this.onCommitted,
    this.viewer = false,
    super.key,
  });
  final LibraryClassificationScope scope;
  final bool enabled;
  final VoidCallback? onCommitted;
  final bool viewer;

  @override
  ConsumerState<LibraryClassificationControls> createState() =>
      _LibraryClassificationControlsState();
}

class _LibraryClassificationControlsState
    extends ConsumerState<LibraryClassificationControls> {
  LibraryClassificationSelection? _selection;
  LibraryClassificationSummary? _summary;
  bool _working = false;
  bool _menuOpen = false;
  bool _failed = false;

  @override
  void initState() {
    super.initState();
    _prepare();
  }

  @override
  void didUpdateWidget(LibraryClassificationControls oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.scope.catalogRevision != widget.scope.catalogRevision &&
        !_working &&
        !_menuOpen) {
      _prepare();
    }
  }

  void _prepare({bool reportFailure = false}) {
    _selection?.dispose();
    final selection = LibraryClassificationSelection(
      ref.read(libraryClassificationReviewServiceProvider),
      widget.scope,
    );
    _selection = selection;
    _summary = null;
    _failed = false;
    unawaited(_load(selection, reportFailure: reportFailure));
  }

  Future<void> _load(
    LibraryClassificationSelection selection, {
    required bool reportFailure,
  }) async {
    try {
      final summary = await selection.prepare();
      if (mounted && identical(_selection, selection)) {
        setState(() => _summary = summary);
      }
    } on Object catch (error) {
      if (mounted && identical(_selection, selection)) {
        setState(() => _failed = true);
        if (reportFailure) {
          ref
              .read(libraryClassificationActionsProvider.notifier)
              .showFailure(error);
        }
      }
    }
  }

  Future<void> _apply(LibraryClassificationCommand command) async {
    final selection = _selection;
    if (selection == null || _working || !widget.enabled) return;
    setState(() => _working = true);
    final onCommitted = widget.onCommitted;
    final saved = await ref
        .read(libraryClassificationActionsProvider.notifier)
        .apply(selection, command);
    if (saved) onCommitted?.call();
    if (!mounted || !identical(_selection, selection)) return;
    setState(() {
      _working = false;
      _prepare();
    });
  }

  @override
  void dispose() {
    _selection?.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final categories =
        ref.watch(libraryClassificationCategoriesProvider).value ?? const [];
    final saving = ref.watch(
      libraryClassificationActionsProvider.select((state) => state.saving),
    );
    final enabled = widget.enabled && !saving && !_working;
    final ready = enabled && _summary != null;
    final canConfirm = ready && _summary!.confirmable > 0;
    final categoryId = _summary?.commonCategoryId;
    final categoryName = categories
        .where((category) => category.id == categoryId)
        .firstOrNull
        ?.label;
    final label = widget.viewer ? categoryName ?? "分类" : "更改分类";
    final labels = [
      if (widget.viewer) "确认分类",
      for (final category in categories) category.label,
    ];
    final menu = AmePopupMenuButton<String>(
      labels: labels,
      hasContentIcon: false,
      leadingIconWidth: AmeMenuMetrics.selectionIndicatorSlotWidth,
      onOpenChanged: (open) => _menuOpen = open,
      onSelected: (choice) => unawaited(
        _apply(
          choice == "confirm"
              ? const LibraryConfirmClassification()
              : LibraryCorrectClassification(choice.substring(9)),
        ),
      ),
      items: [
        if (widget.viewer) ...[
          PopupMenuItem(
            value: "confirm",
            enabled: canConfirm,
            child: const AmeMenuItemContent(
              label: "确认分类",
              icon: Symbols.check_rounded,
            ),
          ),
          const PopupMenuDivider(height: AmeMenuMetrics.dividerHeight),
        ],
        ...libraryClassificationCategoryItems(
          categories: categories,
          selectedCategoryId: categoryId,
        ),
      ],
      builder: (context, openMenu) {
        VoidCallback? activate;
        if (_failed && enabled) {
          activate = () => setState(() => _prepare(reportFailure: true));
        } else if (ready) {
          activate = openMenu;
        }
        return Tooltip(
          message: _failed
              ? "重新读取分类"
              : _summary?.singleItem?.reviewLabel ?? "更改分类",
          child: TextButton.icon(
            key: const Key("library-change-classification"),
            onPressed: activate,
            icon: const Icon(Symbols.label_rounded),
            label: Row(
              mainAxisSize: MainAxisSize.min,
              children: [
                Text(label),
                const SizedBox(width: 4),
                const Icon(Symbols.arrow_drop_down_rounded, size: 20),
              ],
            ),
          ),
        );
      },
    );
    final controls = widget.viewer
        ? menu
        : Row(
            mainAxisSize: MainAxisSize.min,
            children: [
              TextButton.icon(
                key: const Key("library-confirm-classification"),
                onPressed: canConfirm
                    ? () => unawaited(
                        _apply(const LibraryConfirmClassification()),
                      )
                    : null,
                icon: const Icon(Symbols.check_rounded),
                label: const Text("确认分类"),
              ),
              menu,
            ],
          );
    return Stack(
      alignment: Alignment.bottomCenter,
      children: [
        controls,
        if (_working || (_summary == null && !_failed))
          Positioned(
            left: 0,
            right: 0,
            bottom: 0,
            child: LibrarySelectionProgress(
              key: ObjectKey(_selection),
              progress: _working
                  ? const LibraryBatchProgress(LibraryBatchPhase.saving, 0, 0)
                  : null,
            ),
          ),
      ],
    );
  }
}
