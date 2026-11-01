import "dart:async";

import "package:cedarflake_ame/features/library/application/library_classification_configuration.dart";
import "package:cedarflake_ame/features/library/application/library_classification_retry.dart";
import "package:cedarflake_ame/features/library/domain/library_organization.dart";

class ClassificationRetryFixture implements LibraryClassificationRetry {
  final targets = <LibraryOrganizationTarget>[];
  final revisions = <BigInt>[];
  Completer<LibraryClassificationRetryOutcome>? pending;
  LibraryClassificationRetryOutcome outcome =
      LibraryClassificationRetryOutcome.queued;

  @override
  Future<LibraryClassificationRetryOutcome> retry(
    LibraryOrganizationTarget target,
    BigInt expectedDispatchRevision,
  ) async {
    targets.add(target);
    revisions.add(expectedDispatchRevision);
    return pending?.future ?? outcome;
  }
}

LibraryClassificationSettings classificationRetrySettings({
  LibraryClassificationMode mode = LibraryClassificationMode.automatic,
}) => LibraryClassificationSettings(
  revision: BigInt.one,
  dispatchRevision: BigInt.from(7),
  mode: mode,
  categories: const [
    LibraryClassificationCategory(
      id: "drawing",
      label: "插画",
      descriptions: ["drawing"],
    ),
  ],
  runId: "run",
  installationSelected: true,
);
