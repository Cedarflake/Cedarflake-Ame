import "package:cedarflake_ame/app/presentation/ame_localizations.dart";
import "package:cedarflake_ame/app/presentation/ame_choice.dart";
import "package:cedarflake_ame/app/presentation/ame_theme.dart";
import "package:cedarflake_ame/features/library/domain/library_metadata_filter.dart";
import "package:cedarflake_ame/features/library/presentation/coordination/library_metadata_filter_draft.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_metadata_filter_sheet.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_search_bar.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_viewer_notice.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_viewer_toolbar.dart";
import "package:flutter/rendering.dart";
import "package:flutter_test/flutter_test.dart";
import "package:material_ui/material_ui.dart";

import "../../../support/duplicate_review_fixtures.dart";
import "../../../support/localized_test_app.dart";

void main() {
  for (final locale in ameSupportedLocales) {
    testWidgets("filter labels and selected dates fit enlarged $locale", (
      tester,
    ) async {
      LibraryMetadataFilter? applied;
      await _mount(
        tester,
        locale,
        Builder(
          builder: (context) => Center(
            child: FilledButton(
              onPressed: () async =>
                  applied = await showLibraryMetadataFilterSheet(
                    context,
                    const LibraryMetadataFilter(
                      date: LibraryDateRange(
                        start: "2026-11-22",
                        end: "2026-12-31",
                      ),
                    ),
                  ),
              child: Text(context.l10n.filterPhotos),
            ),
          ),
        ),
      );
      await tester.tap(find.byType(FilledButton));
      await tester.pumpAndSettle();
      final strings = tester
          .element(find.byType(LibraryMetadataFilterSheet))
          .l10n;
      _expectComplete(tester, find.text(strings.captureDate));
      await tester.ensureVisible(find.text(strings.captureDate));
      await tester.tap(find.text(strings.captureDate));
      await tester.pumpAndSettle();
      _expectComplete(tester, find.text(strings.modifiedDate));
      await tester.tap(
        find.byKey(
          const ValueKey("ame-choice-option-LibraryDateField.modified"),
        ),
      );
      await tester.pumpAndSettle();
      _expectComplete(tester, find.text(strings.modifiedDate));
      for (final field in ["dateStart", "dateEnd"]) {
        final input = find.byKey(ValueKey("library-filter-$field"));
        await tester.ensureVisible(input);
        await tester.pumpAndSettle();
        final editable = tester
            .state<EditableTextState>(
              find.descendant(of: input, matching: find.byType(EditableText)),
            )
            .renderEditable;
        final end = editable.getLocalRectForCaret(
          TextPosition(offset: editable.text!.toPlainText().length),
        );
        expect(
          end.right,
          lessThanOrEqualTo(editable.size.width),
          reason: "$field must show the whole selected date",
        );
      }
      for (final field in ["minimumBytes", "maximumBytes"]) {
        final input = find.byKey(ValueKey("library-filter-$field"));
        await tester.ensureVisible(input);
        await tester.pumpAndSettle();
        _expectComplete(
          tester,
          find.text(
            field == "minimumBytes" ? strings.minimum : strings.maximum,
          ),
        );
      }
      final unit = find.byType(AmeChoice<LibraryFilterSizeUnit>);
      _expectComplete(
        tester,
        find.descendant(of: unit, matching: find.text("MB")),
      );
      await tester.ensureVisible(unit);
      await tester.pumpAndSettle();
      await tester.tap(unit);
      await tester.pumpAndSettle();
      _expectComplete(tester, find.text("GB"));
      await tester.tap(
        find.byKey(
          const ValueKey("ame-choice-option-LibraryFilterSizeUnit.gigabytes"),
        ),
      );
      await tester.pumpAndSettle();
      _expectComplete(
        tester,
        find.descendant(of: unit, matching: find.text("GB")),
      );
      final maximum = find.byKey(const ValueKey("library-filter-maximumBytes"));
      await tester.ensureVisible(maximum);
      await tester.pumpAndSettle();
      await tester.enterText(maximum, "-1");
      await tester.tap(find.text(strings.apply));
      await tester.pumpAndSettle();
      _expectComplete(tester, find.text(strings.invalidNumber));
      await tester.enterText(maximum, "");
      await tester.ensureVisible(find.text(strings.apply));
      await tester.tap(find.text(strings.apply));
      await tester.pumpAndSettle();
      expect(find.byType(LibraryMetadataFilterSheet), findsNothing);
      expect(applied?.date.field, LibraryDateField.modified);
      expect(applied?.date.start, "2026-11-22");
      expect(tester.takeException(), isNull);
    });

    testWidgets("viewer readouts stay on one line in enlarged $locale", (
      tester,
    ) async {
      await _mount(
        tester,
        locale,
        Builder(
          builder: (context) => Center(
            child: LibraryViewerToolbar(
              asset: reviewAsset("one"),
              positionLabel: context.l10n.photoPosition(12345, 123456),
              isActualSize: false,
              onToggleZoomMode: () {},
              sliderValue: 1,
              zoomPercent: 1600,
              onSliderChanged: (_) {},
              sliderSemanticFormatter: (_) => "1600%",
              onZoomOut: () {},
              onZoomIn: () {},
              onRotate: () {},
              isFullscreen: false,
              onToggleFullscreen: () {},
            ),
          ),
        ),
      );
      final zoom = find.byKey(const Key("viewer-zoom-percentage"));
      _expectComplete(tester, zoom);
      final paragraph = tester.renderObject<RenderParagraph>(
        find.descendant(of: zoom, matching: find.byType(RichText)),
      );
      final boxes = paragraph.getBoxesForSelection(
        const TextSelection(baseOffset: 0, extentOffset: 5),
      );
      expect(boxes.map((box) => box.top).toSet(), hasLength(1));
      final position = tester.getRect(find.byKey(const Key("viewer-position")));
      expect(
        position.right,
        lessThan(
          tester.getRect(find.byKey(const Key("viewer-zoom-mode"))).left,
        ),
      );
      expect(tester.takeException(), isNull);
    });

    testWidgets("search text and viewer acknowledgement fit enlarged $locale", (
      tester,
    ) async {
      final search = TextEditingController(text: "Landscape 日常 照片");
      addTearDown(search.dispose);
      var acknowledged = false;
      await _mount(
        tester,
        locale,
        Builder(
          builder: (context) => Stack(
            children: [
              Align(
                alignment: Alignment.bottomCenter,
                child: SizedBox(
                  width: 500,
                  child: LibrarySearchBar(controller: search, enabled: true),
                ),
              ),
              LibraryViewerNotice(
                icon: Icons.info_outline,
                message: context.l10n.smartResultsAvailable,
                actionLabel: context.l10n.acknowledge,
                onAction: () => acknowledged = true,
              ),
            ],
          ),
        ),
      );
      final editable = tester
          .state<EditableTextState>(find.byType(EditableText))
          .renderEditable;
      final caret = editable.getLocalRectForCaret(
        const TextPosition(offset: 0),
      );
      expect(caret.height, lessThanOrEqualTo(editable.size.height));
      final strings = tester.element(find.byType(LibraryViewerNotice)).l10n;
      _expectComplete(tester, find.text(strings.smartResultsAvailable));
      _expectComplete(tester, find.text(strings.acknowledge));
      await tester.tap(find.text(strings.acknowledge));
      expect(acknowledged, isTrue);
      expect(tester.takeException(), isNull);
    });
  }
}

Future<void> _mount(WidgetTester tester, Locale locale, Widget child) async {
  tester.view.physicalSize = const Size(1100, 700);
  tester.view.devicePixelRatio = 1;
  addTearDown(tester.view.resetPhysicalSize);
  addTearDown(tester.view.resetDevicePixelRatio);
  await tester.pumpWidget(
    LocalizedTestApp(
      locale: locale,
      theme: buildAmeTheme(),
      builder: (context, child) => MediaQuery(
        data: MediaQuery.of(
          context,
        ).copyWith(textScaler: const TextScaler.linear(1.5)),
        child: child!,
      ),
      home: Scaffold(body: child),
    ),
  );
  await tester.pumpAndSettle();
  expect(
    MediaQuery.sizeOf(tester.element(find.byType(Scaffold))),
    const Size(1100, 700),
  );
}

void _expectComplete(WidgetTester tester, Finder text) {
  final paragraph = tester.renderObject<RenderParagraph>(
    find.descendant(of: text, matching: find.byType(RichText)),
  );
  expect(
    paragraph.didExceedMaxLines,
    isFalse,
    reason: paragraph.text.toPlainText(),
  );
}
