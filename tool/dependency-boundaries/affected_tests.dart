import "dart:collection";
import "dart:convert";
import "dart:io";

import "dart_graph.dart";

List<String> affectedTests(
  Map<String, DartModule> modules,
  Set<String> changed,
) {
  final tests = modules.keys.where(
    (path) => path.startsWith("test/") && path.endsWith("_test.dart"),
  );
  if (changed.any((path) => path.endsWith("/flutter_test_config.dart"))) {
    return tests.toList()..sort();
  }
  final importers = <String, Set<String>>{};
  for (final module in modules.values) {
    for (final dependency in module.dependencies) {
      importers.putIfAbsent(dependency.target, () => {}).add(module.path);
    }
  }
  final reached = {...changed};
  final pending = Queue<String>.of(changed);
  while (pending.isNotEmpty) {
    for (final importer in importers[pending.removeFirst()] ?? <String>{}) {
      if (reached.add(importer)) pending.add(importer);
    }
  }
  return tests.where(reached.contains).toList()..sort();
}

void main(List<String> arguments) {
  if (arguments.length != 1 ||
      !RegExp(r"^[0-9a-f]{40}$").hasMatch(arguments.single)) {
    throw const FormatException("An immutable base commit is required");
  }
  final changed = gitPaths([
    "diff",
    "--name-only",
    "--no-renames",
    arguments.single,
    "HEAD",
    "--",
  ]).toSet();
  final modules = <String, DartModule>{};
  for (final path in gitPaths(["ls-files", "--", "lib", "test"])) {
    final file = File(path);
    if (!path.endsWith(".dart") || !file.existsSync()) continue;
    modules[path] = parseDartModule(
      path,
      file.readAsStringSync(encoding: utf8),
      isGeneratedBridge: path.startsWith("lib/src/rust/"),
    );
  }
  stdout.writeln(jsonEncode(affectedTests(modules, changed)));
}

List<String> gitPaths(List<String> arguments) {
  final result = Process.runSync(
    "git",
    ["-c", "core.quotepath=false", ...arguments],
    stdoutEncoding: utf8,
    stderrEncoding: utf8,
  );
  if (result.exitCode != 0) {
    throw StateError(
      "Could not read the test dependency scope: ${result.stderr}",
    );
  }
  return const LineSplitter()
      .convert(result.stdout as String)
      .where((path) => path.isNotEmpty)
      .toList();
}
