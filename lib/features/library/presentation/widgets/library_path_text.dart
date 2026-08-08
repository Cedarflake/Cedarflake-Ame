import "package:flutter/material.dart";

class LibraryPathText extends StatelessWidget {
  const LibraryPathText({
    required this.text,
    required this.path,
    this.alwaysShowPathTooltip = false,
    this.textKey,
    super.key,
  });

  final String text;
  final String path;
  final bool alwaysShowPathTooltip;
  final Key? textKey;

  @override
  Widget build(BuildContext context) {
    return LayoutBuilder(
      builder: (context, constraints) {
        final style = DefaultTextStyle.of(context).style;
        final painter = TextPainter(
          text: TextSpan(text: text, style: style),
          maxLines: 1,
          textDirection: Directionality.of(context),
          textScaler: MediaQuery.textScalerOf(context),
        )..layout(maxWidth: constraints.maxWidth);
        final label = Text(
          text,
          key: textKey,
          maxLines: 1,
          overflow: TextOverflow.ellipsis,
        );
        final Widget content;
        if (!alwaysShowPathTooltip && !painter.didExceedMaxLines) {
          content = label;
        } else {
          content = Tooltip(message: path, child: label);
        }
        return Align(alignment: Alignment.centerLeft, child: content);
      },
    );
  }
}

String displayLibraryFolderPath(String rootPath, String relativePath) {
  final root = rootPath.replaceFirst(RegExp(r"\\+$"), "");
  final relative = relativePath.replaceAll("/", "\\");
  return relative.isEmpty ? root : "$root\\$relative";
}

String displayLibraryFileName(String path) {
  final separatorIndex = path.lastIndexOf("\\");
  if (separatorIndex < 0 || separatorIndex == path.length - 1) {
    return path;
  }
  return path.substring(separatorIndex + 1);
}
