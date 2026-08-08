import "package:cedarflake_ame/prototypes/r2a/r2a_prototype_app.dart";
import "package:flutter/foundation.dart";
import "package:flutter/gestures.dart";
import "package:flutter/material.dart";
import "package:flutter/services.dart";
import "package:flutter_test/flutter_test.dart";

void main() {
  Future<void> pumpPrototype(WidgetTester tester) async {
    tester.view.physicalSize = const Size(1440, 900);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);
    await tester.pumpWidget(const R2aPrototypeApp());
    await tester.pump();
  }

  testWidgets("uses one Simplified Chinese gallery shell", (tester) async {
    await pumpPrototype(tester);

    expect(find.text("图库"), findsNWidgets(2));
    expect(find.text("收藏夹"), findsOneWidget);
    expect(find.text("在图库中搜索"), findsOneWidget);
    expect(find.byKey(const Key("r2a-time-rail")), findsOneWidget);
    expect(find.text("去重"), findsNothing);
    expect(find.text("任务"), findsNothing);
    expect(find.text("分类"), findsNothing);
  });

  testWidgets("keeps primary actions inside a constrained desktop window", (
    tester,
  ) async {
    tester.view.physicalSize = const Size(960, 720);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);
    await tester.pumpWidget(const R2aPrototypeApp());
    await tester.pump();

    expect(find.byKey(const Key("r2a-global-import")), findsOneWidget);
    expect(find.byKey(const Key("r2a-settings-button")), findsOneWidget);
    expect(find.byKey(const Key("r2a-filter-menu")), findsOneWidget);
    expect(find.byKey(const Key("r2a-layout-menu")), findsOneWidget);
    expect(tester.takeException(), isNull);
  });

  testWidgets("anchors browsing actions to the gallery header right edge", (
    tester,
  ) async {
    await pumpPrototype(tester);

    final header = tester.getRect(find.byKey(const Key("r2a-gallery-header")));
    final toolbar = tester.getRect(
      find.byKey(const Key("r2a-browsing-toolbar")),
    );

    expect(header.right - toolbar.right, closeTo(20, 0.01));
  });

  testWidgets("keeps fixed rows and justifies only complete rows", (
    tester,
  ) async {
    await pumpPrototype(tester);

    for (var rowIndex = 0; rowIndex < 2; rowIndex++) {
      final row = find.byKey(ValueKey("r2a-justified-row-0-$rowIndex"));
      final tiles = find.descendant(
        of: row,
        matching: find.byWidgetPredicate((widget) {
          final key = widget.key;
          return key is ValueKey<String> && key.value.startsWith("r2a-asset-");
        }),
      );
      final rowRect = tester.getRect(row);
      final tileRects = [
        for (
          var tileIndex = 0;
          tileIndex < tiles.evaluate().length;
          tileIndex++
        )
          tester.getRect(tiles.at(tileIndex)),
      ];

      expect(tileRects, isNotEmpty);
      expect(tileRects.first.left, closeTo(rowRect.left, 0.01));
      expect(tileRects.map((rect) => rect.height).toSet(), hasLength(1));
      if (rowIndex == 0) {
        expect(tileRects.last.right, closeTo(rowRect.right, 0.01));
      } else {
        expect(tileRects.last.right, lessThan(rowRect.right));
      }
    }
  });

  testWidgets("uses one aligned folder source list", (tester) async {
    await pumpPrototype(tester);

    expect(find.text("OneDrive"), findsNothing);
    expect(find.text("此电脑"), findsNothing);

    final libraryTile = find.byKey(const Key("r2a-library-navigation"));
    final sourceTile = find.byKey(const ValueKey("r2a-source-picture"));
    final unavailableTile = find.byKey(const ValueKey("r2a-source-archive"));
    final libraryIcon = find.descendant(
      of: libraryTile,
      matching: find.byIcon(Icons.photo_library_outlined),
    );
    final sourceIcon = find.descendant(
      of: sourceTile,
      matching: find.byIcon(Icons.folder_outlined),
    );
    final unavailableIcon = find.descendant(
      of: unavailableTile,
      matching: find.byIcon(Icons.folder_off_outlined),
    );
    final addButton = find.descendant(
      of: libraryTile,
      matching: find.byKey(const Key("r2a-sidebar-import")),
    );
    final sourceMenu = find.descendant(
      of: sourceTile,
      matching: find.byType(PopupMenuButton<String>),
    );

    expect(tester.getCenter(libraryIcon).dx, tester.getCenter(sourceIcon).dx);
    expect(
      tester.getCenter(sourceIcon).dx,
      tester.getCenter(unavailableIcon).dx,
    );
    expect(tester.getCenter(addButton).dx, tester.getCenter(sourceMenu).dx);
  });

  testWidgets("composes the time rail around the Material slider", (
    tester,
  ) async {
    await pumpPrototype(tester);

    final photoWall = find.byKey(const Key("r2a-photo-wall"));
    final sliderFinder = find.byKey(const Key("timeline-slider"));
    final slider = tester.widget<Slider>(sliderFinder);
    final sliderTheme = tester.widget<SliderTheme>(
      find.ancestor(of: sliderFinder, matching: find.byType(SliderTheme)).first,
    );

    expect(slider.divisions, isNull);
    expect(slider.allowedInteraction, SliderInteraction.tapAndSlide);
    expect(slider.padding, const EdgeInsets.symmetric(horizontal: 12));
    expect(slider.semanticFormatterCallback?.call(1), "2026 年 8 月");
    expect(sliderTheme.data.trackHeight, 1);
    expect(sliderTheme.data.trackShape, isA<RoundedRectSliderTrackShape>());
    expect(sliderTheme.data.thumbShape, same(SliderComponentShape.noThumb));
    expect(sliderTheme.data.overlayShape, same(SliderComponentShape.noOverlay));
    expect(
      find.ancestor(of: photoWall, matching: find.byType(Scrollbar)),
      findsNothing,
    );
    expect(find.byKey(const Key("timeline-current-line")), findsOneWidget);

    final sliderAxis = tester.getCenter(sliderFinder).dx;
    final previousArrow = find.descendant(
      of: find.byKey(const Key("timeline-previous")),
      matching: find.byIcon(Icons.arrow_drop_up),
    );
    final nextArrow = find.descendant(
      of: find.byKey(const Key("timeline-next")),
      matching: find.byIcon(Icons.arrow_drop_down),
    );
    final yearMarker = tester.getRect(
      find.byKey(const ValueKey("time-label-2026-08")),
    );
    expect(tester.getRect(sliderFinder).width, kMinInteractiveDimension);
    expect(sliderAxis - yearMarker.right, greaterThanOrEqualTo(12));
    final firstAnnotation = tester.getCenter(
      find.byKey(const ValueKey("time-label-2026-08")),
    );
    expect(
      firstAnnotation.dy,
      closeTo(tester.getRect(sliderFinder).top + 12, 0.01),
    );
    final lastMarker = tester.getCenter(
      find.byKey(const ValueKey("time-marker-unknown")),
    );
    final trackBackground = tester.getRect(
      find.byKey(const Key("timeline-track-background")),
    );
    expect(firstAnnotation.dy - trackBackground.top, 12);
    expect(trackBackground.bottom - lastMarker.dy, greaterThan(12));
    expect(tester.getCenter(previousArrow).dx, closeTo(sliderAxis, 0.01));
    expect(tester.getCenter(nextArrow).dx, closeTo(sliderAxis, 0.01));
    expect(
      tester.getCenter(find.byKey(const ValueKey("time-marker-2026-07"))).dx,
      closeTo(sliderAxis, 0.01),
    );
  });

  testWidgets("suppresses the Windows desktop auto scrollbar on hover", (
    tester,
  ) async {
    debugDefaultTargetPlatformOverride = TargetPlatform.windows;
    TestGesture? mouse;
    try {
      await pumpPrototype(tester);

      final photoWall = find.byKey(const Key("r2a-photo-wall"));
      final wallRect = tester.getRect(photoWall);
      mouse = await tester.createGesture(kind: PointerDeviceKind.mouse);
      await mouse.addPointer(location: wallRect.center);
      await mouse.moveTo(Offset(wallRect.right - 2, wallRect.center.dy));
      await tester.pump(const Duration(milliseconds: 300));

      expect(
        find.ancestor(of: photoWall, matching: find.byType(Scrollbar)),
        findsNothing,
      );

      final scrollable = find.descendant(
        of: photoWall,
        matching: find.byType(Scrollable),
      );
      final position = tester.state<ScrollableState>(scrollable).position;
      await tester.drag(photoWall, const Offset(0, -300));
      await tester.pump();
      expect(position.pixels, greaterThan(0));
    } finally {
      await mouse?.removePointer();
      debugDefaultTargetPlatformOverride = null;
    }
  });

  testWidgets("preserves nonuniform global month marker distances", (
    tester,
  ) async {
    await pumpPrototype(tester);

    final august = tester.getCenter(
      find.byKey(const ValueKey("time-label-2026-08")),
    );
    final july = tester.getCenter(
      find.byKey(const ValueKey("time-marker-2026-07")),
    );
    final june = tester.getCenter(
      find.byKey(const ValueKey("time-marker-2026-06")),
    );

    final firstGap = july.dy - august.dy;
    final secondGap = june.dy - july.dy;
    expect(firstGap, greaterThan(secondGap * 3));
  });

  testWidgets("keeps the Material slider and gallery scroll in sync", (
    tester,
  ) async {
    await pumpPrototype(tester);

    final photoWall = find.byKey(const Key("r2a-photo-wall"));
    final sliderFinder = find.byKey(const Key("timeline-slider"));
    final scrollable = find.descendant(
      of: photoWall,
      matching: find.byType(Scrollable),
    );
    final position = tester.state<ScrollableState>(scrollable).position;

    expect(position.pixels, 0);
    tester.widget<Slider>(sliderFinder).onChanged?.call(0.28);
    await tester.pump();
    expect(position.pixels, closeTo(position.maxScrollExtent * 0.72, 0.5));

    await tester.drag(photoWall, const Offset(0, 220));
    await tester.pump();
    expect(tester.widget<Slider>(sliderFinder).value, greaterThan(0.28));
  });

  testWidgets("supports pointer and vertical keyboard navigation", (
    tester,
  ) async {
    await pumpPrototype(tester);

    final sliderFinder = find.byKey(const Key("timeline-slider"));
    final sliderRect = tester.getRect(sliderFinder);
    await tester.tapAt(
      Offset(sliderRect.right - 6, sliderRect.top + sliderRect.height * 0.7),
    );
    await tester.pump();

    final pointerValue = 1 - tester.widget<Slider>(sliderFinder).value;
    expect(pointerValue, greaterThan(0.5));

    tester.widget<Slider>(sliderFinder).focusNode?.requestFocus();
    await tester.pump();
    await tester.sendKeyEvent(LogicalKeyboardKey.arrowDown);
    await tester.pump();
    expect(
      1 - tester.widget<Slider>(sliderFinder).value,
      greaterThan(pointerValue),
    );
  });

  testWidgets("keeps exact duplicate behavior inside Filter", (tester) async {
    await pumpPrototype(tester);

    await tester.tap(find.byKey(const Key("r2a-filter-menu")));
    await tester.pumpAndSettle();

    expect(find.text("显示子文件夹"), findsOneWidget);
    expect(find.text("隐藏子文件夹"), findsOneWidget);
    expect(find.text("显示所有文件"), findsOneWidget);
    expect(find.text("合并完全相同图片"), findsOneWidget);
    expect(find.text("仅显示重复图片"), findsOneWidget);
    expect(find.text("审查重复组"), findsOneWidget);
  });

  testWidgets("uses independent layout shape and density groups", (
    tester,
  ) async {
    await pumpPrototype(tester);

    await tester.tap(find.byKey(const Key("r2a-layout-menu")));
    await tester.pumpAndSettle();

    expect(find.text("等高"), findsOneWidget);
    expect(find.text("方形"), findsOneWidget);
    expect(find.text("小"), findsOneWidget);
    expect(find.text("中等"), findsOneWidget);
    expect(find.text("大"), findsOneWidget);
  });

  testWidgets("replaces browsing actions with selection actions", (
    tester,
  ) async {
    await pumpPrototype(tester);

    await tester.tap(find.byKey(const Key("r2a-select-button")));
    await tester.pump();

    expect(find.byKey(const Key("r2a-selection-toolbar")), findsOneWidget);
    expect(find.byKey(const Key("r2a-filter-menu")), findsNothing);
    expect(
      find.byKey(const Key("r2a-cancel-selection")).hitTestable(),
      findsOneWidget,
    );

    await tester.tap(find.byKey(const ValueKey("r2a-asset-1")));
    await tester.pump();

    expect(find.text("已选择 1 个项目"), findsOneWidget);
    expect(find.text("加入相册"), findsOneWidget);
    expect(find.text("重复信息"), findsOneWidget);

    await tester.tap(find.byKey(const Key("r2a-cancel-selection")));
    await tester.pump();

    expect(find.byKey(const Key("r2a-selection-toolbar")), findsNothing);
    expect(find.byKey(const Key("r2a-select-button")), findsOneWidget);
    expect(find.text("已选择 1 个项目"), findsNothing);
  });

  testWidgets("shows plain-language settings without engineering controls", (
    tester,
  ) async {
    await pumpPrototype(tester);

    await tester.tap(find.byKey(const Key("r2a-settings-button")));
    await tester.pumpAndSettle();

    expect(find.byKey(const Key("r2a-settings-page")), findsOneWidget);
    expect(find.text("应用主题"), findsOneWidget);
    expect(find.text("图库数据位置"), findsOneWidget);
    expect(find.text("缩略图最大占用空间"), findsOneWidget);
    expect(find.textContaining("不会移动或复制原图片"), findsOneWidget);
    expect(find.text("AnalysisRun"), findsNothing);
    expect(find.text("Worker count"), findsNothing);
  });

  testWidgets("uses temporary import progress instead of task navigation", (
    tester,
  ) async {
    await pumpPrototype(tester);

    await tester.tap(find.byKey(const Key("r2a-global-import")));
    await tester.pump();

    expect(find.byKey(const Key("r2a-import-progress")), findsOneWidget);
    expect(find.textContaining("正在添加文件夹"), findsOneWidget);
    expect(find.text("任务"), findsNothing);

    await tester.tap(find.byKey(const Key("r2a-cancel-import")));
    await tester.pump();
    expect(find.byKey(const Key("r2a-import-progress")), findsNothing);
  });
}
