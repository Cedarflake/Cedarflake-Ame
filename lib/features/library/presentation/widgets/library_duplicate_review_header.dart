import "package:flutter/material.dart";
import "package:material_symbols_icons/symbols.dart";

import "../../../../app/presentation/ame_overlay_semantics.dart";
import "../library_duplicate_strings.dart";
import "library_path_text.dart";

class LibraryDuplicateReviewHeader extends StatelessWidget {
  const LibraryDuplicateReviewHeader({
    required this.title,
    required this.totalGroups,
    required this.ignored,
    required this.onCategoryChanged,
    required this.actions,
    this.onBack,
    super.key,
  });

  static const backInset = 48.0;
  final String title;
  final int? totalGroups;
  final bool ignored;
  final ValueChanged<bool> onCategoryChanged;
  final VoidCallback? onBack;
  final Widget actions;

  @override
  Widget build(BuildContext context) => Column(
    crossAxisAlignment: CrossAxisAlignment.stretch,
    children: [
      Row(
        children: [
          if (onBack != null) ...[
            SizedBox(
              width: 40,
              child: AmeTooltip(
                message: LibraryDuplicateStrings.back,
                child: IconButton(
                  key: const Key("duplicate-review-back"),
                  onPressed: onBack,
                  icon: const Icon(Symbols.arrow_back_rounded),
                ),
              ),
            ),
            const SizedBox(width: 8),
          ],
          Expanded(
            child: DefaultTextStyle(
              style: Theme.of(context).textTheme.headlineSmall!,
              child: LibraryPathText(
                text: title,
                path: title,
                textKey: const Key("duplicate-review-title"),
              ),
            ),
          ),
        ],
      ),
      const SizedBox(height: 8),
      if (onBack == null) ...[
        Visibility(
          visible: totalGroups != null,
          maintainSize: true,
          maintainAnimation: true,
          maintainState: true,
          child: Text(
            LibraryDuplicateStrings.groups(totalGroups ?? 0),
            key: const Key("duplicate-review-count"),
          ),
        ),
        const SizedBox(height: 12),
      ],
      Padding(
        padding: EdgeInsets.only(left: onBack == null ? 0 : backInset),
        child: OverflowBar(
          alignment: onBack == null
              ? MainAxisAlignment.spaceBetween
              : MainAxisAlignment.end,
          overflowAlignment: OverflowBarAlignment.end,
          spacing: 16,
          overflowSpacing: 8,
          children: [
            if (onBack == null)
              SegmentedButton<bool>(
                key: const Key("duplicate-review-category"),
                segments: const [
                  ButtonSegment(
                    value: false,
                    label: Text(LibraryDuplicateStrings.pendingGroups),
                  ),
                  ButtonSegment(
                    value: true,
                    label: Text(LibraryDuplicateStrings.ignoredGroups),
                  ),
                ],
                selected: {ignored},
                onSelectionChanged: (value) => onCategoryChanged(value.single),
              ),
            actions,
          ],
        ),
      ),
    ],
  );
}
