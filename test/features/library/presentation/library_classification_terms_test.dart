import "dart:async";

import "package:cedarflake_ame/app/presentation/ame_localizations.dart";
import "package:cedarflake_ame/app/presentation/ame_theme.dart";
import "package:cedarflake_ame/features/library/application/library_classification_terms.dart";
import "package:cedarflake_ame/features/library/domain/library_classification_configuration.dart";
import "package:cedarflake_ame/features/library/domain/library_classification_term.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_classification_term_editor.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_classification_terms_sheet.dart";
import "package:material_ui/material_ui.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:flutter_test/flutter_test.dart";

import "../../../support/localized_test_app.dart";
import "../support/library_classification_terms_fixture.dart";

void main() {
  Future<void> open(
    WidgetTester tester,
    ClassificationTermsFixture fixture, {
    Locale locale = const Locale("en"),
    double textScale = 1,
  }) async {
    tester.view.physicalSize = const Size(1100, 700);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);
    await tester.pumpWidget(
      ProviderScope(
        overrides: [
          libraryClassificationTermsProvider.overrideWithValue(fixture.editor),
        ],
        child: LocalizedTestApp(
          locale: locale,
          theme: buildAmeTheme(locale: locale),
          builder: (context, child) => MediaQuery(
            data: MediaQuery.of(
              context,
            ).copyWith(textScaler: TextScaler.linear(textScale)),
            child: child!,
          ),
          home: Scaffold(
            body: Builder(
              builder: (context) => TextButton(
                onPressed: () => showLibraryClassificationTerms(context),
                child: const Text("Open"),
              ),
            ),
          ),
        ),
      ),
    );
    await tester.tap(find.text("Open"));
    await tester.pumpAndSettle();
  }

  Future<void> click(WidgetTester tester, String key) async {
    final target = find.byKey(Key(key));
    await tester.ensureVisible(target);
    await tester.tap(target);
    await tester.pumpAndSettle();
  }

  for (final locale in [
    const Locale("en"),
    const Locale("zh", "Hans"),
    const Locale("ja"),
  ]) {
    testWidgets(
      "unchanged localized name stays canonical and controls remain usable: $locale",
      (tester) async {
        final fixture = ClassificationTermsFixture();
        await open(tester, fixture, locale: locale, textScale: 1.5);
        await click(tester, "classification-term-photo");
        final field = tester.widget<TextFormField>(
          find.byKey(const Key("classification-term-name")),
        );
        expect(field.controller!.text, isNotEmpty);
        final description = find.byType(TextFormField).last;
        final semantics = tester.ensureSemantics();
        try {
          await tester.pump();
          expect(
            tester.getSemantics(description).label,
            contains(
              tester.element(description).l10n.smartAlbumTermDescription(1),
            ),
          );
        } finally {
          semantics.dispose();
        }
        await click(tester, "classification-term-save");
        expect(fixture.terms.single.category.label, starterCategory.label);
        expect(fixture.retired, 0);
        expect(find.byType(LibraryClassificationTermEditor), findsNothing);
        expect(tester.takeException(), isNull);
      },
    );
  }

  testWidgets(
    "create, reopen, edit descriptions, reset and delete persist at each action",
    (tester) async {
      final fixture = ClassificationTermsFixture();
      await open(tester, fixture);
      await click(tester, "classification-term-add");
      await tester.enterText(
        find.byKey(const Key("classification-term-name")),
        "My cats",
      );
      await tester.enterText(find.byType(TextFormField).last, "a sleeping cat");
      await click(tester, "classification-term-save");
      final created = fixture.terms.last;
      expect(created.category.id, startsWith("custom-"));
      expect(fixture.retired, 1);
      await click(tester, "classification-term-${created.category.id}");
      await tester.enterText(
        find.byType(TextFormField).last,
        "a cat on a sofa",
      );
      await click(tester, "classification-term-save");
      expect(fixture.terms.last.category.descriptions, ["a cat on a sofa"]);
      await click(tester, "classification-term-photo");
      await tester.enterText(
        find.byKey(const Key("classification-term-name")),
        "My photographs",
      );
      await click(tester, "classification-term-save");
      expect(fixture.retired, 2);
      await click(tester, "classification-term-photo");
      final reset = find.byKey(const Key("classification-term-reset"));
      await tester.tap(reset);
      await tester.pump();
      expect(tester.widget<TextButton>(reset).onPressed, isNull);
      await tester.pumpAndSettle();
      expect(fixture.terms.first.category.label, starterCategory.label);
      await click(tester, "classification-term-${created.category.id}");
      await click(tester, "classification-term-delete");
      await click(tester, "classification-term-delete-confirm");
      expect(fixture.terms.map((term) => term.category.id), ["photo"]);
      expect(fixture.changes, 5);
    },
  );

  testWidgets(
    "unconfigured built-in resets its draft and activates only on save",
    (tester) async {
      final fixture = ClassificationTermsFixture()
        ..terms = [
          const LibraryClassificationTerm(
            category: starterCategory,
            defaultCategory: starterCategory,
            isCustom: false,
            isConfigured: false,
          ),
        ];
      await open(tester, fixture);
      await click(tester, "classification-term-photo");
      await tester.enterText(
        find.byType(TextFormField).last,
        "a changed description",
      );
      await click(tester, "classification-term-reset");
      expect(find.byType(LibraryClassificationTermEditor), findsOneWidget);
      expect(
        tester
            .widget<TextFormField>(find.byType(TextFormField).last)
            .controller!
            .text,
        starterCategory.descriptions.single,
      );
      expect(fixture.saves, 0);
      expect(fixture.changes, 0);
      expect(fixture.retired, 0);
      await click(tester, "classification-term-save");
      expect(fixture.lastExpected, isNull);
      expect(fixture.terms.single.isConfigured, isTrue);
      expect(fixture.loads, 2);
      expect(fixture.retired, 1);
      expect(fixture.changes, 1);
      await click(tester, "classification-term-photo");
      await tester.enterText(
        find.byKey(const Key("classification-term-name")),
        "Renamed photographs",
      );
      await click(tester, "classification-term-save");
      expect(fixture.lastExpected?.id, "photo");
      expect(fixture.retired, 1);
    },
  );

  testWidgets("saving a lower row preserves the tray scroll position", (
    tester,
  ) async {
    final fixture = ClassificationTermsFixture()
      ..terms = [
        for (var index = 0; index < 24; index++)
          LibraryClassificationTerm(
            category: LibraryClassificationCategory(
              id: "custom-$index",
              label: "Category $index",
              descriptions: ["a photo of subject $index"],
            ),
            isCustom: true,
          ),
      ];
    await open(tester, fixture);
    final list = find.byType(ListView);
    final controller = tester.widget<ListView>(list).controller!;
    controller.jumpTo(650);
    await tester.pumpAndSettle();
    final before = controller.offset;
    final row = find.byKey(const Key("classification-term-custom-10"));
    await tester.tap(row);
    await tester.pumpAndSettle();
    await tester.enterText(
      find.byKey(const Key("classification-term-name")),
      "Edited subject",
    );
    await click(tester, "classification-term-save");
    expect(controller.offset, before);
    expect(find.text("Edited subject"), findsOneWidget);
  });

  testWidgets(
    "save holds dismissal and duplicate submission; failed draft survives and reopening refreshes expected term",
    (tester) async {
      final fixture = ClassificationTermsFixture()
        ..reject = true
        ..saveGate = Completer<void>();
      await open(tester, fixture);
      await click(tester, "classification-term-photo");
      await tester.enterText(
        find.byKey(const Key("classification-term-name")),
        "My pictures",
      );
      await tester.tap(find.byKey(const Key("classification-term-save")));
      await tester.pump();
      expect(
        tester
            .widget<FilledButton>(
              find.byKey(const Key("classification-term-save")),
            )
            .onPressed,
        isNull,
      );
      final context = tester.element(
        find.byType(LibraryClassificationTermEditor),
      );
      await Navigator.of(context).maybePop();
      await tester.pump();
      expect(find.byType(LibraryClassificationTermEditor), findsOneWidget);
      fixture.saveGate!.complete();
      await tester.pumpAndSettle();
      expect(
        tester
            .widget<TextFormField>(
              find.byKey(const Key("classification-term-name")),
            )
            .controller!
            .text,
        "My pictures",
      );
      const newer = LibraryClassificationCategory(
        id: "photo",
        label: "Newest",
        descriptions: ["a photograph"],
        defaultLabel: "Photography",
      );
      fixture.terms = [
        const LibraryClassificationTerm(
          category: newer,
          isCustom: false,
          defaultCategory: starterCategory,
        ),
      ];
      await Navigator.of(context).maybePop();
      await tester.pumpAndSettle();
      await click(tester, "classification-term-photo");
      expect(
        tester
            .widget<TextFormField>(
              find.byKey(const Key("classification-term-name")),
            )
            .controller!
            .text,
        "Newest",
      );
      expect(fixture.loads, 2);
      expect(fixture.saves, 1);
      expect(fixture.changes, 0);
      fixture.reject = false;
      await tester.enterText(
        find.byKey(const Key("classification-term-name")),
        "Updated pictures",
      );
      await click(tester, "classification-term-save");
      expect(fixture.lastExpected, same(newer));
      expect(fixture.terms.single.category.label, "Updated pictures");
      expect(fixture.changes, 1);
      expect(find.byType(LibraryClassificationTermEditor), findsNothing);
    },
  );
}
