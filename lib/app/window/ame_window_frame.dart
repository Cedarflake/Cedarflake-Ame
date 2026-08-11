import "package:flutter/material.dart";
import "package:material_symbols_icons/symbols.dart";

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
                  const Expanded(
                    child: AmeWindowDragRegion(
                      child: Padding(
                        padding: EdgeInsets.symmetric(horizontal: 14),
                        child: Row(
                          children: [
                            Icon(Symbols.photo_library_rounded, size: 18),
                            SizedBox(width: 9),
                            Text("Cedarflake Ame"),
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
