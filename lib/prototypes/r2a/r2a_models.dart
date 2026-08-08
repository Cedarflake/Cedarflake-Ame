import "package:flutter/material.dart";

enum R2aLayoutShape { equalHeight, square }

enum R2aThumbnailSize { small, medium, large }

enum R2aDuplicateMode { allFiles, mergedExact, duplicatesOnly }

enum R2aSortKey { captureDate, createdDate, modifiedDate, name }

enum R2aSortDirection { ascending, descending }

class R2aAsset {
  const R2aAsset({
    required this.id,
    required this.name,
    required this.path,
    required this.dateLabel,
    required this.aspectRatio,
    required this.colorValue,
    required this.icon,
    this.duplicateGroup,
    this.isFavorite = false,
  });

  final String id;
  final String name;
  final String path;
  final String dateLabel;
  final double aspectRatio;
  final int colorValue;
  final IconData icon;
  final String? duplicateGroup;
  final bool isFavorite;
}

class R2aSource {
  const R2aSource({
    required this.id,
    required this.label,
    required this.path,
    this.isAvailable = true,
  });

  final String id;
  final String label;
  final String path;
  final bool isAvailable;
}

class R2aTimelineBucket {
  const R2aTimelineBucket({
    required this.id,
    required this.year,
    required this.month,
    required this.contentExtent,
    this.isUnknown = false,
  });

  final String id;
  final int? year;
  final int? month;
  final double contentExtent;
  final bool isUnknown;

  String get label {
    if (isUnknown) {
      return "拍摄日期未知";
    }
    return "$year 年 $month 月";
  }
}

class R2aDuplicateGroup {
  const R2aDuplicateGroup({required this.id, required this.assets});

  final String id;
  final List<R2aAsset> assets;
}
