import "package:flutter/material.dart";

import "r2a_models.dart";

abstract final class R2aFixtureData {
  static const sources = [
    R2aSource(
      id: "one-drive",
      label: "图片",
      path: "G:\\OneDrive - ExampleAccount\\图片",
    ),
    R2aSource(id: "picture", label: "Picture", path: "E:\\ExampleLibrary"),
    R2aSource(
      id: "archive",
      label: "旧图库",
      path: "H:\\Archive",
      isAvailable: false,
    ),
  ];

  static final assets = List<R2aAsset>.unmodifiable([
    ..._group(
      dateLabel: "2026年8月5日",
      start: 0,
      count: 11,
      colors: const [
        0xFFE9A5B7,
        0xFFA9C7FF,
        0xFFFFCE8A,
        0xFFB5DFD0,
        0xFFC6B8F4,
      ],
    ),
    ..._group(
      dateLabel: "2026年7月27日",
      start: 11,
      count: 10,
      colors: const [
        0xFF9FCED0,
        0xFFE8C0A0,
        0xFFAEC7A0,
        0xFFD6B3E8,
        0xFFF1A89D,
      ],
    ),
    ..._group(
      dateLabel: "2025年12月18日",
      start: 21,
      count: 8,
      colors: const [0xFFB4C4E7, 0xFFF0C5D8, 0xFFB8D9B1, 0xFFE9D28F],
    ),
    ..._group(
      dateLabel: "拍摄日期未知",
      start: 29,
      count: 5,
      colors: const [0xFFC7CBD8, 0xFFD7B9A7, 0xFFAEC6C6],
    ),
  ]);

  static const timelineBuckets = [
    R2aTimelineBucket(id: "2026-08", year: 2026, month: 8, contentExtent: 720),
    R2aTimelineBucket(id: "2026-07", year: 2026, month: 7, contentExtent: 180),
    R2aTimelineBucket(id: "2026-06", year: 2026, month: 6, contentExtent: 510),
    R2aTimelineBucket(id: "2026-04", year: 2026, month: 4, contentExtent: 110),
    R2aTimelineBucket(id: "2025-12", year: 2025, month: 12, contentExtent: 640),
    R2aTimelineBucket(id: "2025-11", year: 2025, month: 11, contentExtent: 95),
    R2aTimelineBucket(id: "2025-03", year: 2025, month: 3, contentExtent: 360),
    R2aTimelineBucket(id: "2023-09", year: 2023, month: 9, contentExtent: 220),
    R2aTimelineBucket(id: "2022-01", year: 2022, month: 1, contentExtent: 130),
    R2aTimelineBucket(
      id: "unknown",
      year: null,
      month: null,
      contentExtent: 260,
      isUnknown: true,
    ),
  ];

  static List<R2aAsset> _group({
    required String dateLabel,
    required int start,
    required int count,
    required List<int> colors,
  }) {
    const ratios = [0.72, 1.0, 1.42, 1.78, 0.82, 1.25];
    const icons = [
      Icons.auto_awesome_outlined,
      Icons.landscape_outlined,
      Icons.pets_outlined,
      Icons.local_florist_outlined,
      Icons.nightlight_outlined,
      Icons.coffee_outlined,
    ];
    return List.generate(count, (index) {
      final number = start + index + 1;
      final duplicateGroup = switch (number) {
        2 || 14 || 23 => "duplicate-a",
        7 || 19 => "duplicate-b",
        _ => null,
      };
      return R2aAsset(
        id: "asset-$number",
        name: "图片_${number.toString().padLeft(3, "0")}.jpg",
        path: number.isEven
            ? "E:\\ExampleLibrary\\旅行\\图片_$number.jpg"
            : "G:\\OneDrive - ExampleAccount\\图片\\收藏\\图片_$number.jpg",
        dateLabel: dateLabel,
        aspectRatio: ratios[number % ratios.length],
        colorValue: colors[number % colors.length],
        icon: icons[number % icons.length],
        duplicateGroup: duplicateGroup,
        isFavorite: number == 4 || number == 16,
      );
    });
  }
}
