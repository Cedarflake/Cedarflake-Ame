import "dart:convert";
import "dart:io";

import "package:analyzer/dart/analysis/utilities.dart";
import "package:analyzer/dart/ast/ast.dart";
import "package:analyzer/dart/ast/visitor.dart";

const catalogDirectory = "lib/app/presentation/l10n";
final han = RegExp(r"[\u3400-\u9fff]");
final englishWord = RegExp(r"[A-Za-z]{2,}");

void main() {
  final issues = <String>[];
  checkGeneratedArguments(issues);
  for (final file in Directory(
    "lib",
  ).listSync(recursive: true, followLinks: false).whereType<File>()) {
    final path = file.path.replaceAll("\\", "/");
    if (!path.endsWith(".dart") ||
        path.startsWith("lib/src/rust/") ||
        path.startsWith("$catalogDirectory/generated/")) {
      continue;
    }
    final source = file.readAsStringSync();
    final unit = parseString(content: source, path: path).unit;
    unit.accept(_ApplicationCopy(path, unit, issues));
  }
  if (issues.isNotEmpty) {
    stderr.writeln(issues.join("\n"));
    exitCode = 1;
  }
}

void checkGeneratedArguments(List<String> issues) {
  final template =
      jsonDecode(File("$catalogDirectory/app_zh.arb").readAsStringSync())
          as Map<String, Object?>;
  final file = File("$catalogDirectory/generated/app_localizations.dart");
  final unit = parseString(
    content: file.readAsStringSync(),
    path: file.path,
  ).unit;
  final api = unit.declarations.whereType<ClassDeclaration>().singleWhere(
    (declaration) => declaration.namePart.typeName.lexeme == "AppLocalizations",
  );
  // gen-l10n infers undeclared arguments from every supported language.
  final requiredKeys = template.keys
      .where((key) => !key.startsWith("@"))
      .toSet();
  for (final method in api.body.childEntities.whereType<MethodDeclaration>()) {
    final key = method.name.lexeme;
    if (!requiredKeys.remove(key)) continue;
    final metadata = template["@$key"] as Map<String, Object?>?;
    final arguments = metadata?["placeholders"] as Map<String, Object?>?;
    final declared = arguments?.keys.toSet() ?? <String>{};
    final generated = {
      for (final parameter
          in method.parameters?.parameters ?? <FormalParameter>[])
        parameter.name!.lexeme,
    };
    if (generated.length != declared.length ||
        !declared.containsAll(generated)) {
      issues.add(
        "Declare all arguments for $key in app_zh.arb. Generated $generated, declared $declared.",
      );
    }
  }
  if (requiredKeys.isNotEmpty) {
    issues.add("Missing generated messages: ${requiredKeys.join(', ')}");
  }
}

class _ApplicationCopy extends RecursiveAstVisitor<void> {
  _ApplicationCopy(this.path, this.unit, this.issues);
  final String path;
  final CompilationUnit unit;
  final List<String> issues;

  @override
  void visitSimpleStringLiteral(SimpleStringLiteral node) {
    _check(node, node.value);
  }

  @override
  void visitInterpolationString(InterpolationString node) {
    _check(node, node.value);
  }

  void _check(AstNode node, String text) {
    AstNode literal = node;
    while (literal.parent is StringInterpolation ||
        literal.parent is AdjacentStrings) {
      literal = literal.parent!;
    }
    final parent = literal.parent;
    final invocation = parent is ArgumentList ? parent.parent : null;
    final callee = switch (invocation) {
      MethodInvocation(:final methodName) => methodName.name,
      InstanceCreationExpression(:final constructorName) =>
        constructorName.type.toSource(),
      _ => null,
    };
    if (han.hasMatch(text)) {
      if (callee == "RegExp") return;
      if (path ==
              "lib/features/library/presentation/library_category_label.dart" &&
          parent is ConstantPattern) {
        return;
      }
    } else {
      final namedCopy =
          parent is NamedArgument &&
          const {
            "tooltip",
            "semanticLabel",
            "semanticsLabel",
            "helpText",
            "hintText",
            "labelText",
            "errorText",
            "confirmButtonText",
          }.contains(parent.name.lexeme);
      final directText = callee == "Text" || callee == "AmeDialogError";
      if (!namedCopy && !directText) return;
      if (!englishWord.hasMatch(text) ||
          const {"Ame", "Cedarflake Ame"}.contains(text.trim())) {
        return;
      }
    }
    final line = unit.lineInfo.getLocation(node.offset).lineNumber;
    issues.add(
      "$path:$line Move the fixed interface text into the ARB catalogs: $text",
    );
  }
}
