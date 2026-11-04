import "package:cedarflake_ame/app/bootstrap/library_classification_bindings.dart";
import "package:cedarflake_ame/features/library/application/library_classification_configuration.dart";
import "package:cedarflake_ame/features/library/application/library_classification_lifecycle.dart";
import "package:cedarflake_ame/features/library/application/library_classification_review.dart";
import "package:cedarflake_ame/features/library/application/library_controller.dart";
import "package:cedarflake_ame/features/library/application/library_model_installation.dart";
import "package:cedarflake_ame/features/library/domain/library_classification_configuration.dart";
import "package:cedarflake_ame/features/library/domain/library_classification_review.dart";
import "package:cedarflake_ame/features/library/domain/library_organization.dart";
import "package:flutter/foundation.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:flutter_test/flutter_test.dart";

Future<void> verifyClassificationBindingsWorkflow(
  ProviderContainer container,
  LibraryClassificationBindings bindings,
) async {
  final configuration = container.read(
    libraryClassificationConfigurationProvider,
  )!;
  expect(configuration, same(bindings.configuration));
  expect(
    container.read(libraryClassificationLifecycleProvider),
    same(bindings.lifecycle),
  );
  expect(
    container.read(libraryModelInstallationProvider),
    same(bindings.modelInstallation),
  );
  final settings = await configuration.load();
  expect(settings.installationSelected, isFalse);
  expect(settings.mode, LibraryClassificationMode.disabled);
  expect(settings.categories, isEmpty);
  bindings.lifecycle.startInBackground();
  await bindings.lifecycle.waitUntilOpened();
  expect(bindings.lifecycle.error, isNull);
  expect(bindings.modelInstallation.busy, isFalse);
  final navigation = await configuration.loadNavigation();
  expect(navigation.revision, settings.revision);
  expect(navigation.installationSelected, isFalse);

  final state = container.read(libraryControllerProvider);
  expect(state.catalogRevision, greaterThan(BigInt.zero));
  final target = LibraryOrganizationTarget.fromAsset(state.assets.first);
  final review = container.read(libraryClassificationReviewServiceProvider);
  final items = await review.loadItems([target]);
  expect(items, isEmpty);
  await expectLater(
    review.annotations(BigInt.zero, [target]),
    throwsA(
      isA<LibraryClassificationFailure>().having(
        (failure) => failure.code,
        "code",
        "catalog_revision_changed",
      ),
    ),
  );
  debugPrint("AME_CLASSIFICATION_BINDINGS_NATIVE_WORKFLOW_PASSED");
}
