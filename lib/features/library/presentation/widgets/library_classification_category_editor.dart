import "dart:math";

import "package:flutter/material.dart";

import "../../application/library_classification_configuration.dart";

Future<LibraryClassificationCategory?> showLibraryClassificationCategoryEditor(
  BuildContext context,
  LibraryClassificationCategory? category,
) => showDialog<LibraryClassificationCategory>(
  context: context,
  builder: (context) => _CategoryEditor(category),
);

class _CategoryEditor extends StatefulWidget {
  const _CategoryEditor(this.category);
  final LibraryClassificationCategory? category;

  @override
  State<_CategoryEditor> createState() => _CategoryEditorState();
}

class _CategoryEditorState extends State<_CategoryEditor> {
  late final _name = TextEditingController(text: widget.category?.label);
  late final _descriptions = TextEditingController(
    text: widget.category?.descriptions.join("\n"),
  );
  String? _error;

  @override
  void dispose() {
    _name.dispose();
    _descriptions.dispose();
    super.dispose();
  }

  void _save() {
    final name = _name.text.trim();
    final descriptions = _descriptions.text
        .split("\n")
        .map((line) => line.trim())
        .where((line) => line.isNotEmpty)
        .toList();
    if (name.isEmpty || name.runes.length > 80) {
      setState(() => _error = "分类名称需要 1–80 个字。");
      return;
    }
    if (descriptions.isEmpty ||
        descriptions.length > 8 ||
        descriptions.any((text) => text.runes.length > 1024) ||
        descriptions.toSet().length != descriptions.length) {
      setState(() => _error = "填写 1–8 条不同的描述，每条不超过 1024 个字。");
      return;
    }
    final random = Random.secure();
    final id =
        widget.category?.id ??
        "category-${List.generate(16, (_) => random.nextInt(256).toRadixString(16).padLeft(2, '0')).join()}";
    Navigator.of(context).pop(
      LibraryClassificationCategory(
        id: id,
        label: name,
        descriptions: List.unmodifiable(descriptions),
      ),
    );
  }

  @override
  Widget build(BuildContext context) => AlertDialog(
    title: Text(widget.category == null ? "新建分类" : "编辑分类"),
    scrollable: true,
    content: SizedBox(
      width: 480,
      child: Column(
        mainAxisSize: MainAxisSize.min,
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          TextField(
            key: const Key("classification-category-name"),
            controller: _name,
            autofocus: true,
            decoration: const InputDecoration(labelText: "分类名称"),
          ),
          const SizedBox(height: 16),
          TextField(
            key: const Key("classification-category-descriptions"),
            controller: _descriptions,
            minLines: 4,
            maxLines: 8,
            decoration: const InputDecoration(
              labelText: "分类描述",
              helperText: "每行一条，说明图片的内容或用途",
            ),
          ),
          if (_error != null)
            Padding(
              padding: const EdgeInsets.only(top: 12),
              child: Text(
                _error!,
                style: TextStyle(color: Theme.of(context).colorScheme.error),
              ),
            ),
        ],
      ),
    ),
    actions: [
      TextButton(
        onPressed: () => Navigator.of(context).pop(),
        child: const Text("取消"),
      ),
      FilledButton(onPressed: _save, child: const Text("保存")),
    ],
  );
}
