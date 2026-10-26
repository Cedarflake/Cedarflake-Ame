import "package:cedarflake_ame/app/notifications/ame_notification_controller.dart";
import "package:cedarflake_ame/app/presentation/ame_theme.dart";
import "package:cedarflake_ame/features/library/application/library_archive_controller.dart";
import "package:cedarflake_ame/features/library/application/library_archive_state.dart";
import "package:cedarflake_ame/features/library/application/library_controller.dart";
import "package:cedarflake_ame/features/library/application/library_organization_archive.dart";
import "package:cedarflake_ame/features/library/domain/library_organization_archive.dart";
import "package:cedarflake_ame/features/settings/application/organization_archive_picker.dart";
import "package:cedarflake_ame/features/settings/presentation/widgets/organization_archive_section.dart";
import "package:cedarflake_ame/features/settings/presentation/widgets/organization_record_feedback.dart";
import "package:flutter/material.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:flutter_test/flutter_test.dart";

import "../../library/support/library_archive_fixture.dart";
import "../../library/support/library_organization_fixture.dart";

void main() {
  late ArchiveServiceFixture service;
  late _Picker picker;
  late ProviderContainer container;
  setUp(() {
    service = ArchiveServiceFixture();
    picker = _Picker();
    container = ProviderContainer(
      overrides: [
        libraryOrganizationArchivesProvider.overrideWithValue(service),
        organizationArchivePickerProvider.overrideWithValue(picker),
        libraryControllerProvider.overrideWith(OrganizationLibraryFixture.new),
      ],
    );
  });
  tearDown(() => container.dispose());

  Future<void> host(WidgetTester tester, {double scale = 1}) =>
      tester.pumpWidget(
        UncontrolledProviderScope(
          container: container,
          child: MaterialApp(
            theme: buildAmeTheme(),
            builder: (context, child) => MediaQuery(
              data: MediaQuery.of(
                context,
              ).copyWith(textScaler: TextScaler.linear(scale)),
              child: child!,
            ),
            home: Scaffold(
              body: Stack(
                children: [
                  const SingleChildScrollView(
                    child: OrganizationArchiveSection(),
                  ),
                  Consumer(
                    builder: (context, ref, _) => Align(
                      alignment: Alignment.bottomCenter,
                      child:
                          buildOrganizationRecordFeedback(context, ref) ??
                          const SizedBox.shrink(),
                    ),
                  ),
                ],
              ),
            ),
          ),
        ),
      );

  Future<void> inspect(WidgetTester tester) async {
    await tester.tap(find.byKey(const Key("settings-archive-inspect")));
    await tester.pump();
    service.tasks.last.events.add(LibraryArchiveInspected(archiveInspection()));
    await tester.pump();
    expect(
      container.read(libraryArchiveControllerProvider),
      isA<LibraryArchiveReady>(),
    );
    await tester.pumpAndSettle();
  }

  testWidgets(
    "native picker cancellation starts no task and inspection requires confirmation",
    (tester) async {
      await host(tester);
      final cleanupPosition = tester.getTopLeft(find.text("清理未关联记录"));
      picker.selected = null;
      await tester.tap(find.byKey(const Key("settings-archive-export")));
      await tester.pumpAndSettle();
      expect(service.tasks, isEmpty);
      picker.selected = "备份.ame-backup";
      await inspect(tester);
      expect(
        find.byKey(const Key("organization-archive-confirmation")),
        findsOneWidget,
      );
      expect(find.text("可关联 7 组，暂未关联 3 组"), findsOneWidget);
      expect(service.tasks.single.kind, "inspect");
      await tester.tap(find.text("取消").last);
      await tester.pumpAndSettle();
      expect(service.tasks, hasLength(1));
      expect(
        container.read(libraryArchiveControllerProvider),
        isA<LibraryArchiveIdle>(),
      );
      await inspect(tester);
      await tester.tap(
        find.byKey(const Key("organization-archive-restore-confirm")),
      );
      await tester.pump();
      expect(service.tasks.last.kind, "restore");
      final task = service.tasks.last;
      task.events.add(LibraryArchiveRestored(task.operationId!));
      await tester.pumpAndSettle();
      expect(find.text("整理记录已恢复"), findsNothing);
      expect(find.text("关闭"), findsNothing);
      expect(tester.getTopLeft(find.text("清理未关联记录")), cleanupPosition);
      expect(
        container.read(ameNotificationControllerProvider).current?.title,
        "整理记录已恢复",
      );
      expect(tester.takeException(), isNull);
    },
  );

  testWidgets("returning after inspection uses the existing restore entry", (
    tester,
  ) async {
    await host(tester);
    final cleanupPosition = tester.getTopLeft(find.text("清理未关联记录"));
    await tester.tap(find.byKey(const Key("settings-archive-inspect")));
    await tester.pump();
    await tester.pumpWidget(const SizedBox.shrink());
    service.tasks.single.events.add(
      LibraryArchiveInspected(archiveInspection()),
    );
    await tester.pumpAndSettle();
    await host(tester);
    await tester.pumpAndSettle();
    expect(find.textContaining("已检查"), findsNothing);
    expect(find.text("查看备份"), findsOneWidget);
    expect(tester.getTopLeft(find.text("清理未关联记录")), cleanupPosition);
    await tester.tap(find.byKey(const Key("settings-archive-inspect")));
    await tester.pumpAndSettle();
    expect(
      find.byKey(const Key("organization-archive-confirmation")),
      findsOneWidget,
    );
    expect(service.tasks, hasLength(1));
    await tester.tap(find.text("取消"));
    await tester.pumpAndSettle();
    expect(find.text("选择文件"), findsOneWidget);
    expect(find.text("查看备份"), findsNothing);
  });

  testWidgets(
    "cancel remains stopping until the worker confirms cancellation",
    (tester) async {
      await host(tester);
      final cleanupPosition = tester.getTopLeft(find.text("清理未关联记录"));
      await tester.tap(find.byKey(const Key("settings-archive-export")));
      await tester.pump();
      expect(tester.getTopLeft(find.text("清理未关联记录")), cleanupPosition);
      expect(
        tester
            .getTopLeft(find.byKey(const Key("organization-record-feedback")))
            .dy,
        greaterThan(
          tester.getBottomLeft(find.byType(OrganizationArchiveSection)).dy,
        ),
      );
      await tester.tap(find.byKey(const Key("settings-archive-cancel")));
      await tester.pump();
      expect(find.text("正在停止"), findsOneWidget);
      expect(
        tester
            .widget<FilledButton>(
              find.byKey(const Key("settings-archive-export")),
            )
            .onPressed,
        isNull,
      );
      service.tasks.single.events.add(const LibraryArchiveCancelled());
      await tester.pump();
      expect(
        container.read(libraryArchiveControllerProvider),
        isA<LibraryArchiveFinished>(),
      );
      await tester.pumpAndSettle();
      expect(find.text("已停止导出"), findsNothing);
      expect(find.text("关闭"), findsNothing);
      expect(tester.getTopLeft(find.text("清理未关联记录")), cleanupPosition);
      expect(
        tester
            .widget<FilledButton>(
              find.byKey(const Key("settings-archive-export")),
            )
            .onPressed,
        isNotNull,
      );
    },
  );

  testWidgets("stale confirmation offers reinspection before another restore", (
    tester,
  ) async {
    await host(tester);
    final cleanupPosition = tester.getTopLeft(find.text("清理未关联记录"));
    await inspect(tester);
    await tester.tap(
      find.byKey(const Key("organization-archive-restore-confirm")),
    );
    await tester.pump();
    service.tasks.last.events.addError(
      const LibraryArchiveFailure(
        "organization_archive_inspection_changed",
        "stale",
      ),
    );
    await tester.pumpAndSettle();
    expect(find.text("重新检查"), findsOneWidget);
    expect(tester.getTopLeft(find.text("清理未关联记录")), cleanupPosition);
    await tester.tap(find.byKey(const Key("settings-archive-retry")));
    await tester.pump();
    expect(service.tasks.last.kind, "inspect");
    service.tasks.last.events.add(
      LibraryArchiveInspected(archiveInspection(revision: 11)),
    );
    await tester.pumpAndSettle();
    expect(
      find.byKey(const Key("organization-archive-confirmation")),
      findsOneWidget,
    );
    expect(service.tasks.where((task) => task.kind == "restore"), hasLength(1));
  });

  testWidgets("confirmation stays scrollable with enlarged desktop text", (
    tester,
  ) async {
    tester.view.physicalSize = const Size(800, 600);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);
    await host(tester, scale: 2);
    await inspect(tester);
    expect(
      tester
          .widget<AlertDialog>(
            find.byKey(const Key("organization-archive-confirmation")),
          )
          .scrollable,
      isTrue,
    );
    expect(
      find
          .byKey(const Key("organization-archive-restore-confirm"))
          .hitTestable(),
      findsOneWidget,
    );
    expect(tester.takeException(), isNull);
  });
}

class _Picker implements OrganizationArchivePicker {
  String? selected = "备份.ame-backup";
  @override
  Future<String?> destination() async => selected;
  @override
  Future<String?> backup() async => selected;
}
