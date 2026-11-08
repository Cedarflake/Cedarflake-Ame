import "package:cedarflake_ame/app/notifications/ame_notification_controller.dart";
import "package:cedarflake_ame/features/library/application/library_classification_lifecycle.dart";
import "package:cedarflake_ame/features/library/application/library_controller.dart";
import "package:cedarflake_ame/features/library/application/library_foreground_reads.dart";
import "package:cedarflake_ame/features/library/application/ports/library_classification.dart";
import "package:cedarflake_ame/features/library/presentation/library_classification_notifications.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_analysis_activity.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_bottom_feedback.dart";
import "package:material_ui/material_ui.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:flutter_test/flutter_test.dart";

import "../../../support/application_test_bindings.dart";
import "../support/library_organization_fixture.dart";

void main() {
  testWidgets(
    "a real lifecycle error reaches the bottom host and resolves after reopening",
    (tester) async {
      final runner = _Runner();
      final reads = LibraryForegroundReads();
      final lifecycle = LibraryClassificationLifecycle(
        runner,
        foregroundReads: reads,
      );
      final container = ProviderContainer(
        overrides: applicationTestOverrides([
          libraryControllerProvider.overrideWith(
            OrganizationLibraryFixture.new,
          ),
          libraryClassificationLifecycleProvider.overrideWith(
            (ref) => lifecycle,
          ),
        ]),
      );
      addTearDown(() {
        container.dispose();
        reads.dispose();
      });
      await tester.pumpWidget(
        UncontrolledProviderScope(
          container: container,
          child: MaterialApp(
            home: LibraryAnalysisActivity(
              child: Scaffold(
                body: LibraryBottomFeedback(
                  onRetryPrimary: () async {},
                  onNotificationAction: (_) {},
                ),
              ),
            ),
          ),
        ),
      );
      lifecycle.startInBackground();
      await tester.pumpAndSettle();
      expect(find.text(libraryClassificationFailureTitle), findsOneWidget);
      expect(
        container.read(ameNotificationControllerProvider).pendingIds,
        hasLength(1),
      );
      runner.fail = false;
      await lifecycle.changeConfiguration(() async {});
      await tester.pumpAndSettle();
      expect(find.text(libraryClassificationFailureTitle), findsNothing);
      expect(
        container.read(ameNotificationControllerProvider).pendingIds,
        isEmpty,
      );
      await lifecycle.close();
      await tester.pumpWidget(const SizedBox());
    },
  );

  test(
    "runtime failure is persistent, deduplicated and resolved after recovery",
    () {
      final container = ProviderContainer(
        overrides: applicationTestOverrides(),
      );
      addTearDown(container.dispose);
      final owner = container.read(ameNotificationControllerProvider.notifier);
      final feedback = LibraryClassificationNotifications(owner);
      feedback.accept(StateError("startup failed"));
      feedback.accept(StateError("still unavailable"));
      final failed = container.read(ameNotificationControllerProvider);
      expect(failed.history, hasLength(1));
      expect(failed.current!.isPersistent, isTrue);
      expect(failed.current!.title, libraryClassificationFailureTitle);
      expect(failed.current!.actionId, isNull);
      feedback.accept(null);
      final recovered = container.read(ameNotificationControllerProvider);
      expect(recovered.current, isNull);
      expect(recovered.history.single.isActive, isFalse);
    },
  );
}

class _Runner implements LibraryClassification {
  bool fail = true;
  @override
  Future<LibraryClassificationSession?> open() async {
    if (fail) throw StateError("model unavailable");
    return null;
  }
}
