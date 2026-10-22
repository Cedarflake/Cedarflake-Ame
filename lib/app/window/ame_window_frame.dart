import "package:flutter/material.dart";

import "../presentation/ame_brand_mark.dart";
import "../presentation/ame_typography.dart";
import "ame_window_chrome.dart";

class AmeWindowFrame extends StatelessWidget {
  const AmeWindowFrame({required this.child, super.key});

  final Widget child;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    return Column(
      children: [
        Material(
          color: colorScheme.surfaceContainerLow,
          child: DecoratedBox(
            decoration: BoxDecoration(
              border: Border(
                bottom: BorderSide(color: colorScheme.outlineVariant),
              ),
            ),
            child: SizedBox(
              height: 40,
              child: Row(
                children: [
                  Expanded(
                    child: AmeWindowDragRegion(
                      child: Padding(
                        padding: const EdgeInsets.symmetric(horizontal: 14),
                        child: Row(
                          children: [
                            const AmeBrandMark(size: 18),
                            const SizedBox(width: 9),
                            Flexible(
                              child: Text(
                                "Cedarflake Ame",
                                maxLines: 1,
                                overflow: TextOverflow.ellipsis,
                                style: TextStyle(
                                  color: colorScheme.primary,
                                  fontWeight: ameFontWeightSemibold,
                                ),
                              ),
                            ),
                          ],
                        ),
                      ),
                    ),
                  ),
                  const AmeWindowCaptionControls(),
                ],
              ),
            ),
          ),
        ),
        Expanded(child: child),
      ],
    );
  }
}
