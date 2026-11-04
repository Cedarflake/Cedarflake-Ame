import "dart:convert";
import "dart:io";

import "boundary_rules.dart";
import "dart_graph.dart";

Map<String, Object> dependencyInventoryReport(
  Map<String, DartModule> modules,
  List<BoundaryViolation> violations,
) {
  final testModules = <DartModule>[];
  final root = Directory.current.uri.toString();
  for (final directory in ["test", "integration_test"]) {
    for (final file in Directory(
      directory,
    ).listSync(recursive: true, followLinks: false).whereType<File>()) {
      if (!file.path.endsWith(".dart")) continue;
      final path = Uri.decodeFull(
        file.absolute.uri.toString().substring(root.length),
      );
      testModules.add(
        parseDartModule(path, file.readAsStringSync(encoding: utf8)),
      );
    }
  }
  testModules.sort((a, b) => a.path.compareTo(b.path));
  return {
    "violations": violations.map((violation) => violation.toJson()).toList(),
    "modules": [
      for (final module in modules.values)
        if (!module.isGeneratedBridge)
          _moduleInventory(module, modules.values, testModules),
    ],
  };
}

Map<String, Object> _moduleInventory(
  DartModule module,
  Iterable<DartModule> modules,
  List<DartModule> tests,
) {
  bool importsOwner(DartModule consumer) => consumer.dependencies.any(
    (dependency) => dependency.target == module.path,
  );
  final dedicatedTests = tests.where(importsOwner).toList();
  return {
    "path": module.path,
    "lines": module.lines,
    "declarations": module.declarations,
    "dependencies": module.dependencies.map((edge) => edge.target).toList(),
    "callers": modules
        .where(importsOwner)
        .map((consumer) => consumer.path)
        .toList(),
    "directTestLines": dedicatedTests.fold<int>(
      0,
      (sum, test) => sum + test.lines,
    ),
    "directTests": dedicatedTests.map((test) => test.path).toList(),
  };
}
