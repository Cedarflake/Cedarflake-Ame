import "package:material_ui/material_ui.dart";

import "../coordination/library_metadata_filter_draft.dart";

class LibraryFilterRangeFields extends StatelessWidget {
  const LibraryFilterRangeFields({
    required this.minimum,
    required this.maximum,
    required this.minimumLabel,
    required this.maximumLabel,
    required this.controllers,
    required this.errors,
    required this.onChanged,
    this.unit = "px",
    super.key,
  });

  final LibraryFilterField minimum;
  final LibraryFilterField maximum;
  final String minimumLabel;
  final String maximumLabel;
  final Map<LibraryFilterField, TextEditingController> controllers;
  final Map<LibraryFilterField, String> errors;
  final VoidCallback onChanged;
  final String unit;

  @override
  Widget build(BuildContext context) => Row(
    crossAxisAlignment: CrossAxisAlignment.start,
    children: [
      Expanded(child: _input(minimum, minimumLabel)),
      const SizedBox(width: 12),
      Expanded(child: _input(maximum, maximumLabel)),
    ],
  );

  Widget _input(LibraryFilterField field, String label) => TextFormField(
    key: ValueKey("library-filter-${field.name}"),
    controller: controllers[field],
    keyboardType: const TextInputType.numberWithOptions(decimal: true),
    textInputAction: TextInputAction.next,
    decoration: InputDecoration(
      labelText: label,
      suffixText: unit,
      border: const OutlineInputBorder(),
    ),
    validator: (_) => errors[field],
    onChanged: (_) => onChanged(),
  );
}
