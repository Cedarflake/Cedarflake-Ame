import "package:flutter/material.dart";
import "package:material_symbols_icons/symbols.dart";

import "../../domain/library_metadata_filter.dart";

Future<LibraryMetadataFilter?> showLibraryMetadataFilterEditor({
  required BuildContext context,
  required LibraryMetadataFilter filter,
}) => showDialog<LibraryMetadataFilter>(
  context: context,
  builder: (_) => LibraryMetadataFilterEditor(filter: filter),
);

class LibraryMetadataFilterEditor extends StatefulWidget {
  const LibraryMetadataFilterEditor({required this.filter, super.key});
  final LibraryMetadataFilter filter;

  @override
  State<LibraryMetadataFilterEditor> createState() =>
      _LibraryMetadataFilterEditorState();
}

class _LibraryMetadataFilterEditorState
    extends State<LibraryMetadataFilterEditor> {
  final _form = GlobalKey<FormState>();
  late final Map<String, TextEditingController> _fields;
  late LibraryDateField _dateField;

  @override
  void initState() {
    super.initState();
    final filter = widget.filter;
    _dateField = filter.date.field;
    final values = {
      "extension": filter.extension,
      "minimum-bytes": _sizeText(filter.bytes.minimum),
      "maximum-bytes": _sizeText(filter.bytes.maximum),
      "minimum-width": filter.width.minimum?.toString() ?? "",
      "maximum-width": filter.width.maximum?.toString() ?? "",
      "minimum-height": filter.height.minimum?.toString() ?? "",
      "maximum-height": filter.height.maximum?.toString() ?? "",
      "start": filter.date.start ?? "",
      "end": filter.date.end ?? "",
      "camera": filter.cameraText,
    };
    _fields = values.map(
      (key, value) => MapEntry(key, TextEditingController(text: value)),
    );
  }

  @override
  void dispose() {
    for (final controller in _fields.values) {
      controller.dispose();
    }
    super.dispose();
  }

  String _text(String field) => _fields[field]!.text.trim();

  LibraryNumberRange _range(String name, {bool size = false}) =>
      LibraryNumberRange(
        minimum: _number(_text("minimum-$name"), size: size),
        maximum: _number(_text("maximum-$name"), size: size),
      );

  void _apply() {
    if (!_form.currentState!.validate()) return;
    Navigator.of(context).pop(
      LibraryMetadataFilter(
        extension: _text("extension"),
        bytes: _range("bytes", size: true),
        width: _range("width"),
        height: _range("height"),
        date: LibraryDateRange(
          field: _dateField,
          start: _text("start").isEmpty ? null : _text("start"),
          end: _text("end").isEmpty ? null : _text("end"),
        ),
        cameraText: _text("camera"),
      ).normalized,
    );
  }

  Future<void> _pickDate(String field) async {
    final date = await showDatePicker(
      context: context,
      initialDate: _date(_text(field)) ?? DateTime.now(),
      firstDate: DateTime(1),
      lastDate: DateTime(9999, 12, 31),
    );
    if (date == null || !mounted) return;
    _fields[field]!.text =
        "${date.year.toString().padLeft(4, '0')}-${date.month.toString().padLeft(2, '0')}-${date.day.toString().padLeft(2, '0')}";
  }

  @override
  Widget build(BuildContext context) => AlertDialog(
    key: const Key("library-metadata-filter-editor"),
    title: const Text("筛选条件"),
    scrollable: true,
    content: SizedBox(
      width: 560,
      child: Form(
        key: _form,
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            _pair(
              _field(
                "extension",
                "扩展名",
                hint: "jpg、png 等，填写一种",
                validator: (_) {
                  final extension = _text(
                    "extension",
                  ).replaceFirst(RegExp(r"^\.+"), "");
                  return RegExp(r"^[a-zA-Z0-9]{0,16}$").hasMatch(extension)
                      ? null
                      : "填写一种扩展名";
                },
              ),
              _field(
                "camera",
                "相机品牌或型号",
                validator: (_) {
                  final value = _text("camera");
                  return value.runes.length <= 128 &&
                          !RegExp(r"[\x00-\x1F\x7F-\x9F]").hasMatch(value)
                      ? null
                      : "使用不超过 128 个字符的相机名称";
                },
              ),
            ),
            const SizedBox(height: 16),
            _numberPair("bytes", "文件大小（MB）", size: true),
            const SizedBox(height: 16),
            _numberPair("width", "宽度（像素）"),
            const SizedBox(height: 16),
            _numberPair("height", "高度（像素）"),
            const SizedBox(height: 16),
            DropdownButtonFormField<LibraryDateField>(
              key: const Key("library-filter-date-field"),
              initialValue: _dateField,
              isExpanded: true,
              decoration: const InputDecoration(labelText: "日期依据"),
              items: const [
                DropdownMenuItem(
                  value: LibraryDateField.capture,
                  child: Text("拍摄日期"),
                ),
                DropdownMenuItem(
                  value: LibraryDateField.created,
                  child: Text("创建日期"),
                ),
                DropdownMenuItem(
                  value: LibraryDateField.modified,
                  child: Text("修改日期"),
                ),
              ],
              onChanged: (value) {
                if (value != null) setState(() => _dateField = value);
              },
            ),
            const SizedBox(height: 16),
            _pair(_dateInput("start", "开始日期"), _dateInput("end", "结束日期")),
          ],
        ),
      ),
    ),
    actions: [
      TextButton(
        key: const Key("library-filter-clear"),
        onPressed: () =>
            Navigator.of(context).pop(const LibraryMetadataFilter()),
        child: const Text("清除条件"),
      ),
      TextButton(
        onPressed: () => Navigator.of(context).pop(),
        child: const Text("取消"),
      ),
      FilledButton(
        key: const Key("library-filter-apply"),
        onPressed: _apply,
        child: const Text("应用"),
      ),
    ],
  );

  Widget _numberPair(String name, String label, {bool size = false}) => _pair(
    _numberInput("minimum-$name", "最小$label", size: size),
    _numberInput(
      "maximum-$name",
      "最大$label",
      size: size,
      minimum: "minimum-$name",
    ),
  );

  Widget _numberInput(
    String field,
    String label, {
    bool size = false,
    String? minimum,
  }) => _field(
    field,
    label,
    keyboardType: TextInputType.numberWithOptions(decimal: size),
    validator: (_) {
      final value = _text(field);
      if (value.isEmpty) return null;
      final number = _number(value, size: size);
      if (number == null) return size ? "填写非负数，最多 6 位小数" : "填写非负整数";
      final lower = minimum == null
          ? null
          : _number(_text(minimum), size: size);
      return lower != null && number < lower ? "最大值不能小于最小值" : null;
    },
  );

  Widget _dateInput(String field, String label) => _field(
    field,
    label,
    hint: "YYYY-MM-DD",
    suffix: IconButton(
      tooltip: "选择$label",
      icon: const Icon(Symbols.calendar_month_rounded),
      onPressed: () => _pickDate(field),
    ),
    validator: (_) {
      final value = _text(field);
      if (value.isEmpty) return null;
      final date = _date(value);
      if (date == null) return "填写有效日期，例如 2026-10-30";
      final start = _date(_text("start"));
      return field == "end" && start != null && date.isBefore(start)
          ? "结束日期不能早于开始日期"
          : null;
    },
  );

  Widget _field(
    String field,
    String label, {
    String? hint,
    Widget? suffix,
    TextInputType? keyboardType,
    FormFieldValidator<String>? validator,
  }) => TextFormField(
    key: Key("library-filter-$field"),
    controller: _fields[field],
    decoration: InputDecoration(
      labelText: label,
      hintText: hint,
      suffixIcon: suffix,
    ),
    keyboardType: keyboardType,
    validator: validator,
    onFieldSubmitted: (_) => _apply(),
  );

  Widget _pair(Widget first, Widget second) => Row(
    crossAxisAlignment: CrossAxisAlignment.start,
    children: [
      Expanded(child: first),
      const SizedBox(width: 16),
      Expanded(child: second),
    ],
  );
}

int? _number(String text, {bool size = false}) {
  if (!RegExp(size ? r"^\d+(\.\d{1,6})?$" : r"^\d+$").hasMatch(text)) {
    return null;
  }
  final parts = text.split(".");
  final whole = BigInt.tryParse(parts[0]);
  if (whole == null) return null;
  final value = size
      ? whole * BigInt.from(1000000) +
            BigInt.parse(parts.length == 1 ? "0" : parts[1].padRight(6, "0"))
      : whole;
  return value > BigInt.from(0x7fffffffffffffff) ? null : value.toInt();
}

String _sizeText(int? bytes) {
  if (bytes == null) return "";
  final fraction = (bytes % 1000000)
      .toString()
      .padLeft(6, "0")
      .replaceFirst(RegExp(r"0+$"), "");
  return fraction.isEmpty
      ? "${bytes ~/ 1000000}"
      : "${bytes ~/ 1000000}.$fraction";
}

DateTime? _date(String text) {
  if (!RegExp(r"^\d{4}-\d{2}-\d{2}$").hasMatch(text)) return null;
  final date = DateTime.tryParse(text);
  if (date == null || date.year < 1 || date.year > 9999) return null;
  final parts = text.split("-").map(int.parse).toList();
  return date.year == parts[0] && date.month == parts[1] && date.day == parts[2]
      ? date
      : null;
}
