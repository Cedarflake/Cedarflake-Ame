abstract final class LibraryStrings {
  static const appName = "Ame";
  static const synchronized = "已同步";
  static const synchronizing = "正在更新";
  static const needsReconciliation = "更新受阻";
  static const synchronizationUnconfirmed = "尚未核对";
  static const synchronizationRetryPending = "等待重试";
  static const continuityBaselineRequired = "正在核对";
  static const continuityCatchingUp = "正在同步";
  static const continuityRecoveryRequired = "需要核对";
  static const continuityLiveOnly = "关闭后需手动更新";
  static const continuityUnavailable = "无法自动更新";
  static const continuityLiveOnlyDetail =
      "重新打开 Ame 后，从文件夹菜单选择“更新图库”，补上关闭期间的变化。";
  static const continuityLiveOnlyNotice = "部分文件夹需要手动更新";
  static const continuityLiveOnlyNoticeDetail =
      "关闭期间的变化需要手动补上。重新打开 Ame 后，从文件夹菜单选择“更新图库”。";
  static const sourceUnavailable = "无法访问";
  static const sourceAvailable = "可用";
  static const sourceMissing = "文件夹不存在";
  static const sourceInaccessible = "无法访问";
  static const sourceOffline = "当前离线";
  static const sourceUnknown = "状态未知";
  static const addingSource = "正在添加";
  static const pausingSource = "正在暂停…";
  static const cancellingSource = "正在取消…";
  static const refreshingSourceDisplay = "正在刷新";
  static const sourceAdded = "已添加";
  static const sourceDisplayReloadPending = "需要重新加载";
  static const scanConfirmationPending = "扫描结果读取失败";
  static const scanConfirmationDetail = "在任务面板选择“重新读取”。";
  static const readingScanResult = "正在读取扫描结果…";
  static const rereadScanResult = "重新读取";
  static const galleryDisplayReloadPending = "图片加载失败";
  static const galleryDisplayReloadDetail = "在任务面板选择“重新加载”查看图片。";
  static const firstImportIncomplete = "尚未完成添加";
  static const firstImportInProgress = "正在添加图片";
  static const firstImportInProgressDetail = "添加完成后，图片会显示在这里。";
  static const firstImportPaused = "已暂停添加";
  static const firstImportPausedDetail = "在任务面板选择“继续”接着添加。";
  static const firstImportCancelled = "已取消添加";
  static const firstImportRetryDetail = "在任务面板选择“重试”。";
  static const firstImportGalleryDetail = "重新添加这个文件夹。";
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
  static const relocateLibrary = "重新定位文件夹";
  static const removeFromAme = "从 Ame 中移除";
  static const removeFolderFailed = "移除文件夹失败";
  static const removedFolderRefreshFailed = "文件夹已移除，图片加载失败";
  static String refreshingAfterRemoval(String sourceName) =>
      "已移除“$sourceName”，正在加载图片…";
  static const removingFolderDetail = "原文件夹和图片都会保留。";
  static const refreshingAfterRemovalDetail = "原文件夹和图片已保留。";
  static const expandFolder = "展开文件夹";
  static const collapseFolder = "折叠文件夹";
  static const loadingFolders = "正在加载文件夹…";
  static const showMoreFolders = "显示更多文件夹";
  static const retryFolders = "重新加载文件夹";
  static const unknownCaptureDate = "拍摄日期未知";
  static const retryPreview = "重新加载预览";
  static const retryingPreview = "正在加载预览…";
  static const originalImageUnavailable = "无法打开原图";
  static const originalImagePreviewFallback = "原图无法打开，已显示预览图";
  static const retryOriginalImage = "重新打开";

  static String removingFromAme(String sourceName) =>
      "正在从 Ame 中移除“$sourceName”…";
  static const previewUpdateRequired = "请先更新图库，再重新加载预览。";
  static const retryLoading = "重新加载";
  static const noFolder = "还没有添加文件夹";
  static const emptyLibraryTitle = "建立你的图片图库";
  static const emptyLibraryBody = "选择要在 Ame 中一起浏览的文件夹。";
  static const updatingLibrary = "正在更新图库…";
  static const loadingLibrary = "正在加载图库…";
  static const synchronizationRefreshFailureTitle = "图片加载失败";
  static const synchronizationRefreshFailureMessage = "重新加载以查看最新图片。";
  static const synchronizationSourceUnhealthy = "自动更新已中断。";
  static const synchronizationSourceStarting = "正在准备自动更新。";
  static const synchronizationSourceUnavailable = "打不开这个文件夹。";
  static const synchronizationEvidenceGap = "部分文件变化还没有更新到图库。";
  static const synchronizationCapacityExceeded = "文件变化较多，图库还未更新完成。";
  static const synchronizationMonitoringAccessDenied =
      "没有权限读取这个文件夹的变化，请检查访问权限。";
  static const synchronizationMonitoringPathUnavailable = "打不开这个文件夹。";
  static const synchronizationMonitoringCapacityExceeded = "文件变化较多，图库还未更新完成。";
  static const synchronizationMonitoringEventIncomplete = "无法读取部分文件的变化。";
  static const synchronizationMonitoringFailed = "自动更新已中断。";
  static const synchronizationRecoveryFailed = "文件夹检查未完成。";
  static const synchronizationRecoveryBlocked = "在文件夹菜单中选择“更新图库”重试。";
  static const synchronizationPersistenceFailed = "图库更新未能保存。";
  static const synchronizationNeedsReconciliation = "图库还没有完成更新。";
  static const synchronizationCompleted = "文件夹中的变化已更新到图库。";
  static const synchronizationAutomaticRetry = "部分内容还未更新，稍后会自动重试。";
  static const synchronizationLegacyRecoveryAuthorityMissing =
      "在文件夹菜单中选择“更新图库”，恢复自动更新。";
  static const synchronizationExplicitRecoveryRequired =
      "在文件夹菜单中选择“更新图库”，恢复自动更新。";
  static const synchronizationPhaseWatcherStartup = "正在准备自动更新";
  static const synchronizationPhaseInventoryEnumeration = "正在检查文件夹";
  static const synchronizationPhaseInventoryComparison = "正在检查文件变化";
  static const synchronizationPhaseQueuePublication = "正在更新图库";
  static const synchronizationPhaseRetryWait = "稍后自动重试";
  static const synchronizationPhaseReconciliation = "正在检查文件变化";
  static const synchronizationPhaseFullScan = "正在扫描文件夹";
  static const synchronizationPhaseBlocked = "更新未完成";
  static const synchronizationPhaseSynchronized = "同步完成";
  static const synchronizationPhaseUnavailable = "等待文件夹恢复";
  static const retry = "重试";
  static const reload = "重新加载";

  static String? scanFailureDetail(String? error) {
    if (error == null) return null;
    final separator = error.indexOf(":");
    final code = separator < 0 ? error : error.substring(0, separator);
    return switch (code) {
      "catalog_root_consolidation_incomplete" =>
        "上级文件夹没有扫描完整。请检查文件夹能否打开，以及扫描数量是否达到上限。已添加的子文件夹仍会保留。",
      "catalog_root_consolidation_busy" => "子文件夹还有未完成的扫描。请先继续或取消该任务，再添加父文件夹。",
      "catalog_root_consolidation_unpublished" => "先更新已添加的子文件夹，再添加上级文件夹。",
      "catalog_root_consolidation_changed" ||
      "catalog_root_consolidation_identity_changed" =>
        "无法合并已添加的子文件夹。请检查文件夹位置后重试。",
      "library_update_root_changed" => "无法更新这个文件夹，请检查它能否打开。",
      _ => null,
    };
  }

  static const noSearchResults = "没有找到匹配的图片";
  static const noSearchResultsHint = "请尝试其他名称或路径。";
  static const noSourceResults = "此文件夹中没有可显示的图片";
  static const noSourceResultsHint = "可以选择其他文件夹或返回全部图库。";
  static const backToLibrary = "返回图库";

  static String synchronizationReconciliationTitle(String rootName) =>
      "“$rootName”更新受阻";

  static String synchronizationUnconfirmedTitle(String rootName) =>
      "“$rootName”图库尚未核对";

  static String firstImportRequiredTitle(String rootName) =>
      "“$rootName”尚未完成添加";
}
