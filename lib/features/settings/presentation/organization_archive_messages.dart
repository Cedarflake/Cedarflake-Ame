import "../../library/application/library_archive_requests.dart";
import "../../library/application/library_archive_state.dart";
import "../../library/domain/library_organization_archive.dart";

String archiveOperationName(LibraryArchiveRequest request) => switch (request) {
  LibraryArchiveExport() => "导出",
  LibraryArchiveInspect() => "检查",
  LibraryArchiveRestore() => "恢复",
};

String archiveProgressMessage(LibraryArchiveWorking state) {
  if (state.stopping) return "正在停止";
  return switch (state.phase) {
    LibraryArchivePhase.snapshot => "正在准备备份",
    LibraryArchivePhase.writing => "正在导出整理记录",
    LibraryArchivePhase.verifying => "正在检查备份",
    LibraryArchivePhase.matching => "正在关联图库中的图片",
    LibraryArchivePhase.restoring => "正在恢复整理记录",
    null => "正在${archiveOperationName(state.request)}整理记录",
  };
}

String archiveFailureMessage(
  LibraryArchiveFailure failure,
) => switch (failure.code) {
  "organization_archive_destination_protected" => "请选择图库文件夹和 Ame 数据文件夹以外的位置。",
  "organization_archive_destination_exists" => "这个文件已存在，请使用其他文件名。",
  "organization_archive_version_unsupported" => "暂不支持这份备份的版本，请选择兼容的备份。",
  "organization_archive_invalid" ||
  "organization_archive_records_invalid" => "这份备份不完整或内容有误，请重新选择。",
  "organization_archive_inspection_changed" => "图库或整理记录已更新，请重新检查备份后确认恢复。",
  "organization_archive_input_changed" => "所选备份已改变，请重新检查后确认恢复。",
  "organization_archive_busy" => "上一项备份操作尚未结束，请稍后重试。",
  "organization_archive_input_not_local" => "无法读取这份备份，请确认文件在本机可用。",
  "organization_archive_io" => "无法读写备份文件，请检查所选文件和保存位置后重试。",
  "organization_archive_identity_conflict" ||
  "organization_archive_history_conflict" ||
  "organization_archive_operation_conflict" => "这份备份与当前记录有冲突，当前记录已保留。请选择其他备份。",
  _ => "尚未确认操作结果，请重试。",
};

String archiveFilename(String path) => path.split(RegExp(r"[\\/]")).last;
