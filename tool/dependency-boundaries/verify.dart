import "dart:convert";
import "dart:io";

import "boundary_rules.dart";
import "boundary_policy.dart";
import "dart_graph.dart";
import "generated_bridge_files.dart";
import "inventory_report.dart";
import "legacy_inventory.dart";

const _inventoryPath =
    "tool/dependency-boundaries/legacy_dart_dependencies.json";
const _policyPath = "tool/dependency-boundaries/dart_boundary_policy.json";

Map<String, Object?> _readObject(String path) =>
    jsonDecode(File(path).readAsStringSync(encoding: utf8))
        as Map<String, Object?>;

void main(List<String> arguments) {
  final reportInventory =
      arguments.length == 1 && arguments.single == "--inventory";
  if (arguments.isNotEmpty && !reportInventory) {
    throw const FormatException(
      "Only --inventory is supported; no baseline update mode exists",
    );
  }
  final root = Directory.current.uri;
  final modules = <String, DartModule>{};
  final sources =
      Directory("lib")
          .listSync(recursive: true, followLinks: false)
          .whereType<File>()
          .where((file) => file.path.endsWith(".dart"))
          .toList()
        ..sort((a, b) => a.path.compareTo(b.path));
  for (final file in sources) {
    final path = file.absolute.uri.toString().substring(root.toString().length);
    modules[Uri.decodeFull(path)] = parseDartModule(
      Uri.decodeFull(path),
      file.readAsStringSync(encoding: utf8),
      isGeneratedBridge: generatedBridgeFiles.contains(Uri.decodeFull(path)),
    );
  }
  final missingGenerated = generatedBridgeFiles.difference(
    modules.keys.toSet(),
  );
  if (missingGenerated.isNotEmpty) {
    throw FormatException("Stale generated bridge roster: $missingGenerated");
  }
  final policy = reportInventory && !File(_policyPath).existsSync()
      ? const DartBoundaryPolicy(activeExceptions: [], publicFeatureEdges: {})
      : DartBoundaryPolicy.parse(_readObject(_policyPath), modules);
  final violations = checkDartBoundaries(
    modules,
    publicFeatureEdges: policy.publicFeatureEdges,
  );
  if (reportInventory) {
    stdout.writeln(
      const JsonEncoder.withIndent(
        "  ",
      ).convert(dependencyInventoryReport(modules, violations)),
    );
    return;
  }
  final hash = Process.runSync("git", [
    "hash-object",
    "--path=$_inventoryPath",
    _inventoryPath,
  ]);
  if (hash.exitCode != 0) {
    throw ProcessException(
      "git",
      ["hash-object", _inventoryPath],
      hash.stderr as String,
      hash.exitCode,
    );
  }
  final inventory = parseLegacyInventory(
    _readObject(_inventoryPath),
    blobId: (hash.stdout as String).trim(),
  );
  final errors = compareLegacyDependencies(
    violations: violations,
    inventory: inventory,
    activeIds: policy.activeExceptions,
  );
  for (final entry in inventory.where(
    (entry) => policy.activeExceptions.contains(entry.id),
  )) {
    for (final coverage in entry.coverage) {
      if (!File(coverage).existsSync()) {
        errors.add("${entry.id} lost its regression owner: $coverage");
      }
    }
  }
  for (final error in errors) {
    stderr.writeln(error);
  }
  if (errors.isNotEmpty) {
    exitCode = 1;
    return;
  }
  stdout.writeln(
    "Dart boundaries: ${modules.length} modules; ${violations.length} frozen migration exceptions",
  );
}
