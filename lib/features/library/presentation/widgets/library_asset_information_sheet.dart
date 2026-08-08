import "package:flutter/material.dart";

import "../../domain/library_models.dart";
import "../library_strings.dart";
import "library_path_text.dart";

Future<void> showLibraryAssetInformation(
  BuildContext context,
  LibraryAsset asset,
) {
  return showModalBottomSheet<void>(
    context: context,
    showDragHandle: true,
    isScrollControlled: true,
    builder: (context) => SafeArea(
      child: SingleChildScrollView(
        padding: const EdgeInsets.fromLTRB(24, 8, 24, 24),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            Text(
              LibraryStrings.viewInformation,
              style: Theme.of(context).textTheme.titleLarge,
            ),
            const SizedBox(height: 16),
            _InformationRow(
              label: "文件",
              value: displayLibraryFileName(asset.displayPath),
            ),
            _InformationRow(label: "路径", value: asset.displayPath),
            _InformationRow(label: "类型", value: _fileType(asset.displayPath)),
            _InformationRow(
              label: "尺寸",
              value: asset.width > 0 && asset.height > 0
                  ? "${asset.width} × ${asset.height}"
                  : "未知",
            ),
            _InformationRow(label: "大小", value: _formatBytes(asset.fileSize)),
            _InformationRow(
              label: "拍摄时间",
              value: _formatCaptureTime(asset.captureTime),
            ),
            if (asset.captureTime case final captureTime?)
              _InformationRow(
                label: "时间来源",
                value: _captureSourceLabel(captureTime.source),
              ),
            _InformationRow(
              label: "创建时间",
              value: _formatUnixTime(asset.createdUnixMs),
            ),
            _InformationRow(
              label: "修改时间",
              value: _formatUnixTime(asset.modifiedUnixMs),
            ),
          ],
        ),
      ),
    ),
  );
}

class _InformationRow extends StatelessWidget {
  const _InformationRow({required this.label, required this.value});

  final String label;
  final String value;

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 8),
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          SizedBox(
            width: 88,
            child: Text(
              label,
              style: TextStyle(
                color: Theme.of(context).colorScheme.onSurfaceVariant,
              ),
            ),
          ),
          Expanded(child: SelectableText(value)),
        ],
      ),
    );
  }
}

String _fileType(String path) {
  final fileName = path.replaceAll("\\", "/").split("/").last;
  final separator = fileName.lastIndexOf(".");
  if (separator < 0 || separator == fileName.length - 1) {
    return "未知";
  }
  return fileName.substring(separator + 1).toUpperCase();
}

String _formatCaptureTime(LibraryCaptureTimeEvidence? captureTime) {
  if (captureTime == null) {
    return "未知";
  }
  final parsed = DateTime.tryParse(captureTime.localTime);
  return parsed == null ? captureTime.localTime : _formatDateTime(parsed);
}

String _captureSourceLabel(LibraryCaptureTimeSource source) {
  return switch (source) {
    LibraryCaptureTimeSource.exifDateTimeOriginal => "相机原始拍摄时间",
    LibraryCaptureTimeSource.exifDateTimeDigitized => "数字化时间",
    LibraryCaptureTimeSource.exifDateTime => "图片记录时间",
  };
}

String _formatUnixTime(int? unixMs) {
  if (unixMs == null) {
    return "未知";
  }
  return _formatDateTime(DateTime.fromMillisecondsSinceEpoch(unixMs).toLocal());
}

String _formatDateTime(DateTime value) {
  String twoDigits(int part) => part.toString().padLeft(2, "0");
  return "${value.year}-${twoDigits(value.month)}-${twoDigits(value.day)} "
      "${twoDigits(value.hour)}:${twoDigits(value.minute)}:${twoDigits(value.second)}";
}

String _formatBytes(BigInt value) {
  final bytes = value.toDouble();
  const units = ["B", "KB", "MB", "GB", "TB"];
  var amount = bytes;
  var unitIndex = 0;
  while (amount >= 1024 && unitIndex < units.length - 1) {
    amount /= 1024;
    unitIndex++;
  }
  if (unitIndex == 0) {
    return "${value.toInt()} ${units[unitIndex]}";
  }
  return "${amount.toStringAsFixed(1)} ${units[unitIndex]}";
}
