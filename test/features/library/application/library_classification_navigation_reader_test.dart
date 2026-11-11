import "dart:async";

import "package:cedarflake_ame/features/library/application/library_classification_navigation_reader.dart";
import "package:cedarflake_ame/features/library/domain/library_classification_configuration.dart";
import "package:flutter_test/flutter_test.dart";

void main() {
  testWidgets(
    "publications coalesce without hiding completed navigation reads",
    (tester) async {
      final reads = <Completer<LibraryClassificationSettings>>[];
      final reader = LibraryClassificationNavigationReader(() {
        final pending = Completer<LibraryClassificationSettings>();
        reads.add(pending);
        return pending.future;
      });
      final values = <LibraryClassificationSettings>[];
      final errors = <Object>[];
      final subscription = reader.changes.listen(
        values.add,
        onError: errors.add,
      );
      addTearDown(subscription.cancel);
      addTearDown(reader.close);
      reader.refresh();
      for (var i = 0; i < 20; i++) {
        reader.refresh();
      }
      expect(reads, hasLength(1));
      final first = _settings(1);
      reads[0].complete(first);
      await tester.pump();
      expect(values, [first]);
      expect(reads, hasLength(2));
      reads[1].completeError(StateError("catalog unavailable"));
      await tester.pump();
      expect(errors, hasLength(1));
      reader.refresh();
      final latest = _settings(2);
      reads[2].complete(latest);
      await tester.pump();
      expect(values, [first, latest]);
      reader.refresh();
      reader.close();
      reads[3].complete(_settings(3));
      await tester.pump();
      expect(values, [first, latest]);
    },
  );
}

LibraryClassificationSettings _settings(int revision) =>
    LibraryClassificationSettings(
      revision: BigInt.from(revision),
      dispatchRevision: BigInt.one,
      mode: LibraryClassificationMode.automatic,
      categories: const [],
      runId: "active",
      installationSelected: true,
    );
