abstract final class LibraryStrings {
  static const appName = "Ame";
  static const synchronized = "已同步";
  static const synchronizing = "正在更新图库";
  static const needsReconciliation = "更新受阻";
  static const sourceUnavailable = "目录不可用";
  static const sourceAvailable = "可用";
  static const sourceMissing = "文件夹不存在";
  static const sourceInaccessible = "无法访问";
  static const sourceOffline = "当前离线";
  static const sourceUnknown = "状态未知";
  static const addingSource = "正在添加";
  static const library = "图库";
  static const import = "导入";
  static const searchHint = "在图库中搜索";
  static const clearSearch = "清除搜索";
  static const addFolder = "添加文件夹到图库";
  static const settings = "设置";
  static const select = "选择";
  static const cancel = "取消";
  static const layout = "布局";
  static const sort = "排序";
  static const captureDate = "拍摄日期";
  static const createdDate = "创建日期";
  static const modifiedDate = "修改日期";
  static const fileName = "名称";
  static const ascending = "升序";
  static const descending = "降序";
  static const equalHeight = "等高";
  static const square = "方形";
  static const small = "小";
  static const medium = "中等";
  static const large = "大";
  static const more = "更多";
  static const selectAll = "全选";
  static const open = "打开";
  static const viewInformation = "查看信息";
  static const copyPath = "复制路径";
  static const openInExplorer = "在文件资源管理器中打开";
  static const updateLibrary = "更新图库";
  static const removeFromAme = "从 Ame 中移除";
  static const expandFolder = "展开文件夹";
  static const collapseFolder = "折叠文件夹";
  static const loadingFolders = "正在加载文件夹…";
  static const showMoreFolders = "显示更多文件夹";
  static const retryFolders = "重新加载文件夹";
  static const unknownCaptureDate = "拍摄日期未知";
  static const retryPreview = "重试预览";
  static const retryLoading = "重试加载";
  static const noFolder = "还没有添加文件夹";
  static const emptyLibraryTitle = "建立你的图片图库";
  static const emptyLibraryBody = "选择要在 Ame 中一起浏览的文件夹。";
  static const updatingLibrary = "正在更新图库…";
  static const synchronizationRefreshFailureTitle = "无法显示最新图库内容";
  static const synchronizationRefreshFailureMessage = "图库已经发生变化，但当前页面刷新失败。";
  static const synchronizationSourceUnhealthy = "目录监控已中断，Ame 将重新核对可能遗漏的变化。";
  static const synchronizationEvidenceGap = "检测到无法确认的文件变化，Ame 正在自动重新核对该目录。";
  static const synchronizationCapacityExceeded = "短时间内的文件变化过多，Ame 正在自动重新核对该目录。";
  static const synchronizationMonitoringAccessDenied =
      "Windows 拒绝了目录监控访问。Ame 会保留上次可信内容，并在恢复后重新核对。";
  static const synchronizationMonitoringPathUnavailable =
      "监控中的目录暂时不可用。Ame 会保留上次可信内容，并在目录恢复后重新核对。";
  static const synchronizationMonitoringCapacityExceeded =
      "短时间内的文件变化超出监控可确认范围，Ame 正在重新核对该目录。";
  static const synchronizationMonitoringEventIncomplete =
      "Windows 提供的文件变化信息不完整，Ame 正在重新核对可能受影响的内容。";
  static const synchronizationMonitoringFailed =
      "Windows 目录监控发生错误。Ame 正在重启监控，并会重新核对可能遗漏的变化。";
  static const synchronizationRecoveryFailed = "图库重新核对未能完成，Ame 将自动重试。";
  static const synchronizationPersistenceFailed = "图库更新记录保存失败，Ame 将自动重试。";
  static const synchronizationNeedsReconciliation = "Ame 无法证明当前图库与目录完全一致。";
  static const retry = "重试";
  static const noSearchResults = "没有找到匹配的图片";
  static const noSearchResultsHint = "请尝试其他名称或路径。";
  static const noSourceResults = "此文件夹中没有可显示的图片";
  static const noSourceResultsHint = "可以选择其他文件夹或返回全部图库。";
  static const backToLibrary = "返回图库";

  static String synchronizationReconciliationTitle(String rootName) =>
      "“$rootName”更新受阻";
}
