import "package:flutter_rust_bridge/flutter_rust_bridge.dart" show Uint64List;

import "../../../../src/rust/api/file_operations.dart" as api;
import "../../../../src/rust/domain/file_operation_selection.dart" as dto;
import "../../../../src/rust/domain/organization.dart" as organization;
import "../../application/ports/library_file_operations.dart";
import "../../domain/library_file_operations.dart";
import "../../domain/library_organization.dart";
import "library_gallery_query_mapping.dart";
import "rust_library_file_mapping.dart";

class RustLibraryFileOperations implements LibraryFileOperations {
  const RustLibraryFileOperations(this.platform);
  final LibraryFileOperationPlatform platform;

  @override
  Future<LibraryFileSelectionDraft> open(LibraryFileSelection selection) =>
      fileOperationCall(() async {
        final scope = switch (selection) {
          LibraryFilesInGallery(:final selection) =>
            dto.FileOperationSelectionScope.gallery(
              query: mapLibraryGalleryQueryToRust(selection.query),
              catalogRevision: selection.catalogRevision,
              allMatching: selection.allMatching,
            ),
          LibraryFileGroupsInGallery(:final selection) =>
            dto.FileOperationSelectionScope.mergedGalleryGroups(
              query: mapLibraryGalleryQueryToRust(selection.query),
              catalogRevision: selection.catalogRevision,
              allMatching: selection.allMatching,
            ),
          LibraryFilesAtLocations() =>
            const dto.FileOperationSelectionScope.locations(),
        };
        return _Draft(await api.openFileSelection(scope: scope), platform);
      });

  @override
  Future<List<LibraryFileRecord>> history({BigInt? before, int limit = 40}) =>
      fileOperationCall(
        () async => (await api.loadFileOperationHistory(
          beforeId: before,
          limit: limit,
        )).map(fileRecordFromRust).toList(growable: false),
      );

  @override
  Future<LibraryFileTask> recover(
    List<BigInt> ids, {
    required bool continueMove,
  }) => fileOperationCall(
    () async => _Task(
      await api.recoverFileOperations(
        ids: Uint64List.fromList(ids),
        continueCopiedMove: continueMove,
        ownerWindow: await platform.ownerWindow(),
      ),
    ),
  );

  @override
  Future<void> openRecycleBin() => platform.openRecycleBin();

  @override
  Future<List<LibraryFileExport>> exports({BigInt? before, int limit = 40}) =>
      fileOperationCall(
        () async => (await api.loadFileExportHistory(
          beforeId: before,
          limit: limit,
        )).map(fileExportFromRust).toList(growable: false),
      );
  @override
  Future<LibraryFileTask> writeReport(String batchId) => fileOperationCall(
    () async => _Task(await api.writeFileExportReport(batchId: batchId)),
  );
}

class _Draft implements LibraryFileSelectionDraft {
  _Draft(this.value, this.platform);
  final api.FileSelectionDraft value;
  final LibraryFileOperationPlatform platform;
  Future<void>? _closing;

  @override
  Future<void> appendIds(List<String> ids) =>
      fileOperationCall(() => value.appendIds(assetIds: ids));
  @override
  Future<void> appendLocations(List<LibraryOrganizationTarget> locations) =>
      fileOperationCall(
        () => value.appendLocations(
          locations: [
            for (final item in locations)
              organization.OrganizationSelection(
                assetId: item.assetId,
                locationId: item.locationId,
                sourceGeneration: item.sourceGeneration,
              ),
          ],
        ),
      );
  @override
  Future<LibraryFileSelectionSummary> prepare() => fileOperationCall(() async {
    final result = await value.prepare();
    return LibraryFileSelectionSummary(
      result.fileCount.toInt(),
      result.totalBytes.toInt(),
      mergedGroups: result.mergedGroupCount?.toInt(),
    );
  });
  @override
  Future<LibraryFileTask> execute(String batchId, LibraryFileCommand command) =>
      fileOperationCall(
        () async => _Task(
          await value.execute(
            batchId: batchId,
            command: dto.FileOperationCommand(
              action: fileActionToRust(command.action),
              destinationDirectory: command.directory,
              newName: command.name,
              organizationRule: fileOrganizationRuleToRust(command.rule),
            ),
            ownerWindow: await platform.ownerWindow(),
          ),
        ),
      );
  @override
  Future<LibraryFileProgress> progress() => fileOperationCall(() async {
    final result = await value.progress();
    return LibraryFileProgress(
      phase: LibraryFileTaskPhase.preparing,
      total: result.totalFiles.toInt(),
      processed: result.preparedFiles.toInt(),
      path: result.currentPath,
    );
  });
  @override
  Future<void> cancel() => fileOperationCall(value.cancel);
  @override
  Future<void> close() => _closing ??= _close();
  Future<void> _close() async {
    await fileOperationCall(value.close);
    value.dispose();
  }
}

class _Task implements LibraryFileTask {
  _Task(this.value);
  final api.FileActionTask value;
  Future<void>? _closing;

  @override
  Future<LibraryFileProgress> progress() => fileOperationCall(
    () async => fileProgressFromRust(await value.progress()),
  );
  @override
  Future<void> cancel() => fileOperationCall(value.cancel);
  @override
  Future<void> close() => _closing ??= _close();
  Future<void> _close() async {
    await fileOperationCall(value.close);
    value.dispose();
  }
}
