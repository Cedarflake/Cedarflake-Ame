import "dart:async";

import "package:flutter/material.dart";
import "package:material_symbols_icons/symbols.dart";

import "../../../../app/presentation/ame_choice.dart";
import "../../../../app/presentation/ame_scroll_motion.dart";
import "../../domain/library_metadata_filter.dart";
import "../coordination/library_metadata_filter_draft.dart";
import "../library_filter_strings.dart";
import "library_filter_format_choices.dart";
import "library_filter_range_fields.dart";

Future<LibraryMetadataFilter?> showLibraryMetadataFilterSheet(
  BuildContext context,
  LibraryMetadataFilter filter,
) => showModalBottomSheet<LibraryMetadataFilter>(
  context: context,
  showDragHandle: true,
  isScrollControlled: true,
  useSafeArea: true,
  enableDrag: false,
  builder: (_) => LibraryMetadataFilterSheet(filter: filter),
);

class LibraryMetadataFilterSheet extends StatefulWidget {
  const LibraryMetadataFilterSheet({required this.filter, super.key});
  final LibraryMetadataFilter filter;

  @override
  State<LibraryMetadataFilterSheet> createState() =>
      _LibraryMetadataFilterSheetState();
}

class _LibraryMetadataFilterSheetState
    extends State<LibraryMetadataFilterSheet> {
  final _form = GlobalKey<FormState>();
  late final _initial = LibraryMetadataFilterDraft.fromFilter(widget.filter);
  late final _fields = {
    for (final field in LibraryFilterField.values)
      field: TextEditingController(
        text: field == LibraryFilterField.extension
            ? widget.filter.extensions
                  .where(
                    (value) =>
                        !LibraryFilterFormatChoices.formats.contains(value),
                  )
                  .join(", ")
            : _initial.values[field],
      ),
  };
  late final _formats = widget.filter.extensions
      .where(LibraryFilterFormatChoices.formats.contains)
      .toSet();
  late var _sizeUnit = _initial.sizeUnit;
  late var _dateField = _initial.dateField;
  late var _showCustomFormats =
      _fields[LibraryFilterField.extension]!.text.isNotEmpty;
  Map<LibraryFilterField, String> _errors = {};

  @override
  void dispose() {
    for (final controller in _fields.values) {
      controller.dispose();
    }
    super.dispose();
  }

  void _changed() => setState(() => _errors = {});

  void _apply() {
    final result = LibraryMetadataFilterDraft(
      values: {
        for (final entry in _fields.entries) entry.key: entry.value.text,
        LibraryFilterField.extension: [
          ..._formats,
          _fields[LibraryFilterField.extension]!.text,
        ].join(","),
      },
      sizeUnit: _sizeUnit,
      dateField: _dateField,
    ).parse();
    if (result.filter case final filter?) {
      Navigator.of(context).pop(filter);
      return;
    }
    setState(() {
      _errors = result.errors;
      if (_errors.containsKey(LibraryFilterField.extension)) {
        _showCustomFormats = true;
      }
    });
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
      setState(() {
        controller.text = formatLibraryFilterDate(date);
        _errors = {};
      });
    }
  }

  @override
  Widget build(BuildContext context) => SafeArea(
    child: Padding(
      padding: const EdgeInsets.fromLTRB(24, 8, 24, 24),
      child: ConstrainedBox(
        constraints: BoxConstraints(
          maxHeight: MediaQuery.sizeOf(context).height * .8,
        ),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            Row(
              children: [
                Expanded(
                  child: Text(
                    LibraryFilterStrings.title,
                    style: Theme.of(context).textTheme.titleLarge,
                  ),
                ),
                IconButton(
                  tooltip: MaterialLocalizations.of(context).closeButtonTooltip,
                  onPressed: () => Navigator.of(context).pop(),
                  icon: const Icon(Symbols.close_rounded),
                ),
              ],
            ),
            const SizedBox(height: 16),
            Flexible(
              child: AmeScrollMotion(
                child: ScrollbarTheme(
                  data: ScrollbarTheme.of(context).copyWith(
                    thumbVisibility: const WidgetStatePropertyAll(true),
                  ),
                  child: SingleChildScrollView(
                    padding: const EdgeInsetsDirectional.only(end: 24),
                    child: Form(
                      key: _form,
                      child: Column(
                        crossAxisAlignment: CrossAxisAlignment.start,
                        children: [
                          _section(
                            LibraryFilterStrings.fileFormat,
                            LibraryFilterFormatChoices(
                              selected: _formats,
                              onToggled: (value) => setState(() {
                                if (!_formats.remove(value)) {
                                  _formats.add(value);
                                }
                                _errors = {};
                              }),
                              customController:
                                  _fields[LibraryFilterField.extension]!,
                              showCustom: _showCustomFormats,
                              onShowCustomChanged: (value) => setState(() {
                                _showCustomFormats = value;
                                if (!value) {
                                  _fields[LibraryFilterField.extension]!
                                      .clear();
                                }
                                _errors = {};
                              }),
                              error: _errors[LibraryFilterField.extension],
                            ),
                          ),
                          const SizedBox(height: 20),
                          _section(LibraryFilterStrings.date, _dates()),
                          const SizedBox(height: 20),
                          _section(
                            LibraryFilterStrings.dimensions,
                            Column(
                              children: [
                                _range(
                                  LibraryFilterField.minimumWidth,
                                  LibraryFilterField.maximumWidth,
                                  LibraryFilterStrings.minimumWidth,
                                  LibraryFilterStrings.maximumWidth,
                                ),
                                const SizedBox(height: 16),
                                _range(
                                  LibraryFilterField.minimumHeight,
                                  LibraryFilterField.maximumHeight,
                                  LibraryFilterStrings.minimumHeight,
                                  LibraryFilterStrings.maximumHeight,
                                ),
                              ],
                            ),
                          ),
                          const SizedBox(height: 20),
                          _section(
                            LibraryFilterStrings.fileSize,
                            Row(
                              crossAxisAlignment: CrossAxisAlignment.start,
                              children: [
                                Expanded(
                                  child: _range(
                                    LibraryFilterField.minimumBytes,
                                    LibraryFilterField.maximumBytes,
                                    LibraryFilterStrings.minimum,
                                    LibraryFilterStrings.maximum,
                                    unit: _sizeUnit.label,
                                  ),
                                ),
                                const SizedBox(width: 12),
                                AmeChoice<LibraryFilterSizeUnit>(
                                  width: 112,
                                  label: LibraryFilterStrings.unit,
                                  value: _sizeUnit,
                                  entries: [
                                    for (final unit
                                        in LibraryFilterSizeUnit.values)
                                      AmeChoiceEntry(
                                        value: unit,
                                        label: unit.label,
                                      ),
                                  ],
                                  onSelected: (value) {
                                    if (value != null) {
                                      setState(() {
                                        _sizeUnit = value;
                                        _errors = {};
                                      });
                                    }
                                  },
                                ),
                              ],
                            ),
                          ),
                          const SizedBox(height: 20),
                          _section(
                            LibraryFilterStrings.camera,
                            TextFormField(
                              key: const ValueKey("library-filter-camera"),
                              controller: _fields[LibraryFilterField.camera],
                              textInputAction: TextInputAction.done,
                              decoration: const InputDecoration(
                                hintText: LibraryFilterStrings.cameraHint,
                                border: OutlineInputBorder(),
                              ),
                              validator: (_) =>
                                  _errors[LibraryFilterField.camera],
                            ),
                          ),
                          const SizedBox(height: 4),
                        ],
                      ),
                    ),
                  ),
                ),
              ),
            ),
            const SizedBox(height: 16),
            Row(
              children: [
                TextButton(
                  onPressed: () =>
                      Navigator.of(context).pop(const LibraryMetadataFilter()),
                  child: const Text(LibraryFilterStrings.clear),
                ),
                const Spacer(),
                TextButton(
                  onPressed: () => Navigator.of(context).pop(),
                  child: const Text(LibraryFilterStrings.cancel),
                ),
                const SizedBox(width: 8),
                FilledButton(
                  onPressed: _apply,
                  child: const Text(LibraryFilterStrings.apply),
                ),
              ],
            ),
          ],
        ),
      ),
    ),
  );
  Widget _section(String label, Widget child) => Column(
    crossAxisAlignment: CrossAxisAlignment.start,
    children: [
      Text(label, style: Theme.of(context).textTheme.titleSmall),
      const SizedBox(height: 12),
      child,
    ],
  );

  Widget _range(
    LibraryFilterField minimum,
    LibraryFilterField maximum,
    String minimumLabel,
    String maximumLabel, {
    String unit = "px",
  }) => LibraryFilterRangeFields(
    minimum: minimum,
    maximum: maximum,
    minimumLabel: minimumLabel,
    maximumLabel: maximumLabel,
    controllers: _fields,
    errors: _errors,
    onChanged: _changed,
    unit: unit,
  );

  Widget _dates() => Row(
    crossAxisAlignment: CrossAxisAlignment.start,
    children: [
      AmeChoice<LibraryDateField>(
        width: 168,
        value: _dateField,
        entries: [
          for (final field in LibraryDateField.values)
            AmeChoiceEntry(
              value: field,
              label: LibraryFilterStrings.dateFieldName(field),
            ),
        ],
        onSelected: (value) {
          if (value != null) setState(() => _dateField = value);
        },
      ),
      const SizedBox(width: 12),
      Expanded(
        child: _date(
          LibraryFilterField.dateStart,
          LibraryFilterStrings.dateStart,
        ),
      ),
      const SizedBox(width: 12),
      Expanded(
        child: _date(LibraryFilterField.dateEnd, LibraryFilterStrings.dateEnd),
      ),
    ],
  );
  Widget _date(LibraryFilterField field, String label) {
    final controller = _fields[field]!;
    return TextFormField(
      key: ValueKey("library-filter-${field.name}"),
      controller: controller,
      readOnly: true,
      onTap: () => _pickDate(field, label),
      validator: (_) => _errors[field],
      decoration: InputDecoration(
        hintText: label,
        border: const OutlineInputBorder(),
        suffixIcon: controller.text.isEmpty
            ? IconButton(
                tooltip: label,
                icon: const Icon(Symbols.calendar_month_rounded),
                onPressed: () => _pickDate(field, label),
              )
            : IconButton(
                tooltip: "清除$label",
                icon: const Icon(Symbols.close_rounded),
                onPressed: () => setState(() {
                  controller.clear();
                  _errors = {};
                }),
              ),
      ),
    );
  }
}
