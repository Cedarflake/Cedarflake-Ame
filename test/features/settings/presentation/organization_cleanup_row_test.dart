import "package:cedarflake_ame/app/presentation/ame_theme.dart";
import "package:cedarflake_ame/features/library/application/library_cleanup_controller.dart";
import "package:cedarflake_ame/features/library/application/library_cleanup_state.dart";
import "package:cedarflake_ame/features/library/application/library_controller.dart";
import "package:cedarflake_ame/features/library/application/library_organization_cleanup.dart";
import "package:cedarflake_ame/features/library/domain/library_organization_cleanup.dart";
import "package:cedarflake_ame/features/settings/presentation/widgets/organization_cleanup_row.dart";
import "package:flutter/material.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:flutter_test/flutter_test.dart";

import "../../library/support/library_cleanup_fixture.dart";
import "../../library/support/library_organization_fixture.dart";

void main() {
  late CleanupServiceFixture service;
  late ProviderContainer container;
  setUp(() {
    service = CleanupServiceFixture();
    container = ProviderContainer(
      overrides: [
        libraryOrganizationCleanupProvider.overrideWithValue(service),
        libraryControllerProvider.overrideWith(OrganizationLibraryFixture.new),
      ],
    );
  });
  tearDown(() => container.dispose());
  Future<void> host(WidgetTester tester) => tester.pumpWidget(
    UncontrolledProviderScope(
      container: container,
      child: MaterialApp(
        theme: buildAmeTheme(),
        builder: (context, child) => MediaQuery(
          data: MediaQuery.of(
            context,
          ).copyWith(textScaler: const TextScaler.linear(1.4)),
          child: child ?? const SizedBox.shrink(),
        ),
        home: const Scaffold(
          body: SingleChildScrollView(child: OrganizationCleanupRow()),
        ),
      ),
    ),
  );

  testWidgets(
    "inspection shows affected decisions and cancellation performs no cleanup",
    (tester) async {
      await host(tester);
      expect(find.text("查看并清理"), findsOneWidget);
      expect(find.text("检查"), findsNothing);
      expect(find.byKey(const Key("settings-cleanup-age")), findsNothing);
      await tester.tap(find.byKey(const Key("settings-cleanup-inspect")));
      await tester.pump();
      expect(
        find.descendant(
          of: find.byType(AlertDialog),
          matching: find.byKey(const Key("settings-cleanup-age")),
        ),
        findsOneWidget,
      );
      service.tasks.single.events.add(
        LibraryCleanupInspected(cleanupInspection()),
      );
      await tester.pumpAndSettle();
      expect(
        find.byKey(const Key("organization-cleanup-confirmation")),
        findsOneWidget,
      );
      expect(find.text("相册归属 4 组"), findsOneWidget);
      expect(find.text("收藏 4 组"), findsOneWidget);
      expect(find.textContaining("原图片会保留"), findsOneWidget);
      expect(tester.takeException(), isNull);
      await tester.tap(
        find.descendant(
          of: find.byType(AlertDialog),
          matching: find.text("取消"),
        ),
      );
      await tester.pumpAndSettle();
      expect(service.tasks, hasLength(1));
      expect(find.text("查看并清理"), findsOneWidget);
      expect(find.byKey(const Key("settings-cleanup-age")), findsNothing);
      expect(
        container.read(libraryCleanupControllerProvider),
        isA<LibraryCleanupReady>(),
      );
    },
  );

  testWidgets(
    "cleanup starts only after confirmation and empty inspections have no delete action",
    (tester) async {
      await host(tester);
      await tester.tap(find.byKey(const Key("settings-cleanup-inspect")));
      await tester.pump();
      service.tasks.last.events.add(
        LibraryCleanupInspected(cleanupInspection()),
      );
      await tester.pumpAndSettle();
      await tester.tap(find.byKey(const Key("organization-cleanup-confirm")));
      await tester.pump();
      final applying = service.tasks.last;
      expect(applying.operationId, isNotNull);
      applying.events.add(LibraryCleanupCompleted(applying.operationId!, 4));
      await tester.pumpAndSettle();
      expect(find.text("已清理 4 项记录"), findsOneWidget);
      await tester.tap(find.byKey(const Key("settings-cleanup-inspect")));
      await tester.pump();
      service.tasks.last.events.add(
        LibraryCleanupInspected(cleanupInspection(groups: 0)),
      );
      await tester.pumpAndSettle();
      expect(find.text("没有符合条件的未关联记录"), findsOneWidget);
      expect(find.byType(AlertDialog), findsOneWidget);
      expect(
        tester
            .widget<FilledButton>(
              find.byKey(const Key("organization-cleanup-confirm")),
            )
            .onPressed,
        isNull,
      );
    },
  );

  testWidgets("changing the dialog range requires its new inspection", (
    tester,
  ) async {
    await host(tester);
    await tester.tap(find.byKey(const Key("settings-cleanup-inspect")));
    await tester.pump();
    expect(service.tasks.single.cutoffUnixMs, isNotNull);
    service.tasks.single.events.add(
      LibraryCleanupInspected(cleanupInspection(groups: 0)),
    );
    await tester.pumpAndSettle();
    await tester.tap(find.byKey(const Key("settings-cleanup-age")));
    await tester.pumpAndSettle();
    await tester.tap(find.byKey(const ValueKey("settings-choice-option-0")));
    await tester.pump();
    expect(service.tasks, hasLength(2));
    expect(service.tasks.last.cutoffUnixMs, isNull);
    expect(
      tester
          .widget<FilledButton>(
            find.byKey(const Key("organization-cleanup-confirm")),
          )
          .onPressed,
      isNull,
    );
    expect(find.text("没有符合条件的未关联记录"), findsNothing);
    final current = cleanupInspection(groups: 6);
    service.tasks.last.events.add(LibraryCleanupInspected(current));
    await tester.pumpAndSettle();
    expect(find.text("相册归属 6 组"), findsOneWidget);
    await tester.tap(find.byKey(const Key("organization-cleanup-confirm")));
    await tester.pump();
    expect(service.tasks, hasLength(3));
    expect(service.tasks.last.inspection, same(current));
    service.tasks.last.events.add(
      LibraryCleanupCompleted(service.tasks.last.operationId!, 6),
    );
    await tester.pumpAndSettle();
    expect(find.text("已清理 6 项记录"), findsOneWidget);
  });

  testWidgets("closing a pending inspection prevents late confirmation", (
    tester,
  ) async {
    await host(tester);
    await tester.tap(find.byKey(const Key("settings-cleanup-inspect")));
    await tester.pump();
    await tester.tap(
      find.descendant(of: find.byType(AlertDialog), matching: find.text("取消")),
    );
    await tester.pump();
    expect(service.tasks.single.cancellations, 1);
    service.tasks.single.events.add(
      LibraryCleanupInspected(cleanupInspection()),
    );
    await tester.pumpAndSettle();
    expect(find.byType(AlertDialog), findsNothing);
    expect(service.tasks.single.operationId, isNull);
    expect(find.text("查看并清理"), findsOneWidget);
    expect(tester.takeException(), isNull);
  });

  testWidgets("failed inspection retries inside the same dialog", (
    tester,
  ) async {
    await host(tester);
    await tester.tap(find.byKey(const Key("settings-cleanup-inspect")));
    await tester.pump();
    final cutoff = service.tasks.single.cutoffUnixMs;
    service.tasks.single.events.addError(
      const LibraryCleanupFailure("read_failed", "Inspection failed"),
    );
    await tester.pumpAndSettle();
    expect(find.text("未能完成检查，请重试。"), findsWidgets);
    await tester.tap(find.byKey(const Key("organization-cleanup-retry")));
    await tester.pump();
    expect(service.tasks.last.cutoffUnixMs, cutoff);
    service.tasks.last.events.add(
      LibraryCleanupInspected(cleanupInspection(groups: 0)),
    );
    await tester.pumpAndSettle();
    expect(find.byType(AlertDialog), findsOneWidget);
    expect(find.text("没有符合条件的未关联记录"), findsOneWidget);
    expect(find.byKey(const Key("organization-cleanup-retry")), findsNothing);
    expect(service.tasks.every((task) => task.operationId == null), isTrue);
  });
}
