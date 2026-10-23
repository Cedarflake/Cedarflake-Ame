import "dart:async";

import "package:cedarflake_ame/features/library/application/library_folder_root_loader.dart";
import "package:flutter_test/flutter_test.dart";

void main() {
  test("coalesces revisions while keeping root reads serial", () async {
    final first = Completer<void>();
    final requests = <(BigInt, String)>[];
    final loader = LibraryFolderRootLoader((revision, rootId) async {
      requests.add((revision, rootId));
      if (requests.length == 1) {
        await first.future;
      }
    });
    addTearDown(loader.dispose);

    loader.refresh(BigInt.one, ["first", "removed"]);
    await Future<void>.delayed(Duration.zero);
    loader.refresh(BigInt.two, ["first", "intermediate"]);
    loader.refresh(BigInt.from(3), ["first", "latest"]);
    await Future<void>.delayed(Duration.zero);
    expect(requests, [(BigInt.one, "first")]);

    first.complete();
    await Future<void>.delayed(Duration.zero);
    expect(requests, [
      (BigInt.one, "first"),
      (BigInt.from(3), "first"),
      (BigInt.from(3), "latest"),
    ]);
  });

  test("disposal retires queued roots and later refreshes", () async {
    final pending = Completer<void>();
    final requests = <String>[];
    final loader = LibraryFolderRootLoader((_, rootId) async {
      requests.add(rootId);
      await pending.future;
    });
    loader.refresh(BigInt.one, ["first", "second"]);
    await Future<void>.delayed(Duration.zero);
    loader.dispose();
    loader.refresh(BigInt.two, ["third"]);
    pending.complete();
    await Future<void>.delayed(Duration.zero);
    expect(requests, ["first"]);
  });

  test("a later root can load after an empty refresh", () async {
    final requests = <String>[];
    final loader = LibraryFolderRootLoader((_, rootId) async {
      requests.add(rootId);
    });
    addTearDown(loader.dispose);
    loader.refresh(BigInt.one, []);
    await Future<void>.delayed(Duration.zero);
    loader.refresh(BigInt.two, ["added"]);
    await Future<void>.delayed(Duration.zero);
    expect(requests, ["added"]);
  });
}
