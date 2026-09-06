import "package:cedarflake_ame/features/library/application/library_source_reader.dart";
import "package:cedarflake_ame/features/library/application/rust_library_source_reader.dart";
import "package:cedarflake_ame/features/library/domain/library_models.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_loading_indicator.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_photo_tile.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_source_image.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_viewer_image.dart";
import "package:flutter/material.dart";
import "package:flutter_test/flutter_test.dart";

Future<void> verifyViewerSourceWorkflow(WidgetTester tester) async {
  final openingTile = find.byType(LibraryPhotoTile).hitTestable().first;
  final selected = tester.widget<LibraryPhotoTile>(openingTile).asset;
  const reader = RustLibrarySourceReader();
  await _withSourceLeases((held) async {
    held.add(await reader.acquire(selected));
    held.add(await reader.acquire(selected));
    await tester.tapAt(
      tester.getRect(openingTile).topLeft + const Offset(16, 16),
    );
    // An unchecked FileImage would decode despite both native slots being occupied.
    final deadline = DateTime.now().add(const Duration(seconds: 10));
    while (find.text("原图暂不可用，当前显示缩略图").evaluate().isEmpty &&
        find.text("无法打开原图").evaluate().isEmpty) {
      if (DateTime.now().isAfter(deadline)) {
        throw TestFailure("The viewer bypassed guarded source-read admission");
      }
      await tester.pump(const Duration(milliseconds: 25));
    }
  });
  await tester.tap(
    find.descendant(
      of: find.byType(LibraryViewerImage),
      matching: find.text("重试"),
    ),
  );
  await _waitForOriginal(tester);
  final first = tester
      .widget<LibraryViewerImage>(find.byType(LibraryViewerImage))
      .asset;

  await tester.tap(find.byKey(const Key("viewer-next")));
  await tester.pump();
  await _waitForOriginal(tester, differentFrom: first.locationId);
  final second = tester
      .widget<LibraryViewerImage>(find.byType(LibraryViewerImage))
      .asset;
  expect(second.locationId, isNot(first.locationId));

  await tester.tap(find.byKey(const Key("viewer-previous")));
  await tester.pump();
  await _waitForOriginal(tester, differentFrom: second.locationId);
  expect(
    tester
        .widget<LibraryViewerImage>(find.byType(LibraryViewerImage))
        .asset
        .locationId,
    first.locationId,
  );
  await tester.tap(find.byKey(const Key("viewer-back-button")));
  await tester.pumpAndSettle();
  expect(find.byType(LibraryViewerImage), findsNothing);
  expect(find.byKey(const Key("library-photo-wall")), findsOneWidget);

  // Both native slots must remain available after real decoding and viewer disposal.
  await _withSourceLeases((leases) async {
    for (final asset in <LibraryAsset>[first, second]) {
      final lease = await reader.acquire(asset);
      leases.add(lease);
      expect(lease.sourcePath, isNotEmpty);
    }
  });
}

Future<void> _withSourceLeases(
  Future<void> Function(List<LibrarySourceReadLease>) verify,
) async {
  final leases = <LibrarySourceReadLease>[];
  var verified = false;
  try {
    await verify(leases);
    verified = true;
  } finally {
    try {
      await Future.wait(
        leases.map((lease) => Future<void>.sync(lease.close)),
        eagerError: false,
      );
    } on Object catch (error, stackTrace) {
      if (verified) rethrow;
      debugPrint("Viewer fixture cleanup also failed: $error\n$stackTrace");
    }
  }
}

Future<void> _waitForOriginal(
  WidgetTester tester, {
  String? differentFrom,
}) async {
  final deadline = DateTime.now().add(const Duration(seconds: 30));
  while (!_hasDecodedOriginal(tester, differentFrom)) {
    if (DateTime.now().isAfter(deadline)) {
      throw TestFailure(
        "Native viewer did not decode its guarded original source",
      );
    }
    await tester.pump(const Duration(milliseconds: 25));
  }
  expect(find.text("原图暂不可用，当前显示缩略图"), findsNothing);
  expect(find.text("无法打开原图"), findsNothing);
}

bool _hasDecodedOriginal(WidgetTester tester, String? differentFrom) {
  final viewer = find.byType(LibraryViewerImage);
  if (viewer.evaluate().length != 1 ||
      tester.widget<LibraryViewerImage>(viewer).asset.locationId ==
          differentFrom) {
    return false;
  }
  final original = find.descendant(
    of: viewer,
    matching: find.byWidgetPredicate(
      (widget) => widget is Image && widget.image is LibrarySourceImage,
    ),
  );
  final decoded = find.descendant(
    of: original,
    matching: find.byType(RawImage),
  );
  return decoded.evaluate().length == 1 &&
      tester.widget<RawImage>(decoded).image != null &&
      find
          .descendant(
            of: viewer,
            matching: find.byType(LibraryLoadingIndicator),
          )
          .evaluate()
          .isEmpty &&
      find.text("原图暂不可用，当前显示缩略图").evaluate().isEmpty &&
      find.text("无法打开原图").evaluate().isEmpty;
}
