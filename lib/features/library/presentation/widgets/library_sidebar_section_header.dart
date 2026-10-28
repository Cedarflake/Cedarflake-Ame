import "package:flutter/material.dart";
import "package:material_symbols_icons/symbols.dart";

import "../../../../app/presentation/ame_overlay_semantics.dart";
import "../../../../app/presentation/ame_typography.dart";

class LibrarySidebarSectionHeader extends StatelessWidget {
  const LibrarySidebarSectionHeader({
    required this.title,
    required this.createLabel,
    required this.createKey,
    required this.onCreate,
    super.key,
  });

  final String title;
  final String createLabel;
  final Key createKey;
  final VoidCallback? onCreate;

  @override
  Widget build(BuildContext context) => Padding(
    padding: const EdgeInsets.fromLTRB(16, 8, 4, 0),
    child: Row(
      children: [
        Expanded(
          child: Text(
            title,
            style: Theme.of(
              context,
            ).textTheme.titleSmall?.copyWith(fontWeight: ameFontWeightSemibold),
          ),
        ),
        AmeTooltip(
          message: createLabel,
          child: IconButton(
            key: createKey,
            onPressed: onCreate,
            icon: const Icon(Symbols.add_rounded),
          ),
        ),
      ],
    ),
  );
}
