import "package:flutter_riverpod/misc.dart";

import "../../features/library/adapters/rust/rust_library_classification.dart";
import "../../features/library/adapters/rust/rust_library_classification_configuration.dart";
import "../../features/library/adapters/rust/rust_library_classification_review.dart";
import "../../features/library/adapters/rust/rust_library_model_installation.dart";
import "../../features/library/application/library_automatic_classification.dart";
import "../../features/library/application/library_classification_configuration.dart";
import "../../features/library/application/library_classification_lifecycle.dart";
import "../../features/library/application/library_classification_review.dart";
import "../../features/library/application/library_foreground_reads.dart";
import "../../features/library/application/library_model_activation.dart";
import "../../features/library/application/library_model_installation.dart";
import "../../features/library/application/ports/library_classification.dart";
import "../../features/library/application/ports/library_classification_configuration.dart";
import "../../features/library/application/ports/library_classification_review.dart";
import "../../features/library/application/ports/library_model_installation.dart";

class LibraryClassificationServices {
  const LibraryClassificationServices({
    required this.classification,
    required this.configuration,
    required this.review,
    required this.installation,
  });

  const LibraryClassificationServices.production()
    : classification = const RustLibraryClassification(),
      configuration = const RustLibraryClassificationConfiguration(),
      review = const RustLibraryClassificationReview(),
      installation = const RustLibraryModelInstallation();

  final LibraryClassification classification;
  final LibraryClassificationConfigurationService configuration;
  final LibraryClassificationReviewService review;
  final LibraryModelInstallationService installation;

  List<Override> get overrides => [
    libraryClassificationReviewServiceProvider.overrideWithValue(review),
  ];
}

class LibraryClassificationBindings {
  LibraryClassificationBindings({
    required LibraryClassificationServices services,
    required LibraryForegroundReads foregroundReads,
  }) : _services = services {
    lifecycle = LibraryClassificationLifecycle(
      LibraryAutomaticClassification(
        services.classification,
        services.configuration,
      ),
      foregroundReads: foregroundReads,
    );
    configuration = LibraryClassificationConfiguration(
      services.configuration,
      lifecycle,
    );
    modelInstallation = LibraryModelInstallation(services.installation);
  }

  final LibraryClassificationServices _services;
  late final LibraryClassificationLifecycle lifecycle;
  late final LibraryClassificationConfiguration configuration;
  late final LibraryModelInstallation modelInstallation;

  List<Override> get overrides => [
    ..._services.overrides,
    libraryClassificationLifecycleProvider.overrideWith((ref) => lifecycle),
    libraryClassificationConfigurationProvider.overrideWithValue(configuration),
    libraryModelInstallationProvider.overrideWith((ref) => modelInstallation),
  ];

  Future<void> recoverModelInstallation() =>
      modelInstallation.recover(activateLibraryModel(configuration));
}

final libraryClassificationRequirements = <ProviderListenable<Object?>>[
  libraryClassificationReviewServiceProvider,
  libraryClassificationLifecycleProvider,
  libraryClassificationConfigurationProvider,
  libraryModelInstallationProvider,
];
