import "dart:async";

import "package:cedarflake_ame/features/library/application/library_controller.dart";
import "package:cedarflake_ame/features/library/application/library_organization.dart";
import "package:cedarflake_ame/features/library/domain/library_organization.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_favorite_button.dart";
import "package:flutter/material.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:flutter_test/flutter_test.dart";

import "../support/library_organization_fixture.dart";

void main() {
  testWidgets(
    "shows committed state and disables duplicate input during save",
    (tester) async {
      final service = OrganizationServiceFixture();
      final committed = Completer<LibraryOrganizationReceipt>();
      service.save = (_) => committed.future;
      await tester.pumpWidget(_host(service, organizationTarget("one")));
      await tester.pumpAndSettle();
      expect(_button(tester).isSelected, isFalse);
      await tester.tap(find.byKey(const Key("viewer-favorite")));
      await tester.pump();
      expect(_button(tester).onPressed, isNull);
      expect(_button(tester).isSelected, isFalse);
      committed.complete(service.receipt(service.writes.single.id));
      await tester.pumpAndSettle();
      expect(_button(tester).isSelected, isTrue);
      expect(_button(tester).onPressed, isNotNull);
      expect(tester.takeException(), isNull);
    },
  );

  testWidgets(
    "a previous image's late state cannot replace the current image",
    (tester) async {
      final service = OrganizationServiceFixture();
      final first = Completer<LibraryOrganizationInspection>();
      service.inspectTarget = (target) async => target.locationId == "one"
          ? first.future
          : organizationInspection(false);
      await tester.pumpWidget(_host(service, organizationTarget("one")));
      await tester.pump();
      expect(_button(tester).onPressed, isNull);
      await tester.pumpWidget(_host(service, organizationTarget("two")));
      await tester.pumpAndSettle();
      expect(_button(tester).isSelected, isFalse);
      first.complete(organizationInspection(true));
      await tester.pumpAndSettle();
      expect(_button(tester).isSelected, isFalse);
      await tester.tap(find.byKey(const Key("viewer-favorite")));
      await tester.pumpAndSettle();
      expect(service.writes, hasLength(1));
      expect(tester.takeException(), isNull);
    },
  );

  testWidgets("unavailable evidence offers a read retry without writing", (
    tester,
  ) async {
    final service = OrganizationServiceFixture();
    service.inspectTarget = (_) async => throw const LibraryOrganizationFailure(
      "organization_evidence_pending",
      "Evidence is pending",
    );
    await tester.pumpWidget(_host(service, organizationTarget("one")));
    await tester.pumpAndSettle();
    expect(find.bySemanticsLabel("正在确认图片内容，点击重试"), findsOneWidget);
    service.inspectTarget = null;
    await tester.tap(find.byKey(const Key("viewer-favorite")));
    await tester.pumpAndSettle();
    expect(service.writes, isEmpty);
    expect(_button(tester).isSelected, isFalse);
    expect(_button(tester).onPressed, isNotNull);
  });
}

IconButton _button(WidgetTester tester) =>
    tester.widget<IconButton>(find.byKey(const Key("viewer-favorite")));

Widget _host(
  OrganizationServiceFixture service,
  LibraryOrganizationTarget target,
) => ProviderScope(
  overrides: [
    libraryOrganizationProvider.overrideWithValue(service),
    libraryControllerProvider.overrideWith(OrganizationLibraryFixture.new),
  ],
  child: MaterialApp(
    home: Scaffold(body: LibraryFavoriteButton(target: target)),
  ),
);
