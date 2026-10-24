import "package:cedarflake_ame/app/notifications/ame_notification_controller.dart";
import "package:cedarflake_ame/app/presentation/ame_notifications.dart";
import "package:cedarflake_ame/features/library/application/library_controller.dart";
import "package:cedarflake_ame/features/library/application/library_organization.dart";
import "package:cedarflake_ame/features/library/application/library_organization_actions.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_organization_feedback.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_viewer_notice.dart";
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

  testWidgets(
    "uses the existing top viewer notice for undo and timed dismissal",
    (tester) async {
      await container
          .read(libraryOrganizationActionsProvider.notifier)
          .favorite(organizationTarget("one"), true);
      await tester.pumpWidget(_host(container));
      expect(find.byType(LibraryViewerNotice), findsOneWidget);
      expect(find.byType(AmeNotificationSurface), findsNothing);
      expect(find.byType(IconButton), findsNothing);
      expect(find.byType(TextButton), findsOneWidget);
      expect(find.text("撤销"), findsOneWidget);
      final placement = tester.widget<Positioned>(
        find.descendant(
          of: find.byType(LibraryViewerNotice),
          matching: find.byType(Positioned),
        ),
      );
      expect(placement.top, 16);
      expect(placement.bottom, isNull);
      final noticeHeight = tester
          .getSize(find.byType(LibraryViewerNotice))
          .height;
      await tester.tap(find.byKey(const Key("viewer-organization-action")));
      await tester.pumpAndSettle();
      expect(service.undos, hasLength(1));
      expect(find.text("已撤销"), findsOneWidget);
      expect(find.byType(TextButton), findsNothing);
      expect(
        tester.getSize(find.byType(LibraryViewerNotice)).height,
        noticeHeight,
      );
      await tester.pump(const Duration(seconds: 5));
      await tester.pump();
      expect(find.byType(LibraryViewerNotice), findsNothing);
      expect(
        container.read(ameNotificationControllerProvider).history,
        isEmpty,
      );
      expect(
        container.read(ameNotificationControllerProvider).hasUnread,
        isFalse,
      );
    },
  );

  testWidgets("failed saves keep the top retry and hide internal errors", (
    tester,
  ) async {
    service.save = (_) async => throw StateError("internal receipt failure");
    await container
        .read(libraryOrganizationActionsProvider.notifier)
        .favorite(organizationTarget("one"), true);
    await tester.pumpWidget(_host(container));
    await tester.pump(const Duration(seconds: 6));
    expect(find.text("重试"), findsOneWidget);
    expect(find.byType(IconButton), findsNothing);
    expect(find.textContaining("internal receipt"), findsNothing);
    service.save = null;
    await tester.tap(find.byKey(const Key("viewer-organization-action")));
    await tester.pumpAndSettle();
    expect(find.text("已加入收藏夹"), findsOneWidget);
    await tester.pump(const Duration(seconds: 5));
    await tester.pump();
    expect(find.byType(LibraryViewerNotice), findsNothing);
    expect(
      container.read(ameNotificationControllerProvider).history,
      hasLength(1),
    );
  });
}

Widget _host(ProviderContainer container) => UncontrolledProviderScope(
  container: container,
  child: MaterialApp(
    home: Scaffold(
      body: Consumer(
        builder: (context, ref, _) => Stack(
          fit: StackFit.expand,
          children: [
            const SizedBox.expand(),
            if (ref.watch(libraryOrganizationNoticeProvider) case final notice?)
              LibraryOrganizationFeedback(notification: notice),
          ],
        ),
      ),
    ),
  ),
);
