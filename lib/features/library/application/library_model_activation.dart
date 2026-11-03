import "dart:math";

import "library_classification_configuration.dart";
import "library_model_installation.dart";

EnablePreparedLibraryModel activateLibraryModel(
  LibraryClassificationConfiguration configuration,
) {
  final random = Random.secure();
  final operationId = List.generate(
    16,
    (_) => random.nextInt(256).toRadixString(16).padLeft(2, "0"),
  ).join();
  return (commit) async {
    final observed = await configuration.load();
    await configuration.enableManagedInstallation(
      operationId: operationId,
      observed: observed,
      commitInstallation: commit,
    );
  };
}
