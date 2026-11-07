import "dart:async";

import "package:flutter/material.dart";
import "package:material_symbols_icons/symbols.dart";

import "../../../../app/presentation/ame_overlay_semantics.dart";
import "../../../../app/presentation/ame_scroll_motion.dart";
import "../../domain/library_metadata_filter.dart";
import "../coordination/library_metadata_filter_draft.dart";
import "../library_filter_strings.dart";

Future<LibraryMetadataFilter?> showLibraryMetadataFilterDialog(
  BuildContext context,
  LibraryMetadataFilter filter,
) => showDialog<LibraryMetadataFilter>(
  context: context,
  builder: (_) => _LibraryMetadataFilterDialog(filter: filter),
);

class _LibraryMetadataFilterDialog extends StatefulWidget {
  const _LibraryMetadataFilterDialog({required this.filter});
  final LibraryMetadataFilter filter;

  @override
  State<_LibraryMetadataFilterDialog> createState() =>
      _LibraryMetadataFilterDialogState();
}

class _LibraryMetadataFilterDialogState
    extends State<_LibraryMetadataFilterDialog> {
  final _form = GlobalKey<FormState>();
  final _cameraExpansion = ExpansibleController();
  late final _initial = LibraryMetadataFilterDraft.fromFilter(widget.filter);
  late final _fields = {
    for (final field in LibraryFilterField.values)
      field: TextEditingController(text: _initial.values[field]),
  };
  late var _sizeUnit = _initial.sizeUnit;
  late var _dateField = _initial.dateField;
  Map<LibraryFilterField, String> _errors = {};

  @override
  void dispose() {
    for (final controller in _fields.values) {
      controller.dispose();
    }
    _cameraExpansion.dispose();
    super.dispose();
  }

  void _apply() {
    final result = LibraryMetadataFilterDraft(
      values: {
        for (final entry in _fields.entries) entry.key: entry.value.text,
      },
      sizeUnit: _sizeUnit,
      dateField: _dateField,
    ).parse();
    _errors = result.errors;
    final invalid = _form.currentState!.validateGranularly();
    if (invalid.isEmpty && result.filter != null) {
      Navigator.of(context).pop(result.filter);
      return;
    }
    if (_errors.containsKey(LibraryFilterField.camera)) {
      _cameraExpansion.expand();
    }
    if (invalid.isEmpty) return;
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (!mounted || !invalid.first.mounted) return;
      unawaited(Scrollable.ensureVisible(invalid.first.context));
    });
  }

  Future<void> _pickDate(LibraryFilterField field, String label) async {
    final controller = _fields[field]!;
    final date = await showDatePicker(
      context: context,
      helpText: label,
      initialDate:
          parseLibraryFilterDate(controller.text.trim()) ?? DateTime.now(),
      firstDate: DateTime(1),
      lastDate: DateTime(9999, 12, 31),
    );
    if (mounted && date != null) {
      controller.text = formatLibraryFilterDate(date);
    }
  }

  @override
  Widget build(BuildContext context) => AlertDialog(
    key: const Key("library-metadata-filter-dialog"),
    title: const Text(LibraryFilterStrings.title),
    content: SizedBox(
      width: 600,
      child: AmeScrollMotion(
        child: SingleChildScrollView(
          child: Form(
            key: _form,
            child: Column(
              mainAxisSize: MainAxisSize.min,
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                _heading(LibraryFilterStrings.date),
                AmeOverlayTraversalBoundary(
                  child: DropdownMenu<LibraryDateField>(
                    key: const Key("library-filter-date-field"),
                    label: const Text(LibraryFilterStrings.dateField),
                    initialSelection: _dateField,
                    selectOnly: true,
                    dropdownMenuEntries: [
                      for (final field in LibraryDateField.values)
                        DropdownMenuEntry(
                          value: field,
                          label: LibraryFilterStrings.dateFieldName(field),
                        ),
                    ],
                    onSelected: (value) {
                      if (value != null) _dateField = value;
                    },
                  ),
                ),
                const SizedBox(height: 16),
                Row(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Expanded(
                      child: _date(
                        LibraryFilterField.dateStart,
                        LibraryFilterStrings.dateStart,
                      ),
                    ),
                    const SizedBox(width: 16),
                    Expanded(
                      child: _date(
                        LibraryFilterField.dateEnd,
                        LibraryFilterStrings.dateEnd,
                      ),
                    ),
                  ],
                ),
                _heading(LibraryFilterStrings.file),
                _text(
                  LibraryFilterField.extension,
                  LibraryFilterStrings.extension,
                  hint: LibraryFilterStrings.extensionHint,
                ),
                const SizedBox(height: 16),
                Row(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Expanded(
                      child: _text(
                        LibraryFilterField.minimumBytes,
                        LibraryFilterStrings.minimumBytes,
                        number: true,
                      ),
                    ),
                    const SizedBox(width: 16),
                    Expanded(
                      child: _text(
                        LibraryFilterField.maximumBytes,
                        LibraryFilterStrings.maximumBytes,
                        number: true,
                      ),
                    ),
                    const SizedBox(width: 16),
                    AmeOverlayTraversalBoundary(
                      child: DropdownMenu<LibraryFilterSizeUnit>(
                        key: const Key("library-filter-size-unit"),
                        width: 100,
                        label: const Text(LibraryFilterStrings.unit),
                        initialSelection: _sizeUnit,
                        selectOnly: true,
                        dropdownMenuEntries: [
                          for (final unit in LibraryFilterSizeUnit.values)
                            DropdownMenuEntry(value: unit, label: unit.label),
                        ],
                        onSelected: (value) {
                          if (value != null) _sizeUnit = value;
                        },
                      ),
                    ),
                  ],
                ),
                _heading(LibraryFilterStrings.dimensions),
                _dimensions(
                  LibraryFilterField.minimumWidth,
                  LibraryFilterField.maximumWidth,
                  LibraryFilterStrings.minimumWidth,
                  LibraryFilterStrings.maximumWidth,
                ),
                const SizedBox(height: 16),
                _dimensions(
                  LibraryFilterField.minimumHeight,
                  LibraryFilterField.maximumHeight,
                  LibraryFilterStrings.minimumHeight,
                  LibraryFilterStrings.maximumHeight,
                ),
                const SizedBox(height: 8),
                ExpansionTile(
                  controller: _cameraExpansion,
                  title: const Text(LibraryFilterStrings.camera),
                  tilePadding: EdgeInsets.zero,
                  initiallyExpanded: widget.filter.cameraText.isNotEmpty,
                  maintainState: true,
                  childrenPadding: const EdgeInsets.only(bottom: 8),
                  children: [
                    _text(
                      LibraryFilterField.camera,
                      LibraryFilterStrings.cameraHint,
                    ),
                  ],
                ),
              ],
            ),
          ),
        ),
      ),
    ),
    actions: [
      TextButton(
        onPressed: () =>
            Navigator.of(context).pop(const LibraryMetadataFilter()),
        child: const Text(LibraryFilterStrings.clear),
      ),
      TextButton(
        onPressed: () => Navigator.of(context).pop(),
        child: const Text(LibraryFilterStrings.cancel),
      ),
      FilledButton(
        onPressed: _apply,
        child: const Text(LibraryFilterStrings.apply),
      ),
    ],
  );

  Widget _heading(String label) => Padding(
    padding: const EdgeInsets.only(top: 16, bottom: 12),
    child: Text(label, style: Theme.of(context).textTheme.titleSmall),
  );

  Widget _date(LibraryFilterField field, String label) => _text(
    field,
    label,
    hint: LibraryFilterStrings.dateHint,
    suffix: AmeTooltip(
      message: "选择$label",
      child: IconButton(
        onPressed: () => _pickDate(field, label),
        icon: const Icon(Symbols.calendar_month_rounded),
      ),
    ),
  );

  Widget _dimensions(
    LibraryFilterField minimum,
    LibraryFilterField maximum,
    String minimumLabel,
    String maximumLabel,
  ) => Row(
    crossAxisAlignment: CrossAxisAlignment.start,
    children: [
      Expanded(child: _text(minimum, minimumLabel, number: true, unit: "px")),
      const SizedBox(width: 16),
      Expanded(child: _text(maximum, maximumLabel, number: true, unit: "px")),
    ],
  );

  Widget _text(
    LibraryFilterField field,
    String label, {
    String? hint,
    bool number = false,
    String? unit,
    Widget? suffix,
  }) => TextFormField(
    key: ValueKey("library-filter-${field.name}"),
    controller: _fields[field],
    keyboardType: number
        ? const TextInputType.numberWithOptions(decimal: true)
        : null,
    textInputAction: TextInputAction.next,
    decoration: InputDecoration(
      labelText: label,
      hintText: hint,
      suffixText: unit,
      suffixIcon: suffix,
    ),
    validator: (_) => _errors[field],
  );
}
