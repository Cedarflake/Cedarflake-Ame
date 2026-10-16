import "dart:convert";
import "dart:io";

import "package:cedarflake_ame/features/library/application/library_controller.dart";
import "package:cedarflake_ame/features/library/domain/library_models.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_photo_tile.dart";
import "package:flutter/material.dart";
import "package:flutter/semantics.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:flutter_test/flutter_test.dart";

import "../../../support/duplicate_review_fixtures.dart";
import "../../../support/semantics/retained_semantics_update_harness.dart";

void main() {
  RetainedSemanticsUpdateBinding();
  setUp(RetainedSemanticsUpdateValidator.instance.reset);

  testWidgets(
    "photo exposes one primary action while preview state changes",
    (tester) async {
      final directory = Directory.systemTemp.createTempSync(
        "ame-tile-semantics-",
      );
      final file = File("${directory.path}/preview.png")
        ..writeAsBytesSync(
          base64Decode(
            "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNk+A8AAQUBAScY42YAAAAASUVORK5CYII=",
          ),
        );
      addTearDown(() {
        file.deleteSync();
        directory.deleteSync();
      });
      final semantics = tester.ensureSemantics();
      var opens = 0;
      final container = ProviderContainer(
        overrides: [
          libraryControllerProvider.overrideWith(ReviewFixtureLibrary.new),
        ],
      );
      addTearDown(container.dispose);
      int? primaryNodeId;
      try {
        for (final status in [
          LibraryPreviewStatus.pending,
          LibraryPreviewStatus.ready,
          LibraryPreviewStatus.pending,
        ]) {
          final asset = reviewAsset("photo").withPreview(
            previewPath: file.path,
            previewStatus: status,
            width: 240,
            height: 160,
          );
          await tester.pumpWidget(
            UncontrolledProviderScope(
              container: container,
              child: MaterialApp(
                home: Scaffold(
                  body: Center(
                    child: LibraryPhotoTile(
                      asset: asset,
                      width: 240,
                      height: 160,
                      isSelecting: false,
                      isSelected: false,
                      onOpen: (_) => opens++,
                      onToggleSelection: (_) {},
                      onViewInformation: (_) {},
                      onCopyPath: (_) {},
                      onRevealFile: (_) {},
                    ),
                  ),
                ),
              ),
            ),
          );
          if (status == LibraryPreviewStatus.ready) {
            for (var attempt = 0; attempt < 40; attempt++) {
              await tester.runAsync(() async {
                await Future<void>.delayed(const Duration(milliseconds: 25));
              });
              await tester.pump();
              if (tester
                  .widgetList<RawImage>(find.byType(RawImage))
                  .any((image) => image.image != null)) {
                break;
              }
            }
            expect(
              tester.widget<RawImage>(find.byType(RawImage)).image,
              isNotNull,
            );
          }
          await tester.pumpAndSettle();
          final node = tester.getSemantics(find.byType(LibraryPhotoTile));
          final primaryActions = <SemanticsNode>[];
          void visit(SemanticsNode node) {
            if (node.getSemanticsData().hasAction(SemanticsAction.tap)) {
              primaryActions.add(node);
            }
            node.visitChildren((child) {
              visit(child);
              return true;
            });
          }

          visit(node);
          expect(primaryActions, hasLength(1), reason: status.name);
          expect(primaryActions.single.label, contains("photo.png"));
          primaryNodeId ??= primaryActions.single.id;
          expect(primaryActions.single.id, primaryNodeId);
          RetainedSemanticsUpdateValidator.instance.verifyLatestUpdate(
            trace: status.name,
          );
        }
        await tester.tap(find.byType(LibraryPhotoTile));
        expect(opens, 1);
        expect(tester.takeException(), isNull);
        await tester.pumpWidget(const SizedBox.shrink());
      } finally {
        semantics.dispose();
      }
    },
    variant: TargetPlatformVariant.only(TargetPlatform.windows),
  );
}
