import "dart:async";

import "package:flutter/material.dart";
import "package:material_symbols_icons/symbols.dart";

import "../../../../app/presentation/ame_scroll_motion.dart";
import "../../domain/library_metadata_filter.dart";
import "../coordination/library_metadata_filter_draft.dart";
import "../library_filter_strings.dart";
import "library_filter_range_choices.dart";

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
  static const _formats = ["jpg", "png", "webp", "gif", "avif"];
  final _form = GlobalKey<FormState>();
  final _cameraExpansion = ExpansibleController();
  late final _initial = LibraryMetadataFilterDraft.fromFilter(widget.filter);
  late final _fields = {
    for (final field in LibraryFilterField.values)
      field: TextEditingController(text: _initial.values[field]),
  };
  late var _sizeUnit = _initial.sizeUnit;
  late var _dateField = _initial.dateField;
  late var _customFormat =
      widget.filter.extension.isNotEmpty &&
      !_formats.contains(widget.filter.extension);
  Map<LibraryFilterField, String> _errors = {};

  @override
  void dispose() {
    for (final controller in _fields.values) {
      controller.dispose();
    }
    _cameraExpansion.dispose();
    super.dispose();
  }

  void _changed() => setState(() => _errors = {});

  void _apply() {
    final result = LibraryMetadataFilterDraft(
      values: {
        for (final entry in _fields.entries) entry.key: entry.value.text,
      },
      sizeUnit: _sizeUnit,
      dateField: _dateField,
    ).parse();
    if (result.filter case final filter?) {
      Navigator.of(context).pop(filter);
      return;
    }
    setState(() => _errors = result.errors);
    if (_errors.containsKey(LibraryFilterField.camera)) {
      _cameraExpansion.expand();
    }
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (!mounted) return;
      final invalid = _form.currentState?.validateGranularly();
      if (invalid != null && invalid.isNotEmpty) {
        unawaited(Scrollable.ensureVisible(invalid.first.context));
      }
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
      setState(() => controller.text = formatLibraryFilterDate(date));
    }
  }

  @override
  Widget build(BuildContext context) => AlertDialog(
    key: const Key("library-metadata-filter-dialog"),
    title: const Text(LibraryFilterStrings.title),
    content: SizedBox(
      width: 520,
      child: AmeScrollMotion(
        child: SingleChildScrollView(
          child: Form(
            key: _form,
            child: Column(
              mainAxisSize: MainAxisSize.min,
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(
                  LibraryFilterStrings.fileFormat,
                  style: Theme.of(context).textTheme.titleSmall,
                ),
                const SizedBox(height: 6),
                Wrap(
                  spacing: 8,
                  runSpacing: 4,
                  children: [
                    for (final format in ["", ..._formats])
                      ChoiceChip(
                        label: Text(
                          format.isEmpty
                              ? LibraryFilterStrings.any
                              : format.toUpperCase(),
                        ),
                        selected:
                            !_customFormat &&
                            _fields[LibraryFilterField.extension]!.text ==
                                format,
                        onSelected: (_) => setState(() {
                          _customFormat = false;
                          _fields[LibraryFilterField.extension]!.text = format;
                        }),
                      ),
                    ChoiceChip(
                      label: const Text(LibraryFilterStrings.otherFormat),
                      selected: _customFormat,
                      onSelected: (_) => setState(() => _customFormat = true),
                    ),
                  ],
                ),
                if (_customFormat) ...[
                  const SizedBox(height: 8),
                  _text(
                    LibraryFilterField.extension,
                    LibraryFilterStrings.extension,
                    hint: LibraryFilterStrings.extensionHint,
                  ),
                ],
                const SizedBox(height: 18),
                Text(
                  LibraryFilterStrings.date,
                  style: Theme.of(context).textTheme.titleSmall,
                ),
                const SizedBox(height: 8),
                SegmentedButton<LibraryDateField>(
                  segments: [
                    for (final field in LibraryDateField.values)
                      ButtonSegment(
                        value: field,
                        label: Text(LibraryFilterStrings.dateFieldName(field)),
                      ),
                  ],
                  selected: {_dateField},
                  onSelectionChanged: (value) =>
                      setState(() => _dateField = value.single),
                ),
                const SizedBox(height: 8),
                Wrap(
                  spacing: 8,
                  runSpacing: 4,
                  children: [
                    _date(
                      LibraryFilterField.dateStart,
                      LibraryFilterStrings.dateStart,
                    ),
                    _date(
                      LibraryFilterField.dateEnd,
                      LibraryFilterStrings.dateEnd,
                    ),
                  ],
                ),
                if (_errors[LibraryFilterField.dateEnd] case final error?)
                  Text(
                    error,
                    style: TextStyle(
                      color: Theme.of(context).colorScheme.error,
                    ),
                  ),
                const SizedBox(height: 18),
                LibraryFilterRangeChoices(
                  label: LibraryFilterStrings.fileSize,
                  minimum: LibraryFilterField.minimumBytes,
                  maximum: LibraryFilterField.maximumBytes,
                  controllers: _fields,
                  errors: _errors,
                  presets: const [
                    (label: LibraryFilterStrings.any, minimum: "", maximum: ""),
                    (label: "≤ 1 MB", minimum: "", maximum: "1"),
                    (label: "1–10 MB", minimum: "1", maximum: "10"),
                    (label: "≥ 10 MB", minimum: "10", maximum: ""),
                  ],
                  unit: _sizeUnit.label,
                  presetUnitMatches:
                      _sizeUnit == LibraryFilterSizeUnit.megabytes,
                  onPresetSelected: () =>
                      _sizeUnit = LibraryFilterSizeUnit.megabytes,
                  onChanged: _changed,
                  unitPicker: SegmentedButton<LibraryFilterSizeUnit>(
                    segments: [
                      for (final unit in LibraryFilterSizeUnit.values)
                        ButtonSegment(value: unit, label: Text(unit.label)),
                    ],
                    selected: {_sizeUnit},
                    onSelectionChanged: (value) =>
                        setState(() => _sizeUnit = value.single),
                  ),
                ),
                const SizedBox(height: 18),
                _dimension(
                  LibraryFilterStrings.width,
                  LibraryFilterField.minimumWidth,
                  LibraryFilterField.maximumWidth,
                  1920,
                  3840,
                ),
                const SizedBox(height: 18),
                _dimension(
                  LibraryFilterStrings.height,
                  LibraryFilterField.minimumHeight,
                  LibraryFilterField.maximumHeight,
                  1080,
                  2160,
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

  Widget _dimension(
    String label,
    LibraryFilterField minimum,
    LibraryFilterField maximum,
    int lower,
    int upper,
  ) => LibraryFilterRangeChoices(
    label: label,
    minimum: minimum,
    maximum: maximum,
    controllers: _fields,
    errors: _errors,
    presets: [
      (label: LibraryFilterStrings.any, minimum: "", maximum: ""),
      (label: "≥ $lower px", minimum: "$lower", maximum: ""),
      (label: "≥ $upper px", minimum: "$upper", maximum: ""),
    ],
    onChanged: _changed,
  );

  Widget _date(LibraryFilterField field, String label) {
    final value = _fields[field]!.text;
    return InputChip(
      key: ValueKey("library-filter-${field.name}"),
      avatar: const Icon(Symbols.calendar_month_rounded, size: 18),
      label: Text(value.isEmpty ? label : "$label $value"),
      onPressed: () => _pickDate(field, label),
      onDeleted: value.isEmpty
          ? null
          : () => setState(() => _fields[field]!.clear()),
      deleteButtonTooltipMessage: "清除$label",
    );
  }

  Widget _text(LibraryFilterField field, String label, {String? hint}) =>
      TextFormField(
        key: ValueKey("library-filter-${field.name}"),
        controller: _fields[field],
        textInputAction: TextInputAction.next,
        decoration: InputDecoration(labelText: label, hintText: hint),
        validator: (_) => _errors[field],
      );
}
