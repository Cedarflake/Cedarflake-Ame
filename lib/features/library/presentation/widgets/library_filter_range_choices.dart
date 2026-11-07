import "package:flutter/material.dart";

import "../coordination/library_metadata_filter_draft.dart";
import "../library_filter_strings.dart";

typedef LibraryFilterRangePreset = ({
  String label,
  String minimum,
  String maximum,
});

class LibraryFilterRangeChoices extends StatefulWidget {
  const LibraryFilterRangeChoices({
    required this.label,
    required this.minimum,
    required this.maximum,
    required this.controllers,
    required this.errors,
    required this.presets,
    required this.onChanged,
    this.unit = "px",
    this.unitPicker,
    this.presetUnitMatches = true,
    this.onPresetSelected,
    super.key,
  });

  final String label;
  final LibraryFilterField minimum;
  final LibraryFilterField maximum;
  final Map<LibraryFilterField, TextEditingController> controllers;
  final Map<LibraryFilterField, String> errors;
  final List<LibraryFilterRangePreset> presets;
  final VoidCallback onChanged;
  final String unit;
  final Widget? unitPicker;
  final bool presetUnitMatches;
  final VoidCallback? onPresetSelected;

  @override
  State<LibraryFilterRangeChoices> createState() =>
      _LibraryFilterRangeChoicesState();
}

class _LibraryFilterRangeChoicesState extends State<LibraryFilterRangeChoices> {
  bool _custom = false;

  @override
  Widget build(BuildContext context) {
    final minimum = widget.controllers[widget.minimum]!;
    final maximum = widget.controllers[widget.maximum]!;
    final selected = widget.presets.indexWhere(
      (preset) =>
          (widget.presetUnitMatches ||
              (preset.minimum.isEmpty && preset.maximum.isEmpty)) &&
          minimum.text == preset.minimum &&
          maximum.text == preset.maximum,
    );
    final custom =
        _custom ||
        selected < 0 ||
        widget.errors.containsKey(widget.minimum) ||
        widget.errors.containsKey(widget.maximum);
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Text(widget.label, style: Theme.of(context).textTheme.titleSmall),
        const SizedBox(height: 6),
        Wrap(
          spacing: 8,
          runSpacing: 4,
          children: [
            for (var index = 0; index < widget.presets.length; index++)
              ChoiceChip(
                label: Text(widget.presets[index].label),
                selected: !custom && selected == index,
                onSelected: (_) {
                  setState(() => _custom = false);
                  minimum.text = widget.presets[index].minimum;
                  maximum.text = widget.presets[index].maximum;
                  widget.onPresetSelected?.call();
                  widget.onChanged();
                },
              ),
            ChoiceChip(
              key: ValueKey("library-filter-custom-${widget.minimum.name}"),
              label: const Text(LibraryFilterStrings.custom),
              selected: custom,
              onSelected: (_) => setState(() => _custom = true),
            ),
          ],
        ),
        if (custom) ...[
          const SizedBox(height: 8),
          Row(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Expanded(
                child: _input(widget.minimum, LibraryFilterStrings.minimum),
              ),
              const SizedBox(width: 12),
              Expanded(
                child: _input(widget.maximum, LibraryFilterStrings.maximum),
              ),
            ],
          ),
          if (widget.unitPicker case final picker?) ...[
            const SizedBox(height: 8),
            picker,
          ],
        ],
      ],
    );
  }

  Widget _input(LibraryFilterField field, String label) => TextFormField(
    key: ValueKey("library-filter-${field.name}"),
    controller: widget.controllers[field],
    keyboardType: const TextInputType.numberWithOptions(decimal: true),
    textInputAction: TextInputAction.next,
    decoration: InputDecoration(labelText: label, suffixText: widget.unit),
    validator: (_) => widget.errors[field],
    onChanged: (_) => widget.onChanged(),
  );
}
