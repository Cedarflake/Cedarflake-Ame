import "package:material_ui/material_ui.dart";

import "../../../../app/presentation/ame_scroll_motion.dart";

class LibraryFilterFormatChoices extends StatelessWidget {
  const LibraryFilterFormatChoices({
    required this.selected,
    required this.onToggled,
    super.key,
  });

  static const formats = [
    "jpg",
    "jpeg",
    "png",
    "webp",
    "gif",
    "bmp",
    "tif",
    "tiff",
    "ico",
  ];

  final Set<String> selected;
  final ValueChanged<String> onToggled;

  @override
  Widget build(BuildContext context) => AmeScrollMotion(
    scrollDirection: Axis.horizontal,
    child: SingleChildScrollView(
      scrollDirection: Axis.horizontal,
      padding: const EdgeInsets.symmetric(vertical: 4),
      child: Row(
        children: [
          for (final format in {...formats, ...selected}) ...[
            _format(format),
            const SizedBox(width: 6),
          ],
        ],
      ),
    ),
  );

  Widget _format(String format) => FilterChip(
    label: Text(format.toUpperCase()),
    selected: selected.contains(format),
    onSelected: (_) => onToggled(format),
  );
}
