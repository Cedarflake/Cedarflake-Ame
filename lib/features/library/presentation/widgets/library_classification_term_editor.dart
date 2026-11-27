import "package:material_ui/material_ui.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:material_symbols_icons/symbols.dart";

import "../../../../app/presentation/ame_dialog.dart";
import "../../../../app/presentation/ame_localizations.dart";
import "../../../../app/presentation/ame_overlay_semantics.dart";
import "../../application/library_classification_terms.dart";
import "../../domain/library_classification_configuration.dart";
import "../../domain/library_classification_review.dart";
import "../../domain/library_classification_term.dart";
import "../library_category_label.dart";

class LibraryClassificationTermEditor extends ConsumerStatefulWidget {
  const LibraryClassificationTermEditor({this.original, super.key});
  final LibraryClassificationTerm? original;

  @override
  ConsumerState<LibraryClassificationTermEditor> createState() =>
      _LibraryClassificationTermEditorState();
}

class _LibraryClassificationTermEditorState
    extends ConsumerState<LibraryClassificationTermEditor> {
  final _form = GlobalKey<FormState>();
  final _name = TextEditingController();
  late final _descriptions = (widget.original?.category.descriptions ?? [""])
      .map((text) => TextEditingController(text: text))
      .toList();
  late final _id =
      widget.original?.category.id ??
      "custom-${newClassificationTermOperationId()}";
  String _operationId = newClassificationTermOperationId();
  String? _initialName;
  String? _errorCode;
  bool _saving = false;

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    if (_initialName != null) return;
    final original = widget.original;
    _initialName = original == null
        ? ""
        : libraryCategoryLabel(context.l10n, original.category);
    _name.text = _initialName!;
  }

  void _changed() {
    _operationId = newClassificationTermOperationId();
    if (_errorCode != null) setState(() => _errorCode = null);
  }

  Future<void> _save() async {
    if (_saving || !_form.currentState!.validate()) return;
    final original = widget.original;
    final name = _name.text.trim();
    final replacement = LibraryClassificationCategory(
      id: _id,
      label: original != null && name == _initialName
          ? original.category.label
          : name,
      descriptions: _descriptions.map((field) => field.text.trim()).toList(),
    );
    await _submit(
      (editor) => editor.save(
        operationId: _operationId,
        original: original,
        replacement: replacement,
      ),
    );
  }

  Future<void> _reset() async {
    final original = widget.original!;
    if (!original.isConfigured) {
      final defaults = original.defaultCategory!;
      final previous = _descriptions.toList();
      setState(() {
        _name.text = libraryCategoryLabel(context.l10n, defaults);
        _descriptions
          ..clear()
          ..addAll(
            defaults.descriptions.map(
              (text) => TextEditingController(text: text),
            ),
          );
        _errorCode = null;
      });
      WidgetsBinding.instance.addPostFrameCallback((_) {
        for (final field in previous) {
          field.dispose();
        }
      });
      _changed();
      return;
    }
    await _submit(
      (editor) =>
          editor.reset(operationId: "reset-$_operationId", original: original),
    );
  }

  Future<void> _delete() async {
    final confirmed = await showDialog<bool>(
      context: context,
      builder: (context) => AmeDialog(
        title: context.l10n.deleteSmartAlbumTerm,
        content: AmeDialogBody(
          children: [
            Text(libraryCategoryLabel(context.l10n, widget.original!.category)),
            Text(context.l10n.deleteSmartAlbumTermImpact),
          ],
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.of(context).pop(false),
            child: Text(context.l10n.cancel),
          ),
          FilledButton(
            key: const Key("classification-term-delete-confirm"),
            onPressed: () => Navigator.of(context).pop(true),
            child: Text(context.l10n.deleteAction),
          ),
        ],
      ),
    );
    if (!mounted || confirmed != true) return;
    await _submit(
      (editor) => editor.remove(
        operationId: "delete-$_operationId",
        original: widget.original!,
      ),
    );
  }

  Future<void> _submit(
    Future<LibraryClassificationTermChange> Function(LibraryClassificationTerms)
    action,
  ) async {
    if (_saving) return;
    setState(() {
      _saving = true;
      _errorCode = null;
    });
    try {
      final editor = ref.read(libraryClassificationTermsProvider);
      if (editor == null) throw StateError("Term editing is unavailable");
      final result = await action(editor);
      if (mounted) Navigator.of(context).pop(result);
    } on Object catch (error) {
      if (mounted) {
        setState(() {
          _saving = false;
          _errorCode = error is LibraryClassificationFailure
              ? error.code
              : "classification_failed";
        });
      }
    }
  }

  String? _validate(String? value, int limit) {
    final text = value?.trim() ?? "";
    if (text.isEmpty) return context.l10n.smartAlbumTermFieldRequired;
    if (text.runes.length > limit || text.runes.any((rune) => rune < 32)) {
      return context.l10n.smartAlbumTermFieldInvalid(limit);
    }
    return null;
  }

  @override
  void dispose() {
    _name.dispose();
    for (final field in _descriptions) {
      field.dispose();
    }
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final original = widget.original;
    return PopScope(
      canPop: !_saving,
      child: AmeDialog(
        key: const Key("classification-term-editor"),
        title: original == null
            ? context.l10n.addSmartAlbumTerm
            : context.l10n.editSmartAlbumTerm,
        scrollable: false,
        content: SingleChildScrollView(
          child: Form(
            key: _form,
            child: AmeDialogBody(
              children: [
                TextFormField(
                  key: const Key("classification-term-name"),
                  controller: _name,
                  autofocus: true,
                  enabled: !_saving,
                  decoration: InputDecoration(
                    labelText: context.l10n.smartAlbumTermName,
                    filled: true,
                  ),
                  validator: (value) => _validate(value, 80),
                  onChanged: (_) => _changed(),
                ),
                Column(
                  mainAxisSize: MainAxisSize.min,
                  crossAxisAlignment: CrossAxisAlignment.stretch,
                  spacing: 8,
                  children: [
                    Row(
                      children: [
                        Expanded(
                          child: Row(
                            children: [
                              Flexible(
                                child: Text(
                                  context.l10n.smartAlbumTermDescriptions,
                                  style: Theme.of(context).textTheme.labelLarge,
                                ),
                              ),
                              const SizedBox(width: 8),
                              AmeTooltip(
                                message: context.l10n.smartAlbumTermEnglishHint,
                                child: Icon(
                                  Symbols.info_rounded,
                                  size: 18,
                                  color: Theme.of(
                                    context,
                                  ).colorScheme.onSurfaceVariant,
                                ),
                              ),
                            ],
                          ),
                        ),
                        if (_descriptions.length < 8)
                          TextButton.icon(
                            key: const Key("classification-description-add"),
                            onPressed: _saving
                                ? null
                                : () {
                                    setState(
                                      () => _descriptions.add(
                                        TextEditingController(),
                                      ),
                                    );
                                    _changed();
                                  },
                            icon: const Icon(Symbols.add_rounded, size: 20),
                            label: Text(context.l10n.addSmartAlbumDescription),
                          ),
                      ],
                    ),
                    for (var index = 0; index < _descriptions.length; index++)
                      Semantics(
                        label: context.l10n.smartAlbumTermDescription(
                          index + 1,
                        ),
                        child: TextFormField(
                          key: ObjectKey(_descriptions[index]),
                          controller: _descriptions[index],
                          enabled: !_saving,
                          minLines: 1,
                          maxLines: 4,
                          decoration: InputDecoration(
                            filled: true,
                            suffixIcon: _descriptions.length > 1
                                ? IconButton(
                                    tooltip: context
                                        .l10n
                                        .removeSmartAlbumDescription,
                                    onPressed: _saving
                                        ? null
                                        : () {
                                            final removed =
                                                _descriptions[index];
                                            setState(
                                              () =>
                                                  _descriptions.removeAt(index),
                                            );
                                            WidgetsBinding.instance
                                                .addPostFrameCallback(
                                                  (_) => removed.dispose(),
                                                );
                                            _changed();
                                          },
                                    icon: const Icon(
                                      Symbols.close_rounded,
                                      size: 20,
                                    ),
                                  )
                                : null,
                          ),
                          validator: (value) {
                            final error = _validate(value, 1024);
                            if (error != null) return error;
                            if (_descriptions
                                    .where(
                                      (field) =>
                                          field.text.trim() == value!.trim(),
                                    )
                                    .length >
                                1) {
                              return context
                                  .l10n
                                  .smartAlbumTermDescriptionDuplicate;
                            }
                            return null;
                          },
                          onChanged: (_) => _changed(),
                        ),
                      ),
                  ],
                ),
                if (_errorCode case final code?)
                  AmeDialogError(switch (code) {
                    "classification_term_changed" =>
                      context.l10n.smartAlbumTermChanged,
                    "classification_configuration_invalid" ||
                    "classification_term_invalid" =>
                      context.l10n.smartAlbumTermInvalid,
                    _ => context.l10n.smartAlbumTermSaveFailed,
                  }),
              ],
            ),
          ),
        ),
        actions: [
          if (original?.defaultCategory != null)
            TextButton(
              key: const Key("classification-term-reset"),
              onPressed: _saving ? null : _reset,
              child: Text(context.l10n.restoreSmartAlbumTerm),
            ),
          if (original?.isCustom == true)
            TextButton(
              key: const Key("classification-term-delete"),
              onPressed: _saving ? null : _delete,
              child: Text(context.l10n.deleteAction),
            ),
          TextButton(
            onPressed: _saving ? null : () => Navigator.of(context).pop(),
            child: Text(context.l10n.cancel),
          ),
          FilledButton(
            key: const Key("classification-term-save"),
            onPressed: _saving ? null : _save,
            child: Text(context.l10n.save),
          ),
        ],
      ),
    );
  }
}
