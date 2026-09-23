import "dart:math" as math;

import "package:flutter/material.dart";
import "package:material_symbols_icons/symbols.dart";

import "../../../../app/presentation/ame_overlay_semantics.dart";
import "../library_strings.dart";

enum _PreviewFeedbackKind { failed, updateRequired, retrying }

class LibraryPreviewFeedback extends StatelessWidget {
  const LibraryPreviewFeedback.failed({
    required this.locationId,
    required VoidCallback this.onRetry,
    required bool updateRequired,
    super.key,
  }) : _kind = updateRequired
           ? _PreviewFeedbackKind.updateRequired
           : _PreviewFeedbackKind.failed;

  const LibraryPreviewFeedback.retrying({required this.locationId, super.key})
    : onRetry = null,
      _kind = _PreviewFeedbackKind.retrying;

  static const _iconExtent = 24.0;
  static const _buttonPadding = EdgeInsets.symmetric(
    horizontal: 12,
    vertical: 8,
  );
  static const _guidancePadding = EdgeInsets.symmetric(horizontal: 8);

  final String locationId;
  final VoidCallback? onRetry;
  final _PreviewFeedbackKind _kind;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final bodyStyle =
        theme.textTheme.bodyMedium ?? DefaultTextStyle.of(context).style;
    final labelStyle = theme.textTheme.labelLarge ?? bodyStyle;
    return LayoutBuilder(
      builder: (context, constraints) {
        if (_kind == _PreviewFeedbackKind.retrying) {
          final labelSize = _measure(
            context,
            LibraryStrings.retryingPreview,
            bodyStyle,
          );
          final showText =
              constraints.maxWidth >= labelSize.width &&
              constraints.maxHeight >= _iconExtent + 8 + labelSize.height;
          return Semantics(
            container: true,
            liveRegion: true,
            label: LibraryStrings.retryingPreview,
            child: ExcludeSemantics(
              child: Center(
                child: Column(
                  mainAxisSize: MainAxisSize.min,
                  children: [
                    SizedBox.square(
                      key: ValueKey("preview-retry-progress-$locationId"),
                      dimension: _iconExtent,
                      child: const CircularProgressIndicator(strokeWidth: 2.5),
                    ),
                    if (showText) ...[
                      const SizedBox(height: 8),
                      Text(
                        LibraryStrings.retryingPreview,
                        style: bodyStyle,
                        softWrap: false,
                      ),
                    ],
                  ],
                ),
              ),
            ),
          );
        }

        final labelSize = _measure(
          context,
          LibraryStrings.retryPreview,
          labelStyle,
        );
        final buttonHeight = math.max(
          kMinInteractiveDimension,
          labelSize.height + _buttonPadding.vertical,
        );
        final buttonWidth = math.max(
          kMinInteractiveDimension,
          labelSize.width + _buttonPadding.horizontal,
        );
        final updateRequired = _kind == _PreviewFeedbackKind.updateRequired;
        final guidanceHeight = updateRequired
            ? _measure(
                    context,
                    LibraryStrings.previewUpdateRequired,
                    bodyStyle,
                    maxWidth: math.max(
                      0,
                      constraints.maxWidth - _guidancePadding.horizontal,
                    ),
                  ).height +
                  4
            : 0.0;
        final showText =
            constraints.maxWidth >= buttonWidth &&
            constraints.maxHeight >=
                _iconExtent + 4 + guidanceHeight + buttonHeight;
        if (!showText) {
          final message = updateRequired
              ? "${LibraryStrings.previewUpdateRequired}\n${LibraryStrings.retryPreview}"
              : LibraryStrings.retryPreview;
          return Center(
            child: AmeTooltip(
              message: message,
              child: Semantics(
                liveRegion: updateRequired,
                label: message,
                child: IconButton(
                  key: ValueKey("preview-retry-$locationId"),
                  onPressed: onRetry,
                  style: IconButton.styleFrom(
                    minimumSize: const Size.square(kMinInteractiveDimension),
                  ),
                  icon: const Icon(Symbols.refresh_rounded),
                ),
              ),
            ),
          );
        }

        return Center(
          child: Column(
            mainAxisSize: MainAxisSize.min,
            children: [
              const Icon(Symbols.broken_image_rounded, size: _iconExtent),
              const SizedBox(height: 4),
              if (updateRequired) ...[
                Padding(
                  padding: _guidancePadding,
                  child: Semantics(
                    container: true,
                    liveRegion: true,
                    label: LibraryStrings.previewUpdateRequired,
                    child: ExcludeSemantics(
                      child: Text(
                        LibraryStrings.previewUpdateRequired,
                        textAlign: TextAlign.center,
                        style: bodyStyle,
                      ),
                    ),
                  ),
                ),
                const SizedBox(height: 4),
              ],
              TextButton(
                key: ValueKey("preview-retry-$locationId"),
                onPressed: onRetry,
                style: TextButton.styleFrom(
                  minimumSize: const Size.square(kMinInteractiveDimension),
                  padding: _buttonPadding,
                  textStyle: labelStyle,
                ),
                child: const Text(LibraryStrings.retryPreview, softWrap: false),
              ),
            ],
          ),
        );
      },
    );
  }

  Size _measure(
    BuildContext context,
    String text,
    TextStyle style, {
    double maxWidth = double.infinity,
  }) {
    final painter = TextPainter(
      text: TextSpan(text: text, style: style),
      textDirection: Directionality.of(context),
      textScaler: MediaQuery.textScalerOf(context),
      locale: Localizations.maybeLocaleOf(context),
    )..layout(maxWidth: maxWidth);
    try {
      return painter.size;
    } finally {
      painter.dispose();
    }
  }
}
