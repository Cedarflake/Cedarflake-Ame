import "package:cedarflake_ame/app/bootstrap/ame_provider_assembly.dart";
import "package:cedarflake_ame/app/bootstrap/workbench_presentation_bindings.dart";
import "package:cedarflake_ame/app/window/ame_shutdown_coordinator.dart";
import "package:cedarflake_ame/features/library/presentation/library_workbench_destinations.dart";
import "package:cedarflake_ame/features/settings/presentation/default_album_picker.dart";
import "package:flutter_test/flutter_test.dart";

void main() {
  test("production workbench supplies its presentation contracts", () async {
    final shutdown = AmeShutdownCoordinator();
    final container = AmeProviderAssembly(shutdown: shutdown).assemble(
      requirements: workbenchPresentationRequirements,
      overrides: workbenchPresentationBindings(),
    );
    expect(
      container.read(librarySettingsDestinationProvider),
      isA<LibrarySettingsDestination>(),
    );
    expect(
      container.read(libraryRecordFeedbackProvider),
      isA<LibraryRecordFeedback>(),
    );
    expect(
      container.read(defaultAlbumPickerProvider),
      isA<DefaultAlbumPicker>(),
    );
    await shutdown.shutdown();
  });

  test("each missing destination fails before publication", () async {
    for (final omitted in workbenchPresentationRequirements) {
      final shutdown = AmeShutdownCoordinator();
      expect(
        () => AmeProviderAssembly(shutdown: shutdown).assemble(
          requirements: workbenchPresentationRequirements,
          overrides: workbenchPresentationBindings()
              .where((binding) => !identical(binding.origin, omitted))
              .toList(),
        ),
        throwsA(isA<Exception>()),
      );
      await shutdown.shutdown();
    }
  });
}
