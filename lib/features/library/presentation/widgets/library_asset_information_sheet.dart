import "package:material_ui/material_ui.dart";

import "../../../../app/presentation/ame_localizations.dart";
import "../../../../app/presentation/ame_scroll_motion.dart";
import "../../domain/library_models.dart";
import "../library_asset_summary.dart";
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
      child: AmeScrollMotion(
        child: SingleChildScrollView(
          padding: const EdgeInsets.fromLTRB(24, 8, 24, 24),
          child: Column(
            mainAxisSize: MainAxisSize.min,
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              Text(
                context.l10n.viewInformation,
                style: Theme.of(context).textTheme.titleLarge,
              ),
              const SizedBox(height: 16),
              Table(
                columnWidths: const {
                  0: IntrinsicColumnWidth(),
                  1: FlexColumnWidth(),
                },
                children: [
                  _informationRow(
                    context,
                    label: context.l10n.file,
                    value: displayLibraryFileName(asset.displayPath),
                  ),
                  _informationRow(
                    context,
                    label: context.l10n.path,
                    value: asset.displayPath,
                  ),
                  _informationRow(
                    context,
                    label: context.l10n.fileType,
                    value: libraryFileType(context.l10n, asset.displayPath),
                  ),
                  _informationRow(
                    context,
                    label: context.l10n.imageDimensions,
                    value: asset.width > 0 && asset.height > 0
                        ? "${asset.width} × ${asset.height}"
                        : context.l10n.unknown,
                  ),
                  _informationRow(
                    context,
                    label: context.l10n.size,
                    value: libraryFileSize(asset.fileSize),
                  ),
                  _informationRow(
                    context,
                    label: context.l10n.cameraMake,
                    value: asset.camera.make ?? context.l10n.unknown,
                  ),
                  _informationRow(
                    context,
                    label: context.l10n.cameraModel,
                    value: asset.camera.model ?? context.l10n.unknown,
                  ),
                  _informationRow(
                    context,
                    label: context.l10n.timeTaken,
                    value: _formatCaptureTime(context.l10n, asset.captureTime),
                  ),
                  if (asset.captureTime case final captureTime?)
                    _informationRow(
                      context,
                      label: context.l10n.timeSource,
                      value: _captureSourceLabel(
                        context.l10n,
                        captureTime.source,
                      ),
                    ),
                  _informationRow(
                    context,
                    label: context.l10n.timeCreated,
                    value: _formatUnixTime(context.l10n, asset.createdUnixMs),
                  ),
                  _informationRow(
                    context,
                    label: context.l10n.timeModified,
                    value: _formatUnixTime(context.l10n, asset.modifiedUnixMs),
                  ),
                ],
              ),
            ],
          ),
        ),
      ),
    ),
  );
}

TableRow _informationRow(
  BuildContext context, {
  required String label,
  required String value,
}) => TableRow(
  children: [
    Padding(
      padding: const EdgeInsets.fromLTRB(0, 8, 16, 8),
      child: Text(
        label,
        style: TextStyle(color: Theme.of(context).colorScheme.onSurfaceVariant),
      ),
    ),
    Padding(
      padding: const EdgeInsets.symmetric(vertical: 8),
      child: SelectableText(value),
    ),
  ],
);

String _formatCaptureTime(
  AppLocalizations strings,
  LibraryCaptureTimeEvidence? captureTime,
) {
  if (captureTime == null) {
    return strings.unknown;
  }
  final parsed = DateTime.tryParse(captureTime.localTime);
  return parsed == null
      ? captureTime.localTime
      : strings.photoTimestamp(parsed, parsed);
}

String _captureSourceLabel(
  AppLocalizations strings,
  LibraryCaptureTimeSource source,
) {
  return switch (source) {
    LibraryCaptureTimeSource.exifDateTimeOriginal => strings.cameraOriginalTime,
    LibraryCaptureTimeSource.exifDateTimeDigitized => strings.digitizedTime,
    LibraryCaptureTimeSource.exifDateTime => strings.photoRecordedTime,
  };
}

String _formatUnixTime(AppLocalizations strings, int? unixMs) {
  if (unixMs == null) {
    return strings.unknown;
  }
  final local = DateTime.fromMillisecondsSinceEpoch(unixMs);
  return strings.photoTimestamp(local, local);
}
