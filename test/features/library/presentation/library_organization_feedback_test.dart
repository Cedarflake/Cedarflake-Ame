import "package:cedarflake_ame/app/notifications/ame_notification_controller.dart";
import "package:cedarflake_ame/app/presentation/ame_localizations.dart";
import "package:cedarflake_ame/app/presentation/ame_notifications.dart";
import "package:cedarflake_ame/features/library/application/library_controller.dart";
import "package:cedarflake_ame/features/library/application/library_operation_notice.dart";
import "package:cedarflake_ame/features/library/application/library_organization.dart";
import "package:cedarflake_ame/features/library/application/library_organization_actions.dart";
import "package:cedarflake_ame/features/library/presentation/library_classification_notifications.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_organization_feedback.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_viewer_notice.dart";
import "package:material_ui/material_ui.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:flutter_test/flutter_test.dart";

import "../../../support/localized_test_app.dart";
import "../support/library_organization_fixture.dart";
import "../../../support/application_test_bindings.dart";

void main() {
  late OrganizationServiceFixture service;
  late ProviderContainer container;
  setUp(() {
    service = OrganizationServiceFixture();
    container = ProviderContainer(
      overrides: applicationTestOverrides([
        libraryOrganizationProvider.overrideWithValue(service),
        libraryControllerProvider.overrideWith(OrganizationLibraryFixture.new),
      ]),
    );
  });
  tearDown(() => container.dispose());

  for (final locale in ameSupportedLocales) {
    testWidgets("acknowledges a persistent analysis notice in $locale", (
      tester,
    ) async {
      final notifications = container.read(
        ameNotificationControllerProvider.notifier,
      );
      final classification = LibraryClassificationNotifications(notifications);
      classification.accept(StateError("analysis failed"));
      final id = container.read(ameNotificationControllerProvider).current!.id;
      await tester.pumpWidget(_host(container, locale: locale));
      await tester.pump(const Duration(seconds: 6));
      final strings = tester.element(find.byType(LibraryViewerNotice)).l10n;
      expect(find.text(strings.acknowledge), findsOneWidget);
      expect(find.textContaining("analysis failed"), findsNothing);
      await tester.tap(find.byKey(const Key("viewer-organization-action")));
      await tester.pumpAndSettle();
      expect(find.byType(LibraryViewerNotice), findsNothing);
      final acknowledged = container.read(ameNotificationControllerProvider);
      expect(acknowledged.pendingIds, isNot(contains(id)));
      expect(acknowledged.history.single.id, id);
      expect(acknowledged.history.single.isActive, isTrue);
      classification.accept(null);
      expect(
        container
            .read(ameNotificationControllerProvider)
            .history
            .single
            .isActive,
        isFalse,
      );
      expect(tester.takeException(), isNull);
    });
  }

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

Widget _host(ProviderContainer container, {Locale locale = ameChineseLocale}) =>
    UncontrolledProviderScope(
      container: container,
      child: LocalizedTestApp(
        locale: locale,
        home: Scaffold(
          body: Consumer(
            builder: (context, ref, _) => Stack(
              fit: StackFit.expand,
              children: [
                const SizedBox.expand(),
                if (ref.watch(libraryViewerOperationNoticeProvider)
                    case final notice?)
                  LibraryOrganizationFeedback(notification: notice),
              ],
            ),
          ),
        ),
      ),
    );
