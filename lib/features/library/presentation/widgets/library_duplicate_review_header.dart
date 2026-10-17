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
  Widget build(BuildContext context) {
    if (onBack == null) return _groupHeader(context);
    return _memberHeader(context);
  }

  Widget _groupHeader(BuildContext context) {
    final summary = Column(
      mainAxisSize: MainAxisSize.min,
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Text(
          title,
          key: const Key("duplicate-review-title"),
          style: Theme.of(context).textTheme.headlineSmall,
        ),
        const SizedBox(height: 8),
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
      ],
    );
    final category = SegmentedButton<bool>(
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
    );
    return LayoutBuilder(
      builder: (context, constraints) {
        final textScale = MediaQuery.textScalerOf(context).scale(24) / 24;
        final compactWidth = 600 * textScale;
        if (constraints.maxWidth < compactWidth) {
          return Column(
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              summary,
              const SizedBox(height: 12),
              Align(child: category),
              const SizedBox(height: 4),
              Align(alignment: Alignment.centerRight, child: actions),
            ],
          );
        }
        return Row(
          children: [
            Expanded(child: summary),
            const SizedBox(width: 24),
            category,
            const SizedBox(width: 24),
            Expanded(
              child: Align(alignment: Alignment.centerRight, child: actions),
            ),
          ],
        );
      },
    );
  }

  Widget _memberHeader(BuildContext context) => Column(
    crossAxisAlignment: CrossAxisAlignment.stretch,
    children: [
      Row(
        children: [
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
      Padding(
        padding: const EdgeInsets.only(left: backInset),
        child: OverflowBar(
          alignment: MainAxisAlignment.end,
          overflowAlignment: OverflowBarAlignment.end,
          spacing: 16,
          overflowSpacing: 8,
          children: [actions],
        ),
      ),
    ],
  );
}
