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
  static const continuityUnavailable = "自动更新暂不可用";
  static const continuityLiveOnlyDetail =
      "这个文件夹目前只能在 Ame 打开时自动更新。重新打开 Ame 后，请从文件夹菜单选择“更新图库”，补上关闭期间的文件变化。";
  static const continuityLiveOnlyNotice = "部分文件夹需要手动更新";
  static const continuityLiveOnlyNoticeDetail =
      "文件夹标有“关闭后需手动更新”时，重新打开 Ame 后请从该文件夹菜单选择“更新图库”。";
  static const sourceUnavailable = "文件夹暂不可用";
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
  static const sourceDisplayReloadPending = "等待刷新";
  static const scanConfirmationPending = "暂时无法读取扫描结果";
  static const scanConfirmationDetail = "在任务面板点击“重新读取”再试一次。";
  static const readingScanResult = "正在读取扫描结果…";
  static const rereadScanResult = "重新读取";
  static const galleryDisplayReloadPending = "图库已更新，等待刷新显示";
  static const galleryDisplayReloadDetail = "图库已经更新，请在任务面板点击“重新加载”查看最新内容。";
  static const firstImportIncomplete = "尚未完成添加";
  static const firstImportInProgress = "正在添加图片";
  static const firstImportInProgressDetail = "添加完成后，图片会显示在这里。";
  static const firstImportPaused = "已暂停添加";
  static const firstImportPausedDetail = "在任务面板点击“继续”，接着添加图片。";
  static const firstImportCancellingDetail = "取消后可以重新添加这个文件夹。";
  static const firstImportCancelled = "已取消添加";
  static const firstImportRetryDetail = "请在任务面板查看详情，然后重试。";
  static const firstImportAwaitingUser = "等待手动继续或重新添加";
  static const firstImportRequiredDetail =
      "此文件夹尚未完成首次添加，Ame 不会自动继续扫描。有可恢复任务时，请在任务面板点击“继续”；已取消的任务需要重新添加文件夹。";
  static const firstImportGalleryDetail = "重新添加这个文件夹，把图片加入图库。";
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
  static const removedFolderRefreshFailed = "文件夹已移除，刷新显示失败";
  static String refreshingAfterRemoval(String sourceName) =>
      "已从 Ame 中移除“$sourceName”，正在刷新显示…";
  static const removingFolderDetail = "正在清理该文件夹的图库记录。磁盘上的文件夹和图片不会被删除或修改。";
  static const refreshingAfterRemovalDetail =
      "移除已经完成。正在加载最新图库显示，磁盘上的文件夹和图片未被删除或修改。";
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
  static const synchronizationRefreshFailureTitle = "页面刷新失败";
  static const synchronizationRefreshFailureMessage = "图库已有新内容，请重新加载页面。";
  static const synchronizationSourceUnhealthy = "自动更新已中断，已有图片仍可浏览。";
  static const synchronizationSourceStarting = "正在准备自动更新，你可以继续浏览。";
  static const synchronizationSourceUnavailable = "暂时无法访问这个文件夹，已有图片仍可浏览。";
  static const synchronizationEvidenceGap = "部分文件变化还没有更新到图库。";
  static const synchronizationCapacityExceeded = "文件变化较多，图库还未更新完成。";
  static const synchronizationMonitoringAccessDenied =
      "没有权限读取这个文件夹的变化，请检查访问权限。";
  static const synchronizationMonitoringPathUnavailable =
      "暂时无法访问这个文件夹，已有图片仍可浏览。";
  static const synchronizationMonitoringCapacityExceeded = "文件变化较多，图库还未更新完成。";
  static const synchronizationMonitoringEventIncomplete = "部分文件变化暂时无法读取。";
  static const synchronizationMonitoringFailed = "自动更新已中断，已有图片仍可浏览。";
  static const synchronizationRecoveryFailed = "文件夹检查未完成，已有图片仍可浏览。";
  static const synchronizationRecoveryBlocked =
      "部分文件变化未能更新。可从文件夹菜单选择“更新图库”再次尝试。";
  static const synchronizationPersistenceFailed = "暂时无法保存图库更新，已有内容仍可浏览。";
  static const synchronizationNeedsReconciliation = "图库还没有完成更新。";
  static const synchronizationCompleted = "文件夹中的变化已更新到图库。";
  static const synchronizationAutomaticProgress = "正在更新，你可以继续浏览。";
  static const synchronizationAutomaticRetry = "部分内容还未更新，稍后会自动重试。";
  static const synchronizationLegacyRecoveryAuthorityMissing =
      "请更新一次图库，让 Ame 重新开始自动更新。";
  static const synchronizationExplicitRecoveryRequired = "请更新一次图库，之后即可恢复自动更新。";
  static const synchronizationPhaseWatcherStartup = "正在准备自动更新";
  static const synchronizationPhaseInventoryEnumeration = "正在检查文件夹";
  static const synchronizationPhaseInventoryComparison = "正在整理文件变化";
  static const synchronizationPhaseQueuePublication = "正在保存更新";
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
      "scan_terminal_reconciliation_failed" => "点击“重新读取”再试一次。",
      "catalog_root_consolidation_incomplete" =>
        "尚未完整读取父文件夹，原来的子文件夹已保留。请检查访问状态及扫描数量限制后重试。",
      "catalog_root_consolidation_busy" => "子文件夹还有未完成的扫描。请先继续或取消该任务，再添加父文件夹。",
      "catalog_root_consolidation_unpublished" =>
        "无法确认子文件夹的目录身份，原有内容已保留。请确认该目录可访问后重试。",
      "catalog_root_consolidation_changed" ||
      "catalog_root_consolidation_identity_changed" =>
        "文件夹位置或登记已变化，原来的子文件夹已保留。请确认目录后重试。",
      _ => error,
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
