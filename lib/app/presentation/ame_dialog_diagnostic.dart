import "package:material_ui/material_ui.dart";

import "ame_dialog.dart";

class AmeDialogDiagnostic extends StatelessWidget {
  const AmeDialogDiagnostic({required this.detail, super.key});

  final String detail;

  @override
  Widget build(BuildContext context) {
    final suffix = RegExp(
      r"\s*\((os error \d+|0x[0-9a-fA-F]+)\)\s*$",
    ).firstMatch(detail);
    final code = suffix?.group(1);
    return AmeDialogBody(
      children: [
        AmeDialogError(
          suffix == null
              ? detail
              : detail.substring(0, suffix.start).trimRight(),
        ),
        if (code != null) ...[
          AmeDialogField(label: "错误代码", child: SelectableText(code)),
        ],
      ],
    );
  }
}
