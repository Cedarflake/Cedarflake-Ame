import "package:material_ui/material_ui.dart";
import "package:material_symbols_icons/symbols.dart";

import "ame_dialog.dart";
import "ame_overlay_semantics.dart";

class AmeDialogLocation extends StatefulWidget {
  const AmeDialogLocation({
    required this.label,
    required this.path,
    required this.onOpen,
    super.key,
  });

  final String label;
  final String path;
  final Future<void> Function() onOpen;

  @override
  State<AmeDialogLocation> createState() => _AmeDialogLocationState();
}

class _AmeDialogLocationState extends State<AmeDialogLocation> {
  bool _opening = false;
  bool _failed = false;

  Future<void> _open() async {
    if (_opening) return;
    setState(() {
      _opening = true;
      _failed = false;
    });
    try {
      await widget.onOpen();
    } on Object {
      if (mounted) setState(() => _failed = true);
    } finally {
      if (mounted) setState(() => _opening = false);
    }
  }

  @override
  Widget build(BuildContext context) {
    return AmeDialogField(
      label: widget.label,
      child: Column(
        mainAxisSize: MainAxisSize.min,
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          AmeDialogPath(
            path: widget.path,
            trailing: TextButton(
              onPressed: _opening ? null : _open,
              child: Text(_failed ? "重试" : "打开位置"),
            ),
          ),
          if (_failed)
            Text(
              "无法打开这个位置，请重试。",
              style: TextStyle(color: Theme.of(context).colorScheme.error),
            ),
        ],
      ),
    );
  }
}

class AmeDialogPath extends StatelessWidget {
  const AmeDialogPath({required this.path, required this.trailing, super.key});

  final String path;
  final Widget trailing;

  @override
  Widget build(BuildContext context) {
    final name = path
        .replaceFirst(RegExp(r"[\\/]+$"), "")
        .split(RegExp(r"[\\/]"))
        .last;
    return Row(
      children: [
        const Icon(Symbols.folder_open_rounded),
        const SizedBox(width: 12),
        Expanded(
          child: AmeTooltip(
            message: path,
            child: Text(
              name.isEmpty ? path : name,
              maxLines: 2,
              overflow: TextOverflow.ellipsis,
            ),
          ),
        ),
        const SizedBox(width: 12),
        trailing,
      ],
    );
  }
}
