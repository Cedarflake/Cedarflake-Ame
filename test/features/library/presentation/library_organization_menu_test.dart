import "dart:async";

import "package:cedarflake_ame/app/notifications/ame_notification_controller.dart";
import "package:cedarflake_ame/features/library/application/library_controller.dart";
import "package:cedarflake_ame/features/library/application/library_organization.dart";
import "package:cedarflake_ame/features/library/application/library_organization_history.dart";
import "package:cedarflake_ame/features/library/domain/library_organization.dart";
import "package:cedarflake_ame/features/library/domain/library_organization_history.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_organization_menu.dart";
import "package:flutter/material.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:flutter_test/flutter_test.dart";

import "../support/library_organization_fixture.dart";

void main() {
  late OrganizationServiceFixture service;
  late ProviderContainer container;

  setUp(() {
    service = OrganizationServiceFixture();
    container = ProviderContainer(
      overrides: [
        libraryOrganizationProvider.overrideWithValue(service),
        libraryControllerProvider.overrideWith(OrganizationLibraryFixture.new),
      ],
    );
  });
  tearDown(() => container.dispose());

  Future<void> host(WidgetTester tester) => tester.pumpWidget(
    UncontrolledProviderScope(
      container: container,
      child: const MaterialApp(
        home: Scaffold(body: LibraryOrganizationMenu(enabled: true)),
      ),
    ),
  );

  Future<void> open(WidgetTester tester) async {
    await tester.tap(find.byKey(const Key("library-review-actions-menu")));
    await tester.pumpAndSettle();
  }

  bool enabled(WidgetTester tester, String text) => tester
      .widget<PopupMenuItem<Object?>>(
        find.ancestor(
          of: find.text(text),
          matching: find.byWidgetPredicate((widget) => widget is PopupMenuItem),
        ),
      )
      .enabled;

  testWidgets("empty and loading history keep both commands disabled", (
    tester,
  ) async {
    final loading = Completer<LibraryOrganizationHistory>();
    service.readHistory = () => loading.future;
    await host(tester);
    await open(tester);
    expect(enabled(tester, "撤销操作"), isFalse);
    expect(enabled(tester, "重做操作"), isFalse);
    loading.complete(service.storedHistory);
    await tester.tapAt(const Offset(500, 500));
    await tester.pumpAndSettle();
    await open(tester);
    expect(enabled(tester, "撤销操作"), isFalse);
    expect(enabled(tester, "重做操作"), isFalse);
    expect(service.historySteps, isEmpty);
  });

  testWidgets(
    "menu reverses the last operation and exposes redo after commit",
    (tester) async {
      service.storedHistory = LibraryOrganizationHistory(
        revision: BigInt.two,
        undoOperationId: "previous-image-operation",
      );
      final saving = Completer<LibraryOrganizationHistoryReceipt>();
      service.saveHistory = (_) => saving.future;
      await host(tester);
      await tester.pumpAndSettle();
      await open(tester);
      expect(enabled(tester, "撤销操作"), isTrue);
      expect(enabled(tester, "重做操作"), isFalse);
      await tester.tap(find.text("撤销操作"));
      await tester.pumpAndSettle();
      expect(
        service.historySteps.single.originalId,
        "previous-image-operation",
      );
      expect(
        service.historySteps.single.direction,
        LibraryOrganizationHistoryDirection.undo,
      );
      expect(
        tester
            .widget<IconButton>(
              find.byKey(const Key("library-review-actions-menu")),
            )
            .onPressed,
        isNull,
      );
      final id = service.historySteps.single.id;
      service.storedHistory = LibraryOrganizationHistory(
        revision: BigInt.from(3),
        redoOperationId: id,
      );
      saving.complete(
        LibraryOrganizationHistoryReceipt(operation: service.receipt(id)),
      );
      await tester.pumpAndSettle();
      service.saveHistory = null;
      await open(tester);
      expect(enabled(tester, "撤销操作"), isFalse);
      expect(enabled(tester, "重做操作"), isTrue);
      await tester.tap(find.text("重做操作"));
      await tester.pumpAndSettle();
      expect(service.historySteps.last.originalId, id);
      expect(
        service.historySteps.last.direction,
        LibraryOrganizationHistoryDirection.redo,
      );
      expect(
        container.read(ameNotificationControllerProvider).history,
        isEmpty,
      );
    },
  );

  testWidgets("failed history read offers an actual retry", (tester) async {
    service.readHistory = () async => throw StateError("Read unavailable");
    await host(tester);
    await tester.pumpAndSettle();
    expect(find.byTooltip("操作记录未能加载，点击重试"), findsOneWidget);
    service.readHistory = null;
    await tester.tap(find.byKey(const Key("library-review-actions-menu")));
    await tester.pumpAndSettle();
    await open(tester);
    expect(find.text("撤销操作"), findsOneWidget);
    expect(service.historySteps, isEmpty);
  });

  testWidgets(
    "an open menu retains its observed operation when background history changes",
    (tester) async {
      service.storedHistory = LibraryOrganizationHistory(
        revision: BigInt.two,
        undoOperationId: "observed",
      );
      await host(tester);
      await tester.pumpAndSettle();
      await open(tester);
      service.storedHistory = LibraryOrganizationHistory(
        revision: BigInt.from(3),
        undoOperationId: "newer",
      );
      container.invalidate(libraryOrganizationHistoryProvider);
      await tester.pumpAndSettle();
      await tester.tap(find.text("撤销操作"));
      await tester.pumpAndSettle();
      expect(service.historySteps.single.originalId, "observed");
      expect(service.historySteps.single.revision, BigInt.two);
    },
  );
}
