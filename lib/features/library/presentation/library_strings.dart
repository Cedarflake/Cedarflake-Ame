/// 图库文案按界面位置和使用场景分组；共用项注明受影响的位置。
///
/// 底部主任务卡的动态文案：library_primary_task_feedback.dart。
/// 底部更新任务行的动态文案：library_update_feedback.dart。
/// 通知列表、详情标题和通用按钮：lib/app/notifications/ame_notification_strings.dart。
abstract final class LibraryStrings {
  // 查看器底部｜缩略图条、幻灯片、缩放方式和全屏按钮的悬浮提示。
  static const showFilmstrip = "显示缩略图";
  static const hideFilmstrip = "隐藏缩略图";
  static const playSlideshow = "播放幻灯片（空格）";
  static const pauseSlideshow = "暂停幻灯片（空格）";
  static const fitImageToWindow = "适应窗口（0 / Ctrl+0）";
  static const showImageActualSize = "实际大小（1 / Ctrl+1）";
  static const enterViewerFullscreen = "全屏（F11）";
  static const exitViewerFullscreen = "退出全屏（Esc / F11）";

  // 顶部栏｜应用名称、搜索框占位文字和清除搜索按钮。
  static const appName = "Ame";
  static const searchHint = "在图库中搜索";
  static const clearSearch = "清除搜索";

  // 左侧栏｜图库入口、添加目录入口、设置入口和未添加目录的提示。
  // “图库”也用作主内容区的默认标题。
  static const library = "图库";
  static const addFolder = "添加目录到图库";
  static const settings = "设置";
  static const noFolder = "还没有添加目录";

  // 左侧栏｜目录树的展开、折叠、加载更多和加载失败后的重试。
  static const expandFolder = "展开目录";
  static const collapseFolder = "折叠目录";
  static const loadingFolders = "正在加载目录…";
  static const showMoreFolders = "显示更多目录";
  static const retryFolders = "重新加载目录";

  // 左侧栏｜目录名称下方的同步状态。
  // 状态选择见 library_synchronization_feedback.dart 和 widgets/library_source_navigation_tile.dart。
  static const synchronized = "已同步";
  static const synchronizing = "正在更新";
  static const needsReconciliation = "更新受阻";
  static const synchronizationUnconfirmed = "尚未核对";
  static const synchronizationRetryPending = "等待重试";
  static const continuityLiveOnly = "关闭后需手动更新";
  static const continuityUnavailable = "无法自动更新";

  // 左侧栏｜目录名称下方的访问状态。
  // sourceUnavailable 也用于“更新图库”对话框中无法访问的目录。
  static const sourceUnavailable = "无法访问";
  static const sourceAvailable = "可用";
  static const sourceMissing = "目录不存在";
  static const sourceInaccessible = "无法访问";
  static const sourceOffline = "当前离线";
  static const sourceUnknown = "状态未知";

  // 左侧栏｜新添加的目录尚未进入正式目录列表时显示的状态。
  // 对应 library_pending_source.dart；暂停、取消等共用文字见下方首次添加分组。
  static const addingSource = "正在添加";
  static const refreshingSourceDisplay = "正在刷新";
  static const sourceAdded = "已添加";
  static const sourceDisplayReloadPending = "需要重新加载";
  static const sourceScanResultReadFailed = "读取结果失败";
  static const sourceReadingScanResult = "正在读取结果";

  // 图库顶部｜选择模式、排序菜单、布局菜单和更多菜单。
  static const select = "选择";
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

  // 图库时间分组｜图片没有可用拍摄日期时的日期标题。
  static const unknownCaptureDate = "拍摄日期未知";

  // 右键菜单｜图片的打开、信息、复制路径，以及目录和图片的系统定位。
  // copyPath、openInExplorer 也用于查看器菜单；viewInformation 也用作图片信息标题。
  static const open = "打开";
  static const viewInformation = "查看信息";
  static const copyPath = "复制路径";
  static const openInExplorer = "在文件资源管理器中打开";

  // 目录菜单｜更新、重新定位和从 Ame 移除目录。
  // updateLibrary 也用于更新对话框、更新任务重试及同步通知的操作按钮。
  // removeFromAme 也用于移除确认按钮；updatingLibrary 用于更新对话框中的忙碌目录。
  static const updateLibrary = "更新图库";
  static const relocateLibrary = "重新定位目录";
  static const removeFromAme = "从 Ame 中移除";
  static const updatingLibrary = "正在更新图库…";

  // 共用操作｜图库退出选择、更新对话框和移除确认对话框的取消按钮。
  static const cancel = "取消";

  // 图库中央｜尚未添加任何目录时的标题、说明和导入按钮。
  static const emptyLibraryTitle = "建立你的图片图库";
  static const emptyLibraryBody = "选择要在 Ame 中一起浏览的目录吧。";
  static const import = "导入";

  // 图库中央｜正在读取图库内容时，加载圆圈下面的文字。
  static const loadingLibrary = "正在加载图库…";

  // 图库中央｜首次添加尚未完成时的标题和说明。
  // 具体状态配对见 library_first_import_feedback.dart；继续、取消等按钮位于底部任务卡。
  // firstImportIncomplete 同时用于侧栏目录状态和同步反馈中的首次添加状态。
  static const firstImportIncomplete = "尚未完成添加";

  // 首次添加｜正在扫描。
  static const firstImportInProgress = "正在添加图片";
  static const firstImportInProgressDetail = "添加完成后，图片就会显示在这里。";

  // 首次添加｜暂停请求正在处理、已经暂停。
  // pausingSource 同时用于侧栏新目录状态。
  static const pausingSource = "正在暂停";
  static const firstImportPaused = "已暂停添加";
  static const firstImportPausedDetail = "进度已保存，可以稍后继续。";

  // 首次添加｜取消请求正在处理、已经取消。
  // cancellingSource 同时用于侧栏新目录状态。
  static const cancellingSource = "正在取消";
  static const firstImportCancelled = "已取消添加";

  // 首次添加｜失败后的重试说明，以及取消后或其他未完成状态的重新添加说明。
  static const firstImportRetryDetail = "这个目录暂时无法添加到图库。";
  static const firstImportGalleryDetail = "需要时可以重新添加这个目录。";

  // 图库中央、底部任务卡｜扫描结果读取失败。
  // 标题在两处共用；说明只在图库中央显示，任务卡提供“重新读取”按钮。
  // 读取中的状态用于底部任务卡和添加按钮提示；侧栏目录使用上方的简短状态。
  static const scanConfirmationPending = "扫描结果读取失败";
  static const scanConfirmationDetail = "暂时无法显示扫描到的图片。";
  static const readingScanResult = "正在读取扫描结果…";
  static const rereadScanResult = "重新读取";

  // 图库中央、底部任务卡｜扫描已完成，随后图片加载失败。
  // 标题在两处共用；说明只在图库中央显示，任务卡提供“重新加载”按钮。
  static const galleryDisplayReloadPending = "图片加载失败";
  static const galleryDisplayReloadDetail = "图片已添加，暂时无法显示。";

  // 左侧栏｜“添加目录”按钮不可用时的悬停说明，解释当前任务为何阻止继续添加。
  // 仅在任务失败并等待恢复时使用；恢复进行中的提示由 LibraryPendingSource 选择。
  static const addFolderAwaitingScanResult = "扫描结果读取失败，暂时无法添加目录。";
  static const addFolderAwaitingGalleryReload = "图库加载失败，暂时无法添加目录。";

  // 图库中央｜搜索没有匹配结果，或选中目录没有可显示的图片。
  static const noSearchResults = "没有找到匹配的图片";
  static const noSearchResultsHint = "请尝试其他名称或路径。";
  static const noSourceResults = "此目录中没有可显示的图片";
  static const noSourceResultsHint = "可以选择其他目录，或返回图库。";

  // 图库内容区｜图片列表或分页加载失败后的重新加载按钮。
  // 也用于 library_query_failure_banner.dart 的查询失败提示。
  static const retryLoading = "重新加载";

  // 单张缩略图｜预览失败、正在重试，或文件变化后需要更新图库。
  // 用于图片上的提示、重试按钮及其悬停和无障碍说明。
  static const retryPreview = "重新加载预览";
  static const retryingPreview = "正在加载预览…";
  static const previewFormatUnrecognized = "无法识别图片";
  static const previewUpdateRequired = "请先更新图库，再重新加载预览。";

  // 原图查看器｜返回按钮的悬停说明，以及原图打开失败时的提示和重试按钮。
  // 有可用预览时使用 originalImagePreviewFallback，否则使用 originalImageUnavailable。
  static const backToLibrary = "返回图库";
  static const originalImageUnavailable = "无法打开原图";
  static const originalImagePreviewFallback = "原图无法打开，已暂时显示预览图";
  static const retryOriginalImage = "重新打开";

  // 底部任务卡｜移除目录的进度、后续图片加载和失败说明。
  // 添加、暂停、取消、完成的动态标题和数量说明见 library_primary_task_feedback.dart。
  static String removingFromAme(String sourceName) =>
      "正在从 Ame 中移除“$sourceName”…";
  static const removingFolderDetail = "原目录和图片都会保留。";
  static const removeFolderFailed = "移除目录失败";
  static String refreshingAfterRemoval(String sourceName) =>
      "已移除“$sourceName”，正在重新加载…";
  static const refreshingAfterRemovalDetail = "原目录和图片已保留。";
  static const removedFolderRefreshFailed = "目录已移除，图片加载失败";

  // 底部任务卡｜添加或更新失败的补充说明；也用于暂停时保留的错误。
  // 由错误代码选择可读说明，调用方为主任务卡和更新任务行。
  static String? scanFailureDetail(String? error) {
    if (error == null) return null;
    final separator = error.indexOf(":");
    final code = separator < 0 ? error : error.substring(0, separator);
    return switch (code) {
      "library_update_in_progress" => "另一个目录正在更新，结束后再继续。",
      "library_retained_cancel_failed" => "取消失败，请重试。",
      "catalog_root_consolidation_incomplete" =>
        "上级目录没有扫描完整。请检查目录能否打开，以及扫描数量是否达到上限。已添加的子目录仍会保留。",
      "catalog_root_consolidation_busy" => "子目录还有未完成的扫描。请先继续或取消该任务，再添加父目录。",
      "catalog_root_consolidation_unpublished" => "先更新已添加的子目录，再添加上级目录。",
      "catalog_root_consolidation_changed" ||
      "catalog_root_consolidation_identity_changed" =>
        "无法合并已添加的子目录。请检查目录位置后重试。",
      "library_update_root_changed" => "无法更新这个目录，请检查它能否打开。",
      _ => null,
    };
  }

  // 共用恢复按钮｜底部主任务卡的重试；主任务卡、更新任务行和加载失败通知的重新加载。
  static const retry = "重试";
  static const reload = "重新加载";

  // 同步通知｜自动更新后图片列表加载失败；通知提供重新加载按钮。
  // 本节通知内容同时用于底部通知和铃铛中的通知详情。
  static const synchronizationRefreshFailureTitle = "图片加载失败";
  static const synchronizationRefreshFailureMessage = "重新加载以查看最新图片。";

  // 同步通知｜部分目录在 Ame 关闭期间的变化需要手动更新。
  // Notice 和 NoticeDetail 是汇总通知的标题和正文。
  // Detail 是单个目录同步完成时的备用正文，当前通知策略不会单独显示它。
  static const continuityLiveOnlyNotice = "部分目录需要手动更新";
  static const continuityLiveOnlyNoticeDetail =
      "重新打开 Ame 后，在目录菜单中选择“更新图库”，补上关闭期间的变化。";
  static const continuityLiveOnlyDetail =
      "重新打开 Ame 后，在目录菜单中选择“更新图库”，补上关闭期间的变化。";

  // 同步通知｜带目录名称的标题；状态选择见 library_synchronization_feedback.dart。
  // 目前只发布受阻或中断通知，UnconfirmedTitle、firstImportRequiredTitle 不会显示。
  static String synchronizationReconciliationTitle(String rootName) =>
      "“$rootName”更新受阻";
  static String synchronizationUnconfirmedTitle(String rootName) =>
      "“$rootName”图库尚未核对";
  static String firstImportRequiredTitle(String rootName) =>
      "“$rootName”尚未完成添加";

  // 同步通知正文｜更新中断或尚未完成时的概况。
  // SourceUnavailable、Completed、AutomaticRetry 为其他状态生成正文，当前不会发布通知。
  static const synchronizationSourceUnavailable = "打不开这个目录。";
  static const synchronizationSourceUnhealthy = "自动更新已中断。";
  static const synchronizationSourceStarting = "正在准备自动更新。";
  static const synchronizationEvidenceGap = "部分文件变化还没有更新到图库。";
  static const synchronizationCapacityExceeded = "文件变化较多，图库还未更新完成。";
  static const synchronizationNeedsReconciliation = "图库还没有完成更新。";
  static const synchronizationCompleted = "目录中的变化已更新到图库。";
  static const synchronizationAutomaticRetry = "部分内容还未更新，稍后会自动重试。";

  // 同步通知正文｜恢复受阻，需要通过目录菜单手动更新。
  static const synchronizationRecoveryBlocked = "在目录菜单点击“更新图库”重试。";
  static const synchronizationLegacyRecoveryAuthorityMissing =
      "在目录菜单点击“更新图库”，恢复自动更新。";
  static const synchronizationExplicitRecoveryRequired =
      "在目录菜单点击“更新图库”，恢复自动更新。";

  // 同步通知正文｜已识别的访问、文件变化、检查或保存失败原因。
  static const synchronizationMonitoringAccessDenied = "没有权限读取这个目录的变化，请检查访问权限。";
  static const synchronizationMonitoringPathUnavailable = "打不开这个目录。";
  static const synchronizationMonitoringCapacityExceeded = "文件变化较多，图库还未更新完成。";
  static const synchronizationMonitoringEventIncomplete = "无法读取部分文件的变化。";
  static const synchronizationMonitoringFailed = "自动更新已中断。";
  static const synchronizationRecoveryFailed = "目录检查未完成。";
  static const synchronizationPersistenceFailed = "图库更新未能保存。";

  // 同步阶段的备用通知正文｜由同步状态选择当前一项。
  // 现行策略不发布更新中状态的通知，这组文字目前不会显示在通知中。
  static const synchronizationPhaseWatcherStartup = "正在准备自动更新";
  static const synchronizationPhaseInventoryEnumeration = "正在检查目录";
  static const synchronizationPhaseInventoryComparison = "正在检查文件变化";
  static const synchronizationPhaseQueuePublication = "正在更新图库";
  static const synchronizationPhaseRetryWait = "稍后自动重试";
  static const synchronizationPhaseReconciliation = "正在检查文件变化";
  static const synchronizationPhaseFullScan = "正在扫描目录";
  static const synchronizationPhaseBlocked = "更新未完成";
  static const synchronizationPhaseSynchronized = "同步完成";
  static const synchronizationPhaseUnavailable = "等待目录恢复";

  // 当前未被界面引用的旧状态文案｜保留定义，修改这些值不会改变现有界面。
  static const continuityBaselineRequired = "正在核对";
  static const continuityCatchingUp = "正在同步";
  static const continuityRecoveryRequired = "需要核对";
}
