import "dart:async";

import "package:cedarflake_ame/features/library/application/library_classification_review.dart";
import "package:cedarflake_ame/features/library/application/library_classification_settings.dart";
import "package:cedarflake_ame/features/library/application/library_controller.dart";
import "package:cedarflake_ame/features/library/domain/library_classification_configuration.dart";
import "package:cedarflake_ame/features/library/domain/library_classification_review.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_image_viewer.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_viewer_classification.dart";
import "package:flutter/gestures.dart";
import "package:flutter/material.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:flutter_test/flutter_test.dart";

import "../../../support/application_test_bindings.dart";
import "../../../support/duplicate_review_fixtures.dart";
import "../../../support/semantics/retained_semantics_update_harness.dart";
import "../support/library_classification_review_fixture.dart";
import "../support/library_organization_fixture.dart";

void main() {
  RetainedSemanticsUpdateBinding();
  setUp(RetainedSemanticsUpdateValidator.instance.reset);

  testWidgets("preview category refresh preserves retained Windows semantics", (
    tester,
  ) async {
    final semantics = tester.ensureSemantics();
    final service = ClassificationReviewFixture();
    final target = organizationTarget("one");
    final container = ProviderContainer(
      overrides: applicationTestOverrides([
        libraryClassificationReviewServiceProvider.overrideWithValue(service),
        libraryControllerProvider.overrideWith(OrganizationLibraryFixture.new),
        libraryClassificationSettingsProvider.overrideWith(
          (ref) async => LibraryClassificationSettings(
            revision: BigInt.one,
            dispatchRevision: BigInt.one,
            mode: LibraryClassificationMode.automatic,
            categories: const [
              LibraryClassificationCategory(
                id: "drawing",
                label: "插画",
                descriptions: ["drawing"],
              ),
            ],
            runId: "run",
            installationSelected: true,
          ),
        ),
      ]),
    );
    final pointer = await tester.createGesture(kind: PointerDeviceKind.mouse);
    try {
      await tester.pumpWidget(
        UncontrolledProviderScope(
          container: container,
          child: MaterialApp(
            home: Scaffold(
              body: LibraryImageViewer(
                asset: reviewAsset("one"),
                onBack: () {},
                onInformation: () {},
                onCopyPath: () {},
                onRevealFile: () {},
                organizationActions: Row(
                  mainAxisSize: MainAxisSize.min,
                  children: [
                    IconButton(
                      onPressed: () {},
                      icon: const Icon(Icons.favorite, semanticLabel: "收藏"),
                    ),
                    LibraryViewerClassification(target: target),
                  ],
                ),
              ),
            ),
          ),
        ),
      );
      await tester.pumpAndSettle();
      await pointer.addPointer(location: tester.getCenter(find.text("插画")));
      await pointer.moveTo(tester.getCenter(find.text("插画")));
      await tester.pump(const Duration(milliseconds: 700));
      for (var cycle = 0; cycle < 4; cycle++) {
        final pending = Completer<List<LibraryClassificationItem>>();
        service.pendingRead = pending;
        container.invalidate(libraryClassificationItemProvider(target));
        await tester.pump();
        RetainedSemanticsUpdateValidator.instance.verifyLatestUpdate(
          trace: "classification-loading-$cycle",
        );
        if (cycle == 2) {
          pending.completeError(StateError("catalog unavailable"));
        } else if (cycle == 1) {
          pending.complete(const []);
        } else {
          pending.complete(service.items);
        }
        await tester.pumpAndSettle();
        RetainedSemanticsUpdateValidator.instance.verifyLatestUpdate(
          trace: "classification-result-$cycle",
        );
        expect(tester.takeException(), isNull);
      }
      await tester.pumpWidget(const SizedBox.shrink());
      RetainedSemanticsUpdateValidator.instance.verifyLatestUpdate(
        trace: "classification-retired",
      );
    } finally {
      await pointer.removePointer();
      semantics.dispose();
      container.dispose();
    }
  }, variant: TargetPlatformVariant.only(TargetPlatform.windows));
}
