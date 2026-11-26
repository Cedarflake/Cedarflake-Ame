// ignore_for_file: deprecated_member_use

// ignore: unused_import
import 'package:intl/intl.dart' as intl;
import 'app_localizations.dart';

// ignore_for_file: type=lint

/// The translations for Chinese (`zh`).
class AppLocalizationsZh extends AppLocalizations {
  AppLocalizationsZh([String locale = 'zh']) : super(locale);

  @override
  String get diagnosticsTitle => '诊断信息';

  @override
  String get diagnosticsHelp => '保存运行信息，用于排查问题';

  @override
  String get diagnosticsExport => '导出';

  @override
  String get diagnosticsExportAction => '导出诊断信息';

  @override
  String get diagnosticsFileType => '诊断压缩包';

  @override
  String get diagnosticsCollecting => '正在导出诊断信息';

  @override
  String get diagnosticsSaved => '诊断信息已保存';

  @override
  String get diagnosticsFailed => '无法导出诊断信息';

  @override
  String get diagnosticsChooseAgain => '重新选择位置';

  @override
  String get diagnosticsOpenFailed => '无法打开保存位置';

  @override
  String get diagnosticsCancelled => '已取消导出';

  @override
  String get updateSettings => '更新设置';

  @override
  String get updatePolicy => '自动更新';

  @override
  String get updateChannel => '更新渠道';

  @override
  String get updateManual => '手动检查';

  @override
  String get updateCheckAutomatically => '自动检查更新';

  @override
  String get updateDownloadAutomatically => '自动下载更新';

  @override
  String get updateStable => '正式版';

  @override
  String get updatePreview => '预览版';

  @override
  String get updateCheck => '检查更新';

  @override
  String get updateView => '查看更新';

  @override
  String get updateRestart => '重启并更新';

  @override
  String get updateTitle => '更新 Ame';

  @override
  String get updateAvailable => '发现新版本';

  @override
  String get updateCurrent => '已是最新版本';

  @override
  String get updateChecking => '正在检查更新';

  @override
  String get updateDownloading => '正在下载更新';

  @override
  String get updateVerifying => '正在验证更新';

  @override
  String get updateStopping => '正在停止下载';

  @override
  String get updateReady => '更新已准备就绪';

  @override
  String get updateRestarting => '正在重启 Ame';

  @override
  String get updateDownload => '下载更新';

  @override
  String get updateLater => '稍后';

  @override
  String get updateStop => '停止下载';

  @override
  String get updateReadyHelp => '重启 Ame 后安装更新。';

  @override
  String get updateUnavailable => '请使用安装版或便携版检查更新。';

  @override
  String get updateNetworkFailure => '暂时无法检查或下载更新，请稍后重试。';

  @override
  String get updateReleaseFailure => '这个版本的更新信息不完整，请稍后重试。';

  @override
  String get updatePackageFailure => '下载的更新包不完整，请重新下载。';

  @override
  String get updateStorageFailure => '无法保存更新，请检查可用空间和文件夹权限。';

  @override
  String get updateInstallationFailure => '未能安装更新，当前版本仍可使用。';

  @override
  String get updateShutdownFailure => '还有操作未能结束，请稍后重试。';

  @override
  String get updateInstalled => 'Ame 已更新';

  @override
  String get updateCancelled => '已取消安装更新';

  @override
  String get updateFailed => '更新失败';

  @override
  String get settingsTitle => '设置';

  @override
  String get settingsPersonalization => '个性化';

  @override
  String get settingsLanguage => '应用语言';

  @override
  String get settingsLanguageHelp => '选择 Ame 界面使用的语言';

  @override
  String get languageSystem => '跟随系统';

  @override
  String get languageChinese => '简体中文';

  @override
  String get languageEnglish => 'English';

  @override
  String get languageJapanese => '日本語';

  @override
  String get settingsTheme => '应用主题';

  @override
  String get settingsThemeHelp => '选择明暗外观，主题色跟随 Windows';

  @override
  String get themeLight => '浅色';

  @override
  String get themeDark => '深色';

  @override
  String get settingsBrowsing => '浏览';

  @override
  String get settingsWheel => '鼠标滚轮';

  @override
  String get settingsWheelHelp => '查看单张图片时滚动滚轮的行为';

  @override
  String get settingsWheelZoom => '放大或缩小';

  @override
  String get settingsWheelNavigate => '上一张或下一张';

  @override
  String get settingsOpenPhoto => '打开图片';

  @override
  String get settingsOpenPhotoHelp => '自动模式缩小大图，小图保持实际大小';

  @override
  String get viewerAutomatic => '自动';

  @override
  String get viewerFitWindow => '适应窗口';

  @override
  String get viewerActualSize => '实际大小';

  @override
  String get settingsThumbnailSpeed => '缩略图加载速度';

  @override
  String get settingsThumbnailSpeedHelp => '档位越大，同时加载越多，占用资源也越高';

  @override
  String get sizeSmall => '小';

  @override
  String get sizeMedium => '中';

  @override
  String get sizeLarge => '大';

  @override
  String get settingsAbout => '关于';

  @override
  String get settingsVersion => '版本';

  @override
  String get settingsLicenses => '开源软件声明';

  @override
  String get settingsLicensesHelp => '查看 Ame 使用的开源软件与许可';

  @override
  String get settingsSaveFailed => '无法保存设置';

  @override
  String get startupFailureTitle => 'Cedarflake Ame 无法启动';

  @override
  String get copyDiagnostics => '复制诊断信息';

  @override
  String get diagnosticsCopied => '诊断信息已复制';

  @override
  String get startupUnsupportedCatalog => '图库数据来自不受支持的版本，无法安全打开。请导出诊断信息以便排查。';

  @override
  String get startupDatabaseError =>
      '图库数据暂时无法读取。请确认磁盘可用后重新启动；如果仍然失败，请导出诊断信息以便排查。';

  @override
  String get startupCatalogError => '图库加载失败。请重新启动 Ame；如果仍然失败，请导出诊断信息以便排查。';

  @override
  String get startupComponentError => '应用组件加载失败。请重新启动 Ame；如果仍然失败，请导出诊断信息以便排查。';

  @override
  String get choose => '选择';

  @override
  String get notSet => '未设置';

  @override
  String get notifications => '通知';

  @override
  String get notificationsEmpty => '暂无通知';

  @override
  String get notificationHistory => '历史记录';

  @override
  String get acknowledge => '知道了';

  @override
  String get notificationDetails => '通知详情';

  @override
  String get sourceDirectory => '目录';

  @override
  String get technicalCode => '技术代码';

  @override
  String get close => '关闭';

  @override
  String get operationFailedDetails => '操作未完成，详细信息已保留。';

  @override
  String get retry => '重试';

  @override
  String get errorCode => '错误代码';

  @override
  String get openLocation => '打开位置';

  @override
  String get openLocationFailed => '无法打开这个位置，请重试。';

  @override
  String get loading => '正在加载';

  @override
  String elapsedTime(String duration) {
    return '已持续 $duration';
  }

  @override
  String get occurredAt => '发生时间';

  @override
  String elapsedSeconds(int seconds) {
    return '$seconds 秒';
  }

  @override
  String elapsedMinutes(int minutes, int seconds) {
    return '$minutes 分 $seconds 秒';
  }

  @override
  String elapsedHours(int hours, int minutes) {
    return '$hours 小时 $minutes 分';
  }

  @override
  String get notificationsUnread => '通知，有未读消息';

  @override
  String get notificationsRead => '通知，无未读消息';

  @override
  String get windowMinimize => '最小化';

  @override
  String get windowRestore => '还原';

  @override
  String get windowMaximize => '最大化';

  @override
  String get chooseFolder => '选择文件夹';

  @override
  String get albumCreated => '已新建相册';

  @override
  String get albumRenamed => '已重命名相册';

  @override
  String get albumIconChanged => '已更改相册图标';

  @override
  String get albumDeleted => '已删除相册';

  @override
  String get removedFromAlbum => '已移出相册';

  @override
  String movedToAlbum(String name) {
    return '已移至“$name”';
  }

  @override
  String addedToAlbum(String name) {
    return '已加入“$name”';
  }

  @override
  String get archiveExported => '整理记录已导出';

  @override
  String get backupSaved => '备份已保存到所选位置。';

  @override
  String get archiveRestored => '整理记录已恢复';

  @override
  String get unlinkedRecordsKept => '暂未关联的记录已保留。';

  @override
  String get favoriteAdded => '已加入收藏夹';

  @override
  String get favoriteRemoved => '已取消收藏';

  @override
  String get unlinkedRecordsCleared => '已清理未关联记录';

  @override
  String recordsCleared(int count) {
    final intl.NumberFormat countNumberFormat =
        intl.NumberFormat.decimalPattern(localeName);
    final String countString = countNumberFormat.format(count);

    return '已清理 $countString 项记录。原图片和相册均已保留。';
  }

  @override
  String get changesSaveFailed => '未能保存更改';

  @override
  String get organizationRecordChanged => '图片或整理记录已更改，当前记录已保留。';

  @override
  String get organizationHistoryChanged => '操作记录已更新，请重新打开菜单。';

  @override
  String get organizationSelectionChanged => '图片或整理信息已更新，请重试。';

  @override
  String get organizationSelectionExpired => '图片或整理信息已更新，请重新选择图片。';

  @override
  String get changesUnconfirmed => '更改尚未确认保存，请重试。';

  @override
  String get undoCompleted => '已撤销';

  @override
  String get redoCompleted => '已重做';

  @override
  String get undoFailed => '无法撤销';

  @override
  String get redoFailed => '无法重做';

  @override
  String get undo => '撤销';

  @override
  String get changesSaved => '更改已保存';

  @override
  String get libraryRefreshPending => '图库尚未刷新，请重试。';

  @override
  String get locationRefreshFailed => '无法刷新图库位置信息，请重试。';

  @override
  String get displayRefreshSuperseded => '图库已保存，但当前显示刷新已被其他操作替换，请重试刷新显示。';

  @override
  String get deferredOrganization => '已移至稍后整理';

  @override
  String get ignored => '已忽略';

  @override
  String get restoredToInbox => '已恢复待整理';

  @override
  String get relocationUnsupported => '当前扫描服务不支持重新定位图库。';

  @override
  String get albumRemovalRemaining => '相册信息已更新，请重试剩余项目。';

  @override
  String albumRemovalFailed(String name) {
    return '未能删除“$name”，请重试。';
  }

  @override
  String get operationStopped => '操作已停止。';

  @override
  String get operationIncomplete => '未能完成操作，请重试。';

  @override
  String get folderStillUpdating => '文件夹状态已更改或仍在更新，请稍后重试。';

  @override
  String get folderRemovedRefreshPending => '文件夹已移除，图库尚未刷新。请关闭此窗口后重试图库刷新。';

  @override
  String get folderRemovalFailed => '未能移除文件夹，请重试。';

  @override
  String get scanEndedWithoutResult => '图库更新未返回完成结果';

  @override
  String get updatePausedRestart => '图库更新已暂停，请重新开始更新。';

  @override
  String get sourceChangedDuringUpdate => '更新期间源文件发生变化，请重试。';

  @override
  String get updateSourceChanged => '待更新的来源已发生变化，请重新选择来源。';

  @override
  String get addToAlbum => '加入相册';

  @override
  String get moveToAnotherAlbum => '移至其他相册';

  @override
  String get addOrMoveToAlbum => '加入或移至相册';

  @override
  String filterFormat(String format) {
    return '格式：$format';
  }

  @override
  String filterWidth(String range) {
    return '宽度：$range px';
  }

  @override
  String filterHeight(String range) {
    return '高度：$range px';
  }

  @override
  String filterCamera(String camera) {
    return '相机：$camera';
  }

  @override
  String filterFileSize(String range, String unit) {
    return '文件大小：$range $unit';
  }

  @override
  String get organizationEnterFailed => '无法进入整理模式，请重试。';

  @override
  String get galleryReturnFailed => '无法返回图库，请重试。';

  @override
  String get organizationScopeFailed => '无法切换整理范围，请重试。';

  @override
  String get conflictOpenFailed => '无法打开归属冲突的图片，请重试。';

  @override
  String get organizationPositionUnsaved => '整理位置尚未保存，请重试。';

  @override
  String get organizationPositionUnreadable => '无法读取整理位置，请重试。';

  @override
  String get organizationResumeFailed => '无法恢复整理位置，请重试。';

  @override
  String get organizationNextFailed => '更改已保存，无法打开下一张图片，请重试。';

  @override
  String get organizationLibraryUpdating => '图库正在更新，请重试切换图片。';

  @override
  String get organizationViewChanged => '图库窗口已变化，请重试切换图片。';

  @override
  String get unknown => '未知';

  @override
  String get smartAnalysisInterrupted => '智能相册分析已中断';

  @override
  String get smartResultsAvailable => '已有分类结果仍可浏览。';

  @override
  String get albumConflict => '相册归属冲突';

  @override
  String conflictsDeferred(int count) {
    final intl.NumberFormat countNumberFormat =
        intl.NumberFormat.decimalPattern(localeName);
    final String countString = countNumberFormat.format(count);

    return '$countString 组图片需要选择相册，已放入稍后整理。';
  }

  @override
  String get viewAction => '查看';

  @override
  String get duplicateWorkspace => '重复图片查找';

  @override
  String get openInExplorer => '在文件资源管理器中打开';

  @override
  String get openInExplorerFailed => '无法在文件资源管理器中打开';

  @override
  String get stopping => '正在停止';

  @override
  String get duplicateRunning => '正在查重';

  @override
  String get duplicateIncomplete => '未完成查重';

  @override
  String get waitingForBackground => '等待后台';

  @override
  String get paused => '已暂停';

  @override
  String get duplicateEmptyAfter => '已检查的图片中没有发现重复项';

  @override
  String get duplicateWaitResults => '找到的重复图片会显示在这里。';

  @override
  String get duplicateLocationsChanged => '该组某些图片的位置已发生变化';

  @override
  String get resultsUnavailable => '暂时无法读取结果';

  @override
  String get viewDuplicates => '查看重复图片';

  @override
  String get backToDuplicates => '返回重复图片';

  @override
  String get duplicateGroups => '重复组';

  @override
  String get ignoredGroupsEmpty => '没有已忽略的组';

  @override
  String get similarGroups => '相似组';

  @override
  String get similarityLevel => '相似程度';

  @override
  String get similarityStrict => '严格';

  @override
  String get similarityStandard => '标准';

  @override
  String get similarityBroad => '宽松';

  @override
  String get viewSimilar => '查看相似图片';

  @override
  String get similarEmpty => '已检查的图片中没有发现相似组';

  @override
  String get similarWaitResults => '找到的相似图片会显示在这里';

  @override
  String get comparePhotos => '对比图片';

  @override
  String get chooseComparison => '选择图片进行对比';

  @override
  String get syncComparison => '同步缩放和移动';

  @override
  String get resetComparison => '重置对比视图';

  @override
  String get zoomIn => '放大';

  @override
  String get zoomOut => '缩小';

  @override
  String get setPreferred => '设为首选';

  @override
  String get selectPhoto => '选择图片';

  @override
  String get deselectPhoto => '取消选择';

  @override
  String get clearPreferred => '取消首选';

  @override
  String get ignoreGroup => '忽略此组';

  @override
  String get restoreGroup => '恢复此组';

  @override
  String get selectionSaveFailed => '未能保存选择，请重试';

  @override
  String groupCount(int count) {
    final intl.NumberFormat countNumberFormat =
        intl.NumberFormat.decimalPattern(localeName);
    final String countString = countNumberFormat.format(count);

    return '$countString 个组';
  }

  @override
  String photoCount(int count) {
    final intl.NumberFormat countNumberFormat =
        intl.NumberFormat.decimalPattern(localeName);
    final String countString = countNumberFormat.format(count);

    return '$countString 张图片';
  }

  @override
  String deselectNamedGroup(String name) {
    return '取消选择图片组：$name';
  }

  @override
  String selectNamedGroup(String name) {
    return '选择图片组：$name';
  }

  @override
  String get filter => '筛选';

  @override
  String get showSubfolders => '显示子目录';

  @override
  String get hideSubfolders => '隐藏子目录';

  @override
  String get allPhotosFilter => '全部图片';

  @override
  String locationCount(int count) {
    final intl.NumberFormat countNumberFormat =
        intl.NumberFormat.decimalPattern(localeName);
    final String countString = countNumberFormat.format(count);

    return '$countString 个位置';
  }

  @override
  String duplicatePhotoCount(int count) {
    final intl.NumberFormat countNumberFormat =
        intl.NumberFormat.decimalPattern(localeName);
    final String countString = countNumberFormat.format(count);

    return '$countString 张重复图片';
  }

  @override
  String physicalFileCount(int count) {
    final intl.NumberFormat countNumberFormat =
        intl.NumberFormat.decimalPattern(localeName);
    final String countString = countNumberFormat.format(count);

    return '$countString 份文件';
  }

  @override
  String get cloudOnly => '仅在云端';

  @override
  String get folderOffline => '文件夹离线';

  @override
  String get temporarilyInaccessible => '暂时无法访问';

  @override
  String get deleteChangedFile => '文件已更改，未删除。请查看最新图片后重新选择并确认。';

  @override
  String get noPhotosToDelete => '没有可删除的图片';

  @override
  String aboutToDeletePhotos(int count) {
    final intl.NumberFormat countNumberFormat =
        intl.NumberFormat.decimalPattern(localeName);
    final String countString = countNumberFormat.format(count);

    return '即将删除 $countString 张图片';
  }

  @override
  String confirmDeleteFiles(int count) {
    final intl.NumberFormat countNumberFormat =
        intl.NumberFormat.decimalPattern(localeName);
    final String countString = countNumberFormat.format(count);

    return '删除 $countString 个文件？';
  }

  @override
  String confirmDeleteWholeGroup(int count) {
    final intl.NumberFormat countNumberFormat =
        intl.NumberFormat.decimalPattern(localeName);
    final String countString = countNumberFormat.format(count);

    return '删除这组图片的全部 $countString 个文件？';
  }

  @override
  String confirmDeleteGroups(int groups, int files) {
    final intl.NumberFormat groupsNumberFormat =
        intl.NumberFormat.decimalPattern(localeName);
    final String groupsString = groupsNumberFormat.format(groups);
    final intl.NumberFormat filesNumberFormat =
        intl.NumberFormat.decimalPattern(localeName);
    final String filesString = filesNumberFormat.format(files);

    return '删除 $groupsString 组图片，共 $filesString 个文件？';
  }

  @override
  String get destinationFolderDirect => '直接放入目标文件夹';

  @override
  String get byCategory => '按分类';

  @override
  String get byAlbum => '按相册';

  @override
  String get byDate => '按日期';

  @override
  String get bySourceFolder => '按来源文件夹';

  @override
  String get byAlbumYear => '按相册与年份';

  @override
  String get dateFolderRule => '按图库日期建立 YYYY-MM 文件夹；缺少日期的文件会跳过。';

  @override
  String get albumYearRule => '按图库日期放入相册名／年份文件夹，例如：旅行／2024。';

  @override
  String get unknownDestinationRule => '无法确定目标的文件会跳过，并在目标文件夹中生成结果清单。';

  @override
  String get copy => '复制';

  @override
  String get move => '移动';

  @override
  String get rename => '重命名';

  @override
  String get deleteAction => '删除';

  @override
  String filesCompleted(String action, int count) {
    final intl.NumberFormat countNumberFormat =
        intl.NumberFormat.decimalPattern(localeName);
    final String countString = countNumberFormat.format(count);

    return '$action $countString 个文件';
  }

  @override
  String get deleted => '已删除';

  @override
  String get completed => '已完成';

  @override
  String filesIncomplete(int count) {
    final intl.NumberFormat countNumberFormat =
        intl.NumberFormat.decimalPattern(localeName);
    final String countString = countNumberFormat.format(count);

    return '未完成 $countString 个';
  }

  @override
  String filesStopped(int count) {
    final intl.NumberFormat countNumberFormat =
        intl.NumberFormat.decimalPattern(localeName);
    final String countString = countNumberFormat.format(count);

    return '已停止 $countString 个';
  }

  @override
  String filesSkipped(int count) {
    final intl.NumberFormat countNumberFormat =
        intl.NumberFormat.decimalPattern(localeName);
    final String countString = countNumberFormat.format(count);

    return '已跳过 $countString 个';
  }

  @override
  String get noFilesProcessed => '未处理任何文件。';

  @override
  String get reportNotNeeded => '无需生成结果清单';

  @override
  String get reportSaved => '结果清单已保存';

  @override
  String get reportFailed => '结果清单未能生成';

  @override
  String get checkFileOperation => '核对文件操作';

  @override
  String get generateReport => '生成结果清单';

  @override
  String preparingOperation(String action) {
    return '正在准备$action';
  }

  @override
  String runningOperation(String action) {
    return '正在$action';
  }

  @override
  String operationEnded(String action) {
    return '$action已结束';
  }

  @override
  String operationStoppedLabel(String action) {
    return '$action已停止';
  }

  @override
  String get fileOperationNeedsAttention => '文件操作需要处理';

  @override
  String get notStarted => '尚未开始';

  @override
  String get verifyResults => '需要核对结果';

  @override
  String get moveIncomplete => '移动未完成';

  @override
  String get filesProcessedRefreshPending => '文件已处理，图库尚未更新';

  @override
  String get incomplete => '未完成';

  @override
  String get stopped => '已停止';

  @override
  String get resultsToVerify => '结果待核对';

  @override
  String get duplicateVerificationPending => '重复组尚未核对完成，请稍后重试。';

  @override
  String get selectedPhotosChanged => '所选图片已变化，请查看当前图库后重新选择。';

  @override
  String get selectedFilesChanged => '所选文件已变化，未执行操作。请查看最新文件后重新选择。';

  @override
  String get deleteFilesUnverified => '文件已变化或无法核对，未删除。请查看最新图片后重新选择并确认。';

  @override
  String get filesUnverified => '文件已变化或无法核对，未执行操作。请查看最新文件后重新选择。';

  @override
  String get multipleFileLocations => '所选图片有多个文件位置，请取消多选后，分别在图片右键菜单中操作。';

  @override
  String get destinationMatchesSource => '目标位置与原文件相同，请选择其他文件夹或名称。';

  @override
  String get destinationUnavailable => '无法打开目标文件夹，请重新选择。';

  @override
  String get selectedFilesUnreadable => '无法读取所选文件，请刷新图库后重新选择。';

  @override
  String get retainedPhotoChanged => '要保留的图片已变化或无法读取，尚未删除文件。请重新选择保留图片。';

  @override
  String get fileNameUnavailable => '文件名不可用，请换一个名称。';

  @override
  String get fileOperationPrepareFailed => '未能准备文件操作，请重新选择后重试。';

  @override
  String get fileOperationBusy => '还有文件操作正在处理，请稍后重试。';

  @override
  String get fileHistoryCheckRequired => '请在文件操作记录中核对结果后再继续。';

  @override
  String get fileOperationCancelled => '这次操作已取消。';

  @override
  String get reportWriteFailed => '文件操作结果已保留，但结果清单未能写入。可在记录中重新生成清单。';

  @override
  String get fileProcessingUncertain => '部分文件可能尚未处理，请查看文件操作记录。';

  @override
  String get moreFilters => '更多筛选';

  @override
  String get filterPhotos => '筛选图片';

  @override
  String get clearFilters => '清除筛选';

  @override
  String get cancel => '取消';

  @override
  String get apply => '应用';

  @override
  String get date => '日期';

  @override
  String get dateField => '日期类型';

  @override
  String get startDate => '开始日期';

  @override
  String get endDate => '结束日期';

  @override
  String get fileFormat => '文件格式';

  @override
  String get fileSize => '文件大小';

  @override
  String get width => '宽度';

  @override
  String get height => '高度';

  @override
  String get minimum => '最小值';

  @override
  String get maximum => '最大值';

  @override
  String get unit => '单位';

  @override
  String get dimensions => '图片尺寸';

  @override
  String get minimumWidth => '最小宽度';

  @override
  String get maximumWidth => '最大宽度';

  @override
  String get minimumHeight => '最小高度';

  @override
  String get maximumHeight => '最大高度';

  @override
  String get cameraInformation => '相机信息';

  @override
  String get cameraHint => '品牌或型号';

  @override
  String get invalidExtension => '最多选择 16 种格式，每种限 16 位字母或数字';

  @override
  String get invalidNumber => '请输入有效的非负数';

  @override
  String get invalidInteger => '请输入有效的非负整数';

  @override
  String get fractionalBytes => '换算后的大小需为整数字节';

  @override
  String get numberTooLarge => '数值过大';

  @override
  String get reversedRange => '最大值不能小于最小值';

  @override
  String get invalidDate => '请输入有效日期，如 2026-01-31';

  @override
  String get reversedDates => '结束日期不能早于开始日期';

  @override
  String get invalidCamera => '请输入不超过 128 个字符的品牌或型号';

  @override
  String get captureDate => '拍摄日期';

  @override
  String get createdDate => '创建日期';

  @override
  String get modifiedDate => '修改日期';

  @override
  String get favorites => '收藏夹';

  @override
  String get albums => '相册';

  @override
  String get smartAlbums => '智能相册';

  @override
  String get modelDownloading => '正在下载智能相册模型';

  @override
  String get modelVerifying => '正在校验模型';

  @override
  String get modelEnabling => '正在启用智能相册';

  @override
  String get modelEnabled => '已启用智能相册';

  @override
  String get modelEnableCancelled => '已取消启用智能相册';

  @override
  String get modelEnableFailed => '智能相册未能启用';

  @override
  String get modelPreparing => '正在准备智能相册模型';

  @override
  String get modelConfigUnreadable => '暂时无法读取模型配置，请重试。';

  @override
  String get modelDownloadRetained => '下载进度已保留，可以重试。';

  @override
  String get modelEnableRetry => '请重试启用，已有的完整模型文件会复用。';

  @override
  String get albumNameInvalid => '请输入 1 至 128 个字符的相册名称。';

  @override
  String get albumNameTaken => '已有同名相册，请换一个名字。';

  @override
  String get albumMissing => '这个相册已不存在，请重新选择。';

  @override
  String get organizationDataChanged => '图片或相册信息已更新，请检查后重试。';

  @override
  String get organizationPositionChanged => '整理位置已改变，请重新打开这张图片。';

  @override
  String get organizationReselectPhotos => '图片或整理信息已更新，请取消后重新选择图片。';

  @override
  String get noActionablePhotos => '没有可操作的图片，请重新选择。';

  @override
  String get previousOperationRunning => '上一项操作尚未结束，请稍后重试。';

  @override
  String get chooseConflictAlbum => '相册归属有冲突，请选择一个相册。';

  @override
  String get photoInformationUpdating => '图片信息正在更新，请稍后重试。';

  @override
  String get saveResultUnknown => '尚未确认保存结果，请重试。';

  @override
  String get stoppingCurrentTask => '正在停止当前任务';

  @override
  String get resumeOrCancelPaused => '请先继续或取消已暂停的任务';

  @override
  String get loadingLibrary => '正在加载图库…';

  @override
  String get selectingFolder => '正在选择目录…';

  @override
  String checkingFolderPhotos(String name) {
    return '正在检查“$name”的图片…';
  }

  @override
  String updatingFolder(String name) {
    return '正在更新“$name”…';
  }

  @override
  String addingFolder(String name) {
    return '正在添加“$name”…';
  }

  @override
  String get pausingEllipsis => '正在暂停…';

  @override
  String get stoppingEllipsis => '正在停止…';

  @override
  String get loadingPhotos => '正在加载图片…';

  @override
  String get updateLibraryCancelled => '已取消更新图库';

  @override
  String get addFolderCancelled => '已取消添加目录';

  @override
  String get updateLibraryPaused => '已暂停更新图库';

  @override
  String get addFolderPaused => '已暂停添加目录';

  @override
  String get filesChangedRetry => '文件已变化，请重试';

  @override
  String get updateLibraryFailed => '更新图库失败';

  @override
  String get addFolderFailed => '添加目录失败';

  @override
  String get updateComplete => '更新完成';

  @override
  String get addComplete => '添加完成';

  @override
  String totalPhotos(int count) {
    final intl.NumberFormat countNumberFormat =
        intl.NumberFormat.decimalPattern(localeName);
    final String countString = countNumberFormat.format(count);

    return '共 $countString 张图片';
  }

  @override
  String photosAdded(int count) {
    final intl.NumberFormat countNumberFormat =
        intl.NumberFormat.decimalPattern(localeName);
    final String countString = countNumberFormat.format(count);

    return '已添加 $countString 张图片';
  }

  @override
  String photosValidated(int checked, int total, int files) {
    final intl.NumberFormat checkedNumberFormat =
        intl.NumberFormat.decimalPattern(localeName);
    final String checkedString = checkedNumberFormat.format(checked);
    final intl.NumberFormat totalNumberFormat =
        intl.NumberFormat.decimalPattern(localeName);
    final String totalString = totalNumberFormat.format(total);
    final intl.NumberFormat filesNumberFormat =
        intl.NumberFormat.decimalPattern(localeName);
    final String filesString = filesNumberFormat.format(files);

    return '已检查 $checkedString / $totalString 张图片 · 已检查 $filesString 个文件';
  }

  @override
  String filesCheckedPhotosFound(int files, int photos) {
    final intl.NumberFormat filesNumberFormat =
        intl.NumberFormat.decimalPattern(localeName);
    final String filesString = filesNumberFormat.format(files);
    final intl.NumberFormat photosNumberFormat =
        intl.NumberFormat.decimalPattern(localeName);
    final String photosString = photosNumberFormat.format(photos);

    return '已检查 $filesString 个文件 · 已找到 $photosString 张图片';
  }

  @override
  String get chooseFolderAgain => '重新选择文件夹';

  @override
  String get retryRemoval => '重试移除';

  @override
  String get photos => '图片';

  @override
  String get removeFromLater => '取消稍后整理';

  @override
  String get organizeLater => '稍后整理';

  @override
  String get unignore => '取消忽略';

  @override
  String get ignoreSelected => '忽略所选';

  @override
  String get ignorePhoto => '忽略此图';

  @override
  String get photoAnalysisPaused => '图片分析已暂停';

  @override
  String get searchUnavailable => '搜索暂时不可用';

  @override
  String get retryAnalysis => '重试分析';

  @override
  String get searchContent => '画面内容';

  @override
  String get searchNamePath => '文件名和路径';

  @override
  String get searchCurrentScope => '当前范围';

  @override
  String get searchEntireLibrary => '整个图库';

  @override
  String get findRelatedPhotos => '查找相关图片';

  @override
  String get updateSearchResults => '更新搜索结果';

  @override
  String get retrySearch => '重试搜索';

  @override
  String get contentSearchHint => '描述要找的图片';

  @override
  String get fileSearchHint => '搜索文件名或路径';

  @override
  String get contentSearchEmptyHint => '可以换个描述、调整筛选，或搜索整个图库。';

  @override
  String get filterSearchEmptyHint => '可以调整或清除筛选。';

  @override
  String get searchNeedsModel => '请先在设置中安装图片分析模型。';

  @override
  String get searchDescriptionTooLong => '请输入不超过 512 个字符的描述。';

  @override
  String get searchPartialAnalysis => '部分图片尚未完成分析，已保留可用的搜索结果。请重试分析。';

  @override
  String get searchLibraryChanged => '图库刚有更新，请重试搜索。';

  @override
  String get searchPhotoChanged => '这张图片已变化或无法读取，请重新选择。';

  @override
  String get searchAnalysisUnavailable => '图片分析当前不可用，请重试搜索。';

  @override
  String get searchIndexDamaged => '搜索数据已损坏，请重试以重新建立。';

  @override
  String get searchStorageUnavailable => '暂时无法读写搜索数据，请重试。';

  @override
  String get searchFailedRetained => '暂时无法搜索，已保留原来的图片列表。请重试。';

  @override
  String searchCoverage(int indexed, int total, String status) {
    final intl.NumberFormat indexedNumberFormat =
        intl.NumberFormat.decimalPattern(localeName);
    final String indexedString = indexedNumberFormat.format(indexed);
    final intl.NumberFormat totalNumberFormat =
        intl.NumberFormat.decimalPattern(localeName);
    final String totalString = totalNumberFormat.format(total);

    return '按相关度排列 · 已分析 $indexedString/$totalString 张$status';
  }

  @override
  String get analysisStorageFullSuffix => ' · 分析存储已满';

  @override
  String get partialResults => '部分结果';

  @override
  String get showFilmstrip => '显示缩略图';

  @override
  String get hideFilmstrip => '隐藏缩略图';

  @override
  String get scrollFilmstripLeft => '向左滚动';

  @override
  String get scrollFilmstripRight => '向右滚动';

  @override
  String get fitImageToWindow => '适应窗口（0 / Ctrl+0）';

  @override
  String get showImageActualSize => '实际大小（1 / Ctrl+1）';

  @override
  String get enterViewerFullscreen => '全屏（F11）';

  @override
  String get exitViewerFullscreen => '退出全屏（Esc / F11）';

  @override
  String get clearSearch => '清除搜索';

  @override
  String get library => '图库';

  @override
  String get allImages => '所有图片';

  @override
  String get addFolder => '添加目录到图库';

  @override
  String get noFolder => '还没有添加目录';

  @override
  String get expandFolder => '展开目录';

  @override
  String get collapseFolder => '折叠目录';

  @override
  String get loadingFolders => '正在加载目录…';

  @override
  String get showMoreFolders => '显示更多目录';

  @override
  String get retryFolders => '重新加载目录';

  @override
  String get synchronized => '已同步';

  @override
  String get synchronizing => '正在更新';

  @override
  String get backgroundVerification => '正在核对';

  @override
  String get needsReconciliation => '更新受阻';

  @override
  String get synchronizationUnconfirmed => '尚未核对';

  @override
  String get synchronizationRetryPending => '等待重试';

  @override
  String get continuityLiveOnly => '自动更新不可用';

  @override
  String get continuityUnavailable => '无法自动更新';

  @override
  String get sourceUnavailable => '无法访问';

  @override
  String get sourceAvailable => '可用';

  @override
  String get sourceMissing => '目录不存在';

  @override
  String get sourceOffline => '当前离线';

  @override
  String get sourceUnknown => '状态未知';

  @override
  String get addingSource => '正在添加';

  @override
  String get refreshingSourceDisplay => '正在刷新';

  @override
  String get sourceAdded => '已添加';

  @override
  String get sourceDisplayReloadPending => '需要重新加载';

  @override
  String get sourceScanResultReadFailed => '读取结果失败';

  @override
  String get sourceReadingScanResult => '正在读取结果';

  @override
  String get layout => '布局';

  @override
  String get sort => '排序';

  @override
  String get fileName => '名称';

  @override
  String get newestAddedFirst => '新加入优先';

  @override
  String get ascending => '升序';

  @override
  String get descending => '降序';

  @override
  String get equalHeight => '等高';

  @override
  String get square => '方形';

  @override
  String get sizeMediumLong => '中等';

  @override
  String get more => '更多';

  @override
  String get selectAll => '全选';

  @override
  String get unknownCaptureDate => '拍摄日期未知';

  @override
  String get open => '打开';

  @override
  String get viewInformation => '查看信息';

  @override
  String get copyPath => '复制路径';

  @override
  String get updateLibrary => '更新图库';

  @override
  String get relocateLibrary => '重新定位目录';

  @override
  String get removeFromAme => '从 Ame 中移除';

  @override
  String get updatingLibrary => '正在更新图库…';

  @override
  String get emptyLibraryTitle => '建立你的图片图库';

  @override
  String get emptyLibraryBody => '选择要在 Ame 中一起浏览的图片目录吧。';

  @override
  String get importPhotos => '导入';

  @override
  String get firstImportIncomplete => '尚未完成添加';

  @override
  String get firstImportInProgress => '正在添加图片';

  @override
  String get firstImportInProgressDetail => '添加完成后，图片就会显示在这里。';

  @override
  String get pausingSource => '正在暂停';

  @override
  String get firstImportPaused => '已暂停添加';

  @override
  String get firstImportPausedDetail => '进度已保存，可以稍后继续。';

  @override
  String get cancellingSource => '正在取消';

  @override
  String get firstImportCancelled => '已取消添加';

  @override
  String get firstImportRetryDetail => '这个目录暂时无法添加到图库。';

  @override
  String get firstImportGalleryDetail => '需要时可以重新添加这个目录。';

  @override
  String get scanConfirmationPending => '扫描结果读取失败';

  @override
  String get scanConfirmationDetail => '暂时无法显示扫描到的图片。';

  @override
  String get readingScanResult => '正在读取扫描结果…';

  @override
  String get rereadScanResult => '重新读取';

  @override
  String get galleryDisplayReloadPending => '图片加载失败';

  @override
  String get galleryDisplayReloadDetail => '图片已添加，暂时无法显示。';

  @override
  String get addFolderAwaitingScanResult => '扫描结果读取失败，暂时无法添加目录。';

  @override
  String get addFolderAwaitingGalleryReload => '图库加载失败，暂时无法添加目录。';

  @override
  String get noSearchResults => '没有找到匹配的图片';

  @override
  String get noSearchResultsHint => '请尝试其他名称或路径。';

  @override
  String get noSourceResults => '此目录中没有可显示的图片';

  @override
  String get noSourceResultsHint => '可以选择其他目录，或返回图库。';

  @override
  String get retryLoading => '重新加载';

  @override
  String get retryPreview => '重新加载预览';

  @override
  String get retryingPreview => '正在加载预览…';

  @override
  String get previewFormatUnrecognized => '无法识别图片';

  @override
  String get previewUpdateRequired => '请先更新图库，再重新加载预览。';

  @override
  String get backToLibrary => '返回图库';

  @override
  String get originalImageUnavailable => '无法打开原图';

  @override
  String get originalImagePreviewFallback => '原图无法打开，已暂时显示预览图';

  @override
  String get retryOriginalImage => '重新打开';

  @override
  String get viewerSourceChanged => '图片已更新，重新加载以查看最新内容。';

  @override
  String get viewerSourceMissing => '这张图片已不在图库中。';

  @override
  String removingFromAme(String name) {
    return '正在从 Ame 中移除“$name”…';
  }

  @override
  String get removingFolderDetail => '原目录和图片都会保留。';

  @override
  String get removeFolderFailed => '移除目录失败';

  @override
  String refreshingAfterRemoval(String name) {
    return '已移除“$name”，正在重新加载…';
  }

  @override
  String get refreshingAfterRemovalDetail => '原目录和图片已保留。';

  @override
  String get removedFolderRefreshFailed => '目录已移除，图片加载失败';

  @override
  String get scanOverlapsStorage => '这个文件夹与 Ame 的数据存储位置重叠，请选择其他文件夹。';

  @override
  String get scanBusy => '另一个目录正在更新，结束后再继续。';

  @override
  String get scanCancelFailed => '取消失败，请重试。';

  @override
  String get parentScanIncomplete =>
      '上级目录没有扫描完整。请检查目录能否打开，以及扫描数量是否达到上限。已添加的子目录仍会保留。';

  @override
  String get childScanPending => '子目录还有未完成的扫描。请先继续或取消该任务，再添加父目录。';

  @override
  String get childUpdateRequired => '先更新已添加的子目录，再添加上级目录。';

  @override
  String get childMergeFailed => '无法合并已添加的子目录。请检查目录位置后重试。';

  @override
  String get scanFolderFailed => '无法更新这个目录，请检查它能否打开。';

  @override
  String get synchronizationRefreshFailureMessage => '重新加载以查看最新图片。';

  @override
  String get manualUpdateNotice => '部分目录需要手动更新';

  @override
  String get manualUpdateDetail => '重新打开 Ame 后，在目录菜单中选择“更新图库”，补上关闭期间的变化。';

  @override
  String synchronizationBlockedFolder(String name) {
    return '“$name”更新受阻';
  }

  @override
  String synchronizationUncheckedFolder(String name) {
    return '“$name”图库尚未核对';
  }

  @override
  String firstImportRequiredFolder(String name) {
    return '“$name”尚未完成添加';
  }

  @override
  String get synchronizationSourceUnavailable => '打不开这个目录。';

  @override
  String get synchronizationSourceUnhealthy => '自动更新已中断。';

  @override
  String get synchronizationSourceStarting => '正在准备自动更新。';

  @override
  String get synchronizationEvidenceGap => '部分文件变化还没有更新到图库。';

  @override
  String get synchronizationCapacityExceeded => '文件变化较多，图库还未更新完成。';

  @override
  String get synchronizationNeedsReconciliation => '图库还没有完成更新。';

  @override
  String get synchronizationCompleted => '目录中的变化已更新到图库。';

  @override
  String get synchronizationAutomaticRetry => '部分内容还未更新，稍后会自动重试。';

  @override
  String get synchronizationRecoveryBlocked => '在目录菜单点击“更新图库”重试。';

  @override
  String get synchronizationLegacyRecoveryAuthorityMissing =>
      '在目录菜单点击“更新图库”，恢复自动更新。';

  @override
  String get synchronizationMonitoringAccessDenied => '没有权限读取这个目录的变化，请检查访问权限。';

  @override
  String get synchronizationMonitoringEventIncomplete => '无法读取部分文件的变化。';

  @override
  String get synchronizationRecoveryFailed => '目录检查未完成。';

  @override
  String get synchronizationPersistenceFailed => '图库更新未能保存。';

  @override
  String get synchronizationPreparing => '准备更新';

  @override
  String get synchronizationCheckingFolder => '正在检查目录';

  @override
  String get synchronizationCheckingChanges => '正在检查变化';

  @override
  String get synchronizationUpdatingLibrary => '正在更新图库';

  @override
  String get synchronizationRetryLater => '稍后自动重试';

  @override
  String get synchronizationScanningFolder => '正在扫描目录';

  @override
  String get synchronizationUpdateIncomplete => '更新未完成';

  @override
  String get synchronizationComplete => '同步完成';

  @override
  String get synchronizationWaitingFolder => '等待目录恢复';

  @override
  String get libraryUpdateBlocked => '图库更新受阻';

  @override
  String pendingChanges(String count) {
    return '$count 项等待处理';
  }

  @override
  String retryingChanges(String count) {
    return '$count 项等待重试';
  }

  @override
  String incompleteChanges(String count) {
    return '$count 项更新尚未完成';
  }

  @override
  String get changesToCheck => '还有文件变化待检查';

  @override
  String get waitingForUpdate => '等待更新';

  @override
  String get checkingPhotos => '正在检查图片';

  @override
  String get loadingPhotosShort => '正在加载图片';

  @override
  String get cancelled => '已取消';

  @override
  String get filesChanged => '文件已变化';

  @override
  String get scanCompleted => '扫描已完成。';

  @override
  String reloadRoot(String name) {
    return '重新加载 $name';
  }

  @override
  String updateRoot(String name) {
    return '更新图库 $name';
  }

  @override
  String retryRoot(String name) {
    return '重试更新 $name';
  }

  @override
  String progressPhotos(String phase, int checked, int total, int percent) {
    final intl.NumberFormat checkedNumberFormat =
        intl.NumberFormat.decimalPattern(localeName);
    final String checkedString = checkedNumberFormat.format(checked);
    final intl.NumberFormat totalNumberFormat =
        intl.NumberFormat.decimalPattern(localeName);
    final String totalString = totalNumberFormat.format(total);

    return '$phase，$checkedString / $totalString 张图片，$percent%';
  }

  @override
  String progressFiles(String phase, int files, int photos) {
    final intl.NumberFormat filesNumberFormat =
        intl.NumberFormat.decimalPattern(localeName);
    final String filesString = filesNumberFormat.format(files);
    final intl.NumberFormat photosNumberFormat =
        intl.NumberFormat.decimalPattern(localeName);
    final String photosString = photosNumberFormat.format(photos);

    return '$phase，已检查 $filesString 个文件，已找到 $photosString 张图片';
  }

  @override
  String get duplicates => '去重';

  @override
  String get collapseSidebar => '收起侧栏';

  @override
  String get expandSidebar => '展开侧栏';

  @override
  String get saveSidebarFailed => '无法保存侧栏设置';

  @override
  String get changePhotoFailed => '无法切换图片';

  @override
  String removedFolders(int count) {
    final intl.NumberFormat countNumberFormat =
        intl.NumberFormat.decimalPattern(localeName);
    final String countString = countNumberFormat.format(count);

    return '已从 Ame 中移除 $countString 个文件夹';
  }

  @override
  String get saveGallerySettingsFailed => '无法保存图库显示设置';

  @override
  String get filePathCopied => '已复制文件路径';

  @override
  String get copyPathFailed => '无法复制路径';

  @override
  String get confirmRemoveFolder => '从 Ame 中移除目录？';

  @override
  String confirmRemoveFolderDetail(String name) {
    return '“$name”中的图片将不再显示在 Ame 中。原目录和图片都会保留。';
  }

  @override
  String removedFolder(String name) {
    return '已从 Ame 中移除“$name”';
  }

  @override
  String get seekDateFailed => '无法跳转到所选日期';

  @override
  String get scrollUp => '向上滚动';

  @override
  String scrollPosition(int percent) {
    return '滚动位置 $percent%';
  }

  @override
  String get scrollDown => '向下滚动';

  @override
  String get createAlbum => '新建相册';

  @override
  String get renameAlbum => '重命名相册';

  @override
  String changeAlbumIcon(String name) {
    return '更改 $name 的图标';
  }

  @override
  String get confirmDeleteAlbum => '删除相册？';

  @override
  String get save => '保存';

  @override
  String get saving => '正在保存';

  @override
  String get deleteAlbumImpact => '其中的图片组将进入已忽略。图片文件和收藏状态会保留。';

  @override
  String get albumName => '相册名称';

  @override
  String get chooseAlbumIcon => '选择相册图标';

  @override
  String get albumIconTravel => '旅行';

  @override
  String get albumIconNature => '自然';

  @override
  String get albumIconPets => '宠物';

  @override
  String get albumIconPeople => '人物';

  @override
  String get albumIconFood => '美食';

  @override
  String get albumIconArt => '艺术';

  @override
  String get albumIconWork => '工作';

  @override
  String get albumIconStudy => '学习';

  @override
  String get albumIconGames => '游戏';

  @override
  String get albumIconMusic => '音乐';

  @override
  String get albumIconMemories => '纪念';

  @override
  String confirmDeleteAlbums(int count) {
    final intl.NumberFormat countNumberFormat =
        intl.NumberFormat.decimalPattern(localeName);
    final String countString = countNumberFormat.format(count);

    return '删除 $countString 个相册？';
  }

  @override
  String get deleteAlbumsImpact => '这些相册中的图片组将进入已忽略。图片文件和收藏状态会保留。';

  @override
  String get deleteAlbum => '删除相册';

  @override
  String get selectPageAlbums => '全选本页相册';

  @override
  String get selectAllAlbums => '全选相册';

  @override
  String get deleteSelectedAlbums => '删除所选相册';

  @override
  String get albumsEmpty => '还没有创建相册';

  @override
  String get previousPage => '上一页';

  @override
  String get nextPage => '下一页';

  @override
  String get albumsUnavailable => '相册暂时无法加载';

  @override
  String get changeIcon => '更改图标';

  @override
  String get chooseAlbum => '选择相册';

  @override
  String get updatingAlbums => '正在更新相册';

  @override
  String get createFirstAlbum => '还没有相册，请先新建一个。';

  @override
  String get loadingAlbums => '正在读取相册';

  @override
  String get albumsLoadFailed => '相册暂时无法加载。';

  @override
  String get removeFromAlbum => '移出相册';

  @override
  String get albumGroupConflict => '这组图片的相册归属有冲突，请选择一个相册。';

  @override
  String get albumDetailsUnavailable => '相册信息暂时无法读取，点击重试。';

  @override
  String get albumsSidebarHint => '在侧栏中选择收藏夹或相册，也可以新建相册。';

  @override
  String get file => '文件';

  @override
  String get path => '路径';

  @override
  String get fileType => '类型';

  @override
  String get imageDimensions => '尺寸';

  @override
  String get size => '大小';

  @override
  String get cameraMake => '相机品牌';

  @override
  String get cameraModel => '相机型号';

  @override
  String get timeTaken => '拍摄时间';

  @override
  String get timeSource => '时间来源';

  @override
  String get timeCreated => '创建时间';

  @override
  String get timeModified => '修改时间';

  @override
  String get cameraOriginalTime => '相机原始拍摄时间';

  @override
  String get digitizedTime => '数字化时间';

  @override
  String get photoRecordedTime => '图片记录时间';

  @override
  String get loadingAlbumAssignments => '正在读取相册归属';

  @override
  String get conflictAlbumsUnavailable => '无法读取冲突涉及的相册。';

  @override
  String get albumAssignmentsUpdated => '相册归属已更新。';

  @override
  String get conflictAlbums => '冲突涉及的相册';

  @override
  String deletedAlbumName(String name) {
    return '$name（已删除）';
  }

  @override
  String get similarKind => '相似';

  @override
  String get duplicateKind => '重复';

  @override
  String deletePhotoKind(String kind) {
    return '删除$kind图片';
  }

  @override
  String get saveSelectionFailed => '未能保存选择';

  @override
  String get tryAgain => '请重试。';

  @override
  String similarPhotoCount(int count) {
    final intl.NumberFormat countNumberFormat =
        intl.NumberFormat.decimalPattern(localeName);
    final String countString = countNumberFormat.format(count);

    return '$countString 张相似图片';
  }

  @override
  String get updatingFileLocations => '正在更新文件位置';

  @override
  String get deleteSimilarPhotos => '删除相似图片';

  @override
  String get deleteDuplicatePhotos => '删除重复图片';

  @override
  String get deleteSelected => '删除所选';

  @override
  String get loadingFileLocations => '正在读取文件位置';

  @override
  String get checkingPhotoContent => '正在确认图片内容';

  @override
  String get favoritesUnavailable => '暂时无法读取收藏状态';

  @override
  String get savingChanges => '正在保存更改';

  @override
  String get loadingFavoriteStatus => '正在读取收藏状态';

  @override
  String get removeFavorite => '取消收藏';

  @override
  String get addFavorite => '加入收藏夹';

  @override
  String get readingPhotos => '正在读取图片';

  @override
  String photoOperation(String action) {
    return '$action图片';
  }

  @override
  String fileCount(int count) {
    final intl.NumberFormat countNumberFormat =
        intl.NumberFormat.decimalPattern(localeName);
    final String countString = countNumberFormat.format(count);

    return '$countString 个文件';
  }

  @override
  String get destinationFolder => '目标文件夹';

  @override
  String get change => '更改';

  @override
  String get folderStructure => '文件夹结构';

  @override
  String get fileNameLabel => '文件名';

  @override
  String get permanentDeleteWarning => '文件将被永久删除，无法从回收站恢复。';

  @override
  String get recycleWarning => '文件将移入 Windows 回收站。';

  @override
  String get deletePermanently => '永久删除';

  @override
  String get back => '返回';

  @override
  String get operationReport => '结果清单';

  @override
  String get affectedFiles => '涉及文件';

  @override
  String selectedCountShort(int count) {
    final intl.NumberFormat countNumberFormat =
        intl.NumberFormat.decimalPattern(localeName);
    final String countString = countNumberFormat.format(count);

    return '已选择 $countString 个';
  }

  @override
  String get reportFile => '清单文件';

  @override
  String get reportHistoryUnavailable => '暂时无法读取结果清单记录，请重试。';

  @override
  String get loadingReport => '正在读取结果清单';

  @override
  String get noExportHistory => '暂无导出记录';

  @override
  String get reportPending => '清单尚未生成';

  @override
  String get noReportNeeded => '没有特殊结果，无需清单';

  @override
  String get reportWriteFailedStatus => '清单写入失败，文件操作结果已保留';

  @override
  String get createReport => '生成清单';

  @override
  String get regenerate => '重新生成';

  @override
  String get updatingReport => '正在更新结果清单';

  @override
  String get latestRecords => '最新记录';

  @override
  String get earlierRecords => '更早记录';

  @override
  String get stop => '停止';

  @override
  String get viewHistory => '查看记录';

  @override
  String get noExceptionsReport => '没有特殊结果，无需生成清单。';

  @override
  String get loadingOperationHistory => '正在读取操作记录';

  @override
  String verifiedFiles(int checked, int total) {
    final intl.NumberFormat checkedNumberFormat =
        intl.NumberFormat.decimalPattern(localeName);
    final String checkedString = checkedNumberFormat.format(checked);
    final intl.NumberFormat totalNumberFormat =
        intl.NumberFormat.decimalPattern(localeName);
    final String totalString = totalNumberFormat.format(total);

    return '已核对 $checkedString / $totalString 个文件';
  }

  @override
  String get reportSavedDestination => '结果清单已保存到目标文件夹。';

  @override
  String processedFiles(int processed, int total) {
    final intl.NumberFormat processedNumberFormat =
        intl.NumberFormat.decimalPattern(localeName);
    final String processedString = processedNumberFormat.format(processed);
    final intl.NumberFormat totalNumberFormat =
        intl.NumberFormat.decimalPattern(localeName);
    final String totalString = totalNumberFormat.format(total);

    return '已处理 $processedString / $totalString 个文件';
  }

  @override
  String get fileHistoryUnavailable => '暂时无法读取文件操作记录，请重试。';

  @override
  String get confirmContinueMove => '继续移动这个文件？';

  @override
  String get continueMove => '继续移动';

  @override
  String get fileOperationHistory => '文件操作记录';

  @override
  String get updatingFileRecords => '正在更新文件记录';

  @override
  String get fileRecords => '文件记录';

  @override
  String get loadingFileRecords => '正在读取文件记录';

  @override
  String get noFileOperations => '暂无文件操作记录';

  @override
  String get fileOperationDetails => '文件操作详情';

  @override
  String get checkResults => '核对结果';

  @override
  String get recycleBinOpenFailed => '无法打开回收站，请从 Windows 桌面打开。';

  @override
  String get openRecycleBin => '打开回收站';

  @override
  String get duplicatesOnly => '仅显示重复';

  @override
  String get similarOnly => '仅显示相似';

  @override
  String get mergeDuplicates => '合并重复';

  @override
  String removeFilter(String label) {
    return '移除$label';
  }

  @override
  String selectedItems(int count) {
    final intl.NumberFormat countNumberFormat =
        intl.NumberFormat.decimalPattern(localeName);
    final String countString = countNumberFormat.format(count);

    return '已选择 $countString 个项目';
  }

  @override
  String duplicateGroupCount(int count) {
    final intl.NumberFormat countNumberFormat =
        intl.NumberFormat.decimalPattern(localeName);
    final String countString = countNumberFormat.format(count);

    return '$countString 个重复组';
  }

  @override
  String similarGroupCount(int count) {
    final intl.NumberFormat countNumberFormat =
        intl.NumberFormat.decimalPattern(localeName);
    final String countString = countNumberFormat.format(count);

    return '$countString 个相似组';
  }

  @override
  String deduplicatedCount(String summary, int count) {
    final intl.NumberFormat countNumberFormat =
        intl.NumberFormat.decimalPattern(localeName);
    final String countString = countNumberFormat.format(count);

    return '$summary · 去重后 $countString 张';
  }

  @override
  String organizationCounts(
    int total,
    int organized,
    int pending,
    int ignored,
  ) {
    final intl.NumberFormat totalNumberFormat =
        intl.NumberFormat.decimalPattern(localeName);
    final String totalString = totalNumberFormat.format(total);
    final intl.NumberFormat organizedNumberFormat =
        intl.NumberFormat.decimalPattern(localeName);
    final String organizedString = organizedNumberFormat.format(organized);
    final intl.NumberFormat pendingNumberFormat =
        intl.NumberFormat.decimalPattern(localeName);
    final String pendingString = pendingNumberFormat.format(pending);
    final intl.NumberFormat ignoredNumberFormat =
        intl.NumberFormat.decimalPattern(localeName);
    final String ignoredString = ignoredNumberFormat.format(ignored);

    return '$totalString 张图片 · 已整理 $organizedString · 待整理 $pendingString · 已忽略 $ignoredString';
  }

  @override
  String get organizationMode => '整理模式';

  @override
  String get exitOrganization => '退出整理';

  @override
  String get noPendingPhotos => '没有待整理图片';

  @override
  String get noDeferredPhotos => '没有稍后整理的图片';

  @override
  String get noIgnoredPhotos => '没有已忽略的图片';

  @override
  String get organizationEmptyHint => '可以切换整理队列，或在更多菜单中退出整理模式。';

  @override
  String get noCategoryPhotos => '还没有这个分类的图片';

  @override
  String get categoryEmptyHint => '可以切换分类或返回图库。';

  @override
  String get noAlbumPhotos => '这个相册里还没有图片';

  @override
  String get albumEmptyHint => '在图库中打开图片后，可以将它加入相册。';

  @override
  String get noFavoritePhotos => '收藏夹里还没有图片';

  @override
  String get favoritesEmptyHint => '打开图片后，可以将它加入收藏夹。';

  @override
  String get filmstripSettingFailed => '无法保存缩略图设置，请重试';

  @override
  String photoPosition(int position, int total) {
    final intl.NumberFormat totalNumberFormat =
        intl.NumberFormat.decimalPattern(localeName);
    final String totalString = totalNumberFormat.format(total);

    return '$position / $totalString 张';
  }

  @override
  String clearFilter(String label) {
    return '清除$label';
  }

  @override
  String get hide => '隐藏';

  @override
  String get downloadSmartModel => '下载智能相册模型';

  @override
  String get smartModelDownloadDetail =>
      '需要下载约 504 MB 的模型，之后在本机分析图片。已有的完整模型文件会复用。';

  @override
  String get downloadAndEnable => '下载并启用';

  @override
  String get folders => '文件夹';

  @override
  String get removeSelectedFolders => '从 Ame 中移除所选文件夹';

  @override
  String get resizeSidebar => '调整侧栏宽度';

  @override
  String pixelCount(int count) {
    final intl.NumberFormat countNumberFormat =
        intl.NumberFormat.decimalPattern(localeName);
    final String countString = countNumberFormat.format(count);

    return '$countString 像素';
  }

  @override
  String get undoAction => '撤销操作';

  @override
  String get redoAction => '重做操作';

  @override
  String get historyLoadRetry => '操作记录未能加载，点击重试';

  @override
  String get organizationOptions => '整理选项';

  @override
  String get albumsAndFavoritesUnavailable => '暂时无法读取相册和收藏';

  @override
  String get reopenMenuRetry => '请重新打开菜单重试。';

  @override
  String get copyToFolder => '复制到文件夹';

  @override
  String get moveToFolder => '移动到文件夹';

  @override
  String get photoListFailed => '图片列表加载失败';

  @override
  String get continueOrganizing => '继续整理';

  @override
  String get toOrganize => '待整理';

  @override
  String photosRelatedTo(String name) {
    return '与 $name 相关的图片';
  }

  @override
  String get selectedAlbumConflict => '所选图片中有相册归属冲突，请选择一个相册。';

  @override
  String get actionFailed => '无法完成操作';

  @override
  String get favorite => '收藏';

  @override
  String get preparing => '正在准备';

  @override
  String get processing => '正在处理';

  @override
  String itemsCompleted(int completed, int total) {
    final intl.NumberFormat totalNumberFormat =
        intl.NumberFormat.decimalPattern(localeName);
    final String totalString = totalNumberFormat.format(total);

    return '已完成 $completed / $totalString 项。';
  }

  @override
  String get sortOrderSaveFailed => '排列顺序未能保存，请重新拖动。';

  @override
  String confirmRemoveFolders(int count) {
    final intl.NumberFormat countNumberFormat =
        intl.NumberFormat.decimalPattern(localeName);
    final String countString = countNumberFormat.format(count);

    return '从 Ame 中移除 $countString 个文件夹？';
  }

  @override
  String get removeFoldersImpact =>
      '这些文件夹中的图片将不再显示在 Ame 中，仅属于这些文件夹的相册归属、收藏和整理记录也会移除。其他来源仍有的重复图片会保留记录。原目录和图片文件都会保留。';

  @override
  String selectedCount(int count) {
    final intl.NumberFormat countNumberFormat =
        intl.NumberFormat.decimalPattern(localeName);
    final String countString = countNumberFormat.format(count);

    return '已选择 $countString';
  }

  @override
  String get selectedLong => '已选择';

  @override
  String get selectedShort => '已选';

  @override
  String get deselectAll => '取消全选';

  @override
  String selectAllNamed(String label) {
    return '全选$label';
  }

  @override
  String collapseSection(String section) {
    return '收起$section';
  }

  @override
  String expandSection(String section) {
    return '展开$section';
  }

  @override
  String deselectNamed(String name) {
    return '取消选择$name';
  }

  @override
  String selectNamed(String name) {
    return '选择$name';
  }

  @override
  String get noClassificationResults => '还没有分类结果';

  @override
  String get downloadModel => '下载模型';

  @override
  String get loadingSmartAlbums => '正在加载智能相册';

  @override
  String get categoriesUnavailable => '分类暂时无法加载';

  @override
  String recentlyAddedCount(int count) {
    final intl.NumberFormat countNumberFormat =
        intl.NumberFormat.decimalPattern(localeName);
    final String countString = countNumberFormat.format(count);

    return '新加入 · $countString 张';
  }

  @override
  String previouslyAddedCount(int count) {
    final intl.NumberFormat countNumberFormat =
        intl.NumberFormat.decimalPattern(localeName);
    final String countString = countNumberFormat.format(count);

    return '此前加入 · $countString 张';
  }

  @override
  String get recentlyAdded => '新加入';

  @override
  String get previouslyAdded => '此前';

  @override
  String get pause => '暂停';

  @override
  String get resume => '继续';

  @override
  String get libraryUpdates => '图库更新';

  @override
  String get clearFinishedTasks => '清除已结束任务';

  @override
  String cancelRootUpdate(String name) {
    return '取消更新 $name';
  }

  @override
  String closeRootUpdate(String name) {
    return '关闭 $name 更新任务';
  }

  @override
  String rootUpdateProgress(String name) {
    return '$name 更新进度';
  }

  @override
  String categoryLabel(String name) {
    return '分类：$name';
  }

  @override
  String get previousPhotoShortcut => '上一张（←）';

  @override
  String get nextPhotoShortcut => '下一张（→）';

  @override
  String get photoCopied => '图片已复制到剪贴板';

  @override
  String get photoTooLargeToCopy => '图片过大，无法复制到剪贴板';

  @override
  String get copyPhotoFailed => '无法复制图片，请重试';

  @override
  String get reloadThumbnails => '重新加载缩略图';

  @override
  String get toggleFullscreenFailed => '无法切换全屏，请重试';

  @override
  String get readingPhotoStatus => '正在读取图片状态';

  @override
  String get rereadPhotoStatus => '重新读取图片状态';

  @override
  String get copyPhoto => '复制图片';

  @override
  String get rotateClockwise => '顺时针旋转 90°';

  @override
  String get zoomOutShortcut => '缩小（- / Ctrl+-）';

  @override
  String get zoomInShortcut => '放大（+ / Ctrl++）';

  @override
  String get archiveFileType => 'Ame 整理记录';

  @override
  String get exportAction => '导出';

  @override
  String get chooseBackup => '选择备份';

  @override
  String get inspect => '检查';

  @override
  String get restore => '恢复';

  @override
  String get preparingBackup => '正在准备备份';

  @override
  String get exportingArchive => '正在导出整理记录';

  @override
  String get checkingBackup => '正在检查备份';

  @override
  String get linkingPhotos => '正在关联图库中的图片';

  @override
  String get restoringArchive => '正在恢复整理记录';

  @override
  String archiveInProgress(String operation) {
    return '正在$operation整理记录';
  }

  @override
  String get archiveOutsideLibrary => '请选择图库文件夹和 Ame 数据文件夹以外的位置。';

  @override
  String get archiveFileExists => '这个文件已存在，请使用其他文件名。';

  @override
  String get archiveVersionUnsupported => '暂不支持这份备份的版本，请选择兼容的备份。';

  @override
  String get archiveInvalid => '这份备份不完整或内容有误，请重新选择。';

  @override
  String get archiveLibraryChanged => '图库或整理记录已更新，请重新检查备份后确认恢复。';

  @override
  String get archiveChanged => '所选备份已改变，请重新检查后确认恢复。';

  @override
  String get archiveAlreadyRunning => '上一项备份操作尚未结束，请稍后重试。';

  @override
  String get archiveUnreadable => '无法读取这份备份，请确认文件在本机可用。';

  @override
  String get archiveIoFailed => '无法读写备份文件，请检查所选文件和保存位置后重试。';

  @override
  String get archiveConflictsWithCurrent => '这份备份与当前记录有冲突，当前记录已保留。请选择其他备份。';

  @override
  String get archiveResultUnknown => '尚未确认操作结果，请重试。';

  @override
  String get settingsSaveRetry => '设置未能保存，请重试。';

  @override
  String get unreadable => '无法读取';

  @override
  String get askBeforeAdding => '加入相册前询问';

  @override
  String get askBeforeAddingHelp => '每次加入前选择目标相册';

  @override
  String get defaultAlbum => '默认加入的相册';

  @override
  String get defaultAlbumHelp => '未设置时，会在加入前询问';

  @override
  String get gpuSettingFailed => 'GPU 加速设置未能完成';

  @override
  String get gpuSettingRetry => '请重试；如果仍无法分析图片，可以关闭 GPU 加速。';

  @override
  String get gpuAcceleration => 'GPU 加速';

  @override
  String get loadingSettings => '正在读取设置';

  @override
  String get settingsReadFailed => '无法读取设置，请重试';

  @override
  String get switchingAnalysisDevice => '正在切换分析设备';

  @override
  String get smartAlbumsCpu => '智能相册当前使用 CPU 分析图片';

  @override
  String smartAlbumsDevice(String name) {
    return '智能相册当前图片分析设备：$name';
  }

  @override
  String get smartAlbumsPreferGpu => '智能相册优先使用可用显卡分析图片';

  @override
  String get restoreOrganizationRecords => '恢复整理记录';

  @override
  String createdAt(String timestamp) {
    return '创建时间：$timestamp';
  }

  @override
  String albumCount(int count) {
    final intl.NumberFormat countNumberFormat =
        intl.NumberFormat.decimalPattern(localeName);
    final String countString = countNumberFormat.format(count);

    return '$countString 个相册';
  }

  @override
  String photoGroupCount(int count) {
    final intl.NumberFormat countNumberFormat =
        intl.NumberFormat.decimalPattern(localeName);
    final String countString = countNumberFormat.format(count);

    return '$countString 组图片';
  }

  @override
  String favoriteGroupCount(int count) {
    final intl.NumberFormat countNumberFormat =
        intl.NumberFormat.decimalPattern(localeName);
    final String countString = countNumberFormat.format(count);

    return '$countString 组收藏';
  }

  @override
  String duplicateChoiceCount(int count) {
    final intl.NumberFormat countNumberFormat =
        intl.NumberFormat.decimalPattern(localeName);
    final String countString = countNumberFormat.format(count);

    return '$countString 项去重选择';
  }

  @override
  String matchedGroups(int count) {
    final intl.NumberFormat countNumberFormat =
        intl.NumberFormat.decimalPattern(localeName);
    final String countString = countNumberFormat.format(count);

    return '可关联 $countString 组';
  }

  @override
  String unmatchedGroups(int count) {
    final intl.NumberFormat countNumberFormat =
        intl.NumberFormat.decimalPattern(localeName);
    final String countString = countNumberFormat.format(count);

    return '暂未关联 $countString 组';
  }

  @override
  String albumConflictCount(int count) {
    final intl.NumberFormat countNumberFormat =
        intl.NumberFormat.decimalPattern(localeName);
    final String countString = countNumberFormat.format(count);

    return '$countString 组相册冲突';
  }

  @override
  String unresolvedCopies(int count) {
    final intl.NumberFormat countNumberFormat =
        intl.NumberFormat.decimalPattern(localeName);
    final String countString = countNumberFormat.format(count);

    return '$countString 项首选副本暂无法关联';
  }

  @override
  String get includesOrganizationPosition => '包含继续整理的位置';

  @override
  String get restoreKeepsChoices => '当前选择和整理位置会保留，不会移动或修改原图片。';

  @override
  String get unmatchedAwaitPhotos => '暂未关联的记录会等待图片重新关联。';

  @override
  String get filePickerFailed => '无法打开文件选择窗口';

  @override
  String get organizationRecords => '整理记录';

  @override
  String get exportOrganizationRecords => '导出整理记录';

  @override
  String get exportArchiveHelp => '备份相册、收藏和整理进度，不包含原图片';

  @override
  String get restoreArchiveHelp => '从备份恢复，保留当前已作出的选择';

  @override
  String get viewBackup => '查看备份';

  @override
  String get chooseFile => '选择文件';

  @override
  String get noUnlinkedRecords => '没有符合条件的未关联记录';

  @override
  String get unlinkedLessThanDay => '未关联不足 1 天';

  @override
  String unlinkedDays(int days) {
    return '已未关联 $days 天';
  }

  @override
  String unlinkedRange(String shortest, int longest) {
    return '未关联 $shortest～$longest 天';
  }

  @override
  String get lessThanOne => '不足 1';

  @override
  String get clearUnlinkedRecords => '清理未关联记录';

  @override
  String get checkingRecords => '正在检查记录';

  @override
  String get cleanupScope => '清理范围';

  @override
  String get unlinkedNinetyDays => '未关联超过 90 天';

  @override
  String get unlinkedThirtyDays => '未关联超过 30 天';

  @override
  String get allUnlinkedRecords => '全部未关联记录';

  @override
  String get checkFailedRetry => '未能完成检查，请重试。';

  @override
  String get checkCancelled => '检查已取消';

  @override
  String get clearRecords => '清理记录';

  @override
  String cleanupCounts(int groups, int choices) {
    final intl.NumberFormat groupsNumberFormat =
        intl.NumberFormat.decimalPattern(localeName);
    final String groupsString = groupsNumberFormat.format(groups);
    final intl.NumberFormat choicesNumberFormat =
        intl.NumberFormat.decimalPattern(localeName);
    final String choicesString = choicesNumberFormat.format(choices);

    return '$groupsString 组图片的记录 · $choicesString 项去重选择';
  }

  @override
  String cleanupAlbumGroups(int count) {
    final intl.NumberFormat countNumberFormat =
        intl.NumberFormat.decimalPattern(localeName);
    final String countString = countNumberFormat.format(count);

    return '相册归属 $countString 组';
  }

  @override
  String cleanupFavoriteGroups(int count) {
    final intl.NumberFormat countNumberFormat =
        intl.NumberFormat.decimalPattern(localeName);
    final String countString = countNumberFormat.format(count);

    return '收藏 $countString 组';
  }

  @override
  String cleanupProgressGroups(int count) {
    final intl.NumberFormat countNumberFormat =
        intl.NumberFormat.decimalPattern(localeName);
    final String countString = countNumberFormat.format(count);

    return '整理进度 $countString 组';
  }

  @override
  String cleanupClassificationGroups(int count) {
    final intl.NumberFormat countNumberFormat =
        intl.NumberFormat.decimalPattern(localeName);
    final String countString = countNumberFormat.format(count);

    return '分类记录 $countString 组';
  }

  @override
  String get cleanupRecordImpact => '相册和原图片会保留。重新导入图片不会恢复这些记录。';

  @override
  String get cleanupUndoWarning => '本次清理无法撤销。如需恢复记录，可以手动恢复此前导出的备份。';

  @override
  String get cleanupRecordsHelp => '检查无法关联图片的相册、收藏和整理记录';

  @override
  String get cleanUp => '清理';

  @override
  String get stopNotConfirmed => '尚未停止，请再次尝试。';

  @override
  String get clearingRecords => '正在清理记录';

  @override
  String get checkAgain => '重新检查';

  @override
  String get recordsChanged => '记录已发生变化，请重新检查后确认。';

  @override
  String get cleanupResultUnknown => '未能确认清理结果，请重试。';

  @override
  String get confirmClearThumbnails => '清理缩略图？';

  @override
  String get confirmClearOldThumbnails => '清理旧缩略图目录？';

  @override
  String get oldThumbnailFolder => '旧缩略图文件夹';

  @override
  String get clearThumbnailsImpact =>
      '这会删除可重新生成的缩略图缓存，不会删除或修改原图片。清理后，打开图库时可能需要一些时间重新生成缩略图。';

  @override
  String get clearOldThumbnailsImpact => '只会删除这个文件夹中由 Ame 管理的缩略图。原图片和其他文件会保留。';

  @override
  String get startCleanup => '开始清理';

  @override
  String get cleanupAlreadyFinished => '清理任务已经结束，无法再取消';

  @override
  String get storage => '存储';

  @override
  String get loadingStorageSettings => '正在读取存储设置';

  @override
  String get checkingStoragePaths => '正在检查图库数据与缩略图的保存位置';

  @override
  String get storageSettingsUnavailable => '无法读取存储设置';

  @override
  String get storageRestartRequired => '重启 Ame 后应用新的存储设置';

  @override
  String get existingFilesKept => '现有文件不会被移动或删除';

  @override
  String get catalogStorageLocation => '图库数据位置';

  @override
  String catalogStorageHelp(String path) {
    return '保存图库索引和扫描结果\n$path';
  }

  @override
  String storageUsageWithDetail(String size, String detail) {
    return '当前占用 $size$detail';
  }

  @override
  String storageUsageDetail(String live, String reclaimable) {
    return '，有效数据约 $live，可回收 $reclaimable';
  }

  @override
  String get catalogLocationLocked => '已有图库时不可更改';

  @override
  String get starting => '正在启动';

  @override
  String get cleanupNotNeeded => '无需清理';

  @override
  String get thumbnailLocation => '缩略图位置';

  @override
  String thumbnailStorageHelp(String path) {
    return '保存可随时重新生成的图片预览\n$path';
  }

  @override
  String get retiredThumbnailDirectory => '旧缩略图目录';

  @override
  String retiredThumbnailsHelp(String path) {
    return '新目录已启用；旧目录只会在确认后清理 Ame 管理的缩略图\n$path';
  }

  @override
  String get cleaning => '正在清理';

  @override
  String get cleanOldFolder => '清理旧目录';

  @override
  String get thumbnailBudget => '缩略图最大占用空间';

  @override
  String get thumbnailBudgetHelp => '达到上限时自动回收当前不需要的缩略图，不会修改原图片';

  @override
  String storageUsage(String size) {
    return '当前占用 $size';
  }

  @override
  String get storageSaveFailed => '未能保存存储设置';

  @override
  String get storageOverlapsSource => '存储位置与已添加的图片文件夹重叠。请选择单独的文件夹。';

  @override
  String get storageErrorDetails => '查看错误详情';

  @override
  String get storageHideErrorDetails => '收起错误详情';

  @override
  String get catalogMaintenanceStatusError => '图库数据整理状态异常';

  @override
  String get thumbnailCleanupStatusError => '缩略图清理状态异常';

  @override
  String get storageLocationPending => '完成前不会改变当前正在使用的存储位置';

  @override
  String get optimizeCatalog => '整理图库数据';

  @override
  String get catalogWaiting => '图库数据等待整理';

  @override
  String get catalogInspecting => '正在检查图库数据';

  @override
  String get catalogWaitingIdle => '等待空闲后整理图库数据';

  @override
  String get catalogCheckingSpace => '正在检查整理空间';

  @override
  String get catalogOptimizing => '正在优化图库数据库';

  @override
  String get catalogReclaiming => '正在释放图库数据空间';

  @override
  String get catalogOptimized => '图库数据整理完成';

  @override
  String get catalogOptimizationCancelled => '图库数据整理已取消';

  @override
  String get catalogOptimizationFailed => '图库数据整理失败';

  @override
  String reclaimableSpace(String size) {
    return '约 $size 空间可以安全回收';
  }

  @override
  String get catalogAutoReclaim => '删除图库后会在后台回收数据库空闲页，不会删除原图片';

  @override
  String get catalogRemovalApplied => '移除结果已经生效，后台整理即将开始';

  @override
  String get catalogEstimating => '正在计算有效数据与可回收空间';

  @override
  String get catalogBusyWait => '有更重要的图库操作正在进行，稍后自动继续';

  @override
  String get catalogCheckingTempSpace => '正在确认数据库重建所需的临时磁盘空间';

  @override
  String catalogConverting(String size) {
    return '正在执行一次性数据库转换，可回收约 $size；此阶段无法可靠估算百分比';
  }

  @override
  String catalogReleasedRemaining(String released, String remaining) {
    return '已释放 $released，剩余约 $remaining';
  }

  @override
  String catalogReleased(String released) {
    return '已释放 $released；有效图库数据保持不变';
  }

  @override
  String get catalogStopped => '已停止整理；图库移除结果与有效数据不受影响，可稍后重试';

  @override
  String get catalogRetryLater => '未能完成图库数据整理，可稍后重试';

  @override
  String spaceRequired(String requiredSize, String available) {
    return '需要 $requiredSize，当前可用 $available';
  }

  @override
  String get thumbnails => '缩略图';

  @override
  String cleaningSubject(String subject) {
    return '正在清理$subject';
  }

  @override
  String cleanupSubjectComplete(String subject) {
    return '$subject清理完成';
  }

  @override
  String cleanupSubjectCancelled(String subject) {
    return '$subject清理已取消';
  }

  @override
  String cleanupSubjectFailed(String subject) {
    return '$subject清理失败';
  }

  @override
  String get clearThumbnails => '清理缩略图';

  @override
  String get thumbnailCleanupHelp => '缩略图会在需要时重新生成，不会删除原图片';

  @override
  String get countingThumbnails => '正在统计可清理的缩略图';

  @override
  String cleanupFileProgress(String processed, String total, String size) {
    return '已处理 $processed / $total 个文件，释放 $size';
  }

  @override
  String cleanupFilesRemoved(String count, String size) {
    return '已移除 $count 个文件，释放 $size';
  }

  @override
  String cleanupBeforeStop(String count) {
    return '停止前已移除 $count 个文件，已保留原图片';
  }

  @override
  String get thumbnailCleanupFailed => '未能完成缩略图清理';

  @override
  String cleanupFailedFiles(String count) {
    return '$count 个文件未能清理';
  }

  @override
  String get chooseThisFolder => '选择此目录';

  @override
  String get catalogCleanupAlreadyFinished => '图库数据整理任务已经结束，无法再取消';

  @override
  String folderStatus(String name, String status) {
    return '“$name”$status';
  }

  @override
  String galleryDay(DateTime date) {
    final intl.DateFormat dateDateFormat = intl.DateFormat.yMMMd(localeName);
    final String dateString = dateDateFormat.format(date);

    return '$dateString';
  }

  @override
  String galleryMonth(DateTime date) {
    final intl.DateFormat dateDateFormat = intl.DateFormat.yMMMM(localeName);
    final String dateString = dateDateFormat.format(date);

    return '$dateString';
  }

  @override
  String eventDateTime(DateTime date, DateTime time) {
    final intl.DateFormat dateDateFormat = intl.DateFormat.yMMMd(localeName);
    final String dateString = dateDateFormat.format(date);
    final intl.DateFormat timeDateFormat = intl.DateFormat.jm(localeName);
    final String timeString = timeDateFormat.format(time);

    return '$dateString $timeString';
  }

  @override
  String labelAndValue(String label, String value) {
    return '$label：$value';
  }

  @override
  String get inlineListSeparator => '、';

  @override
  String get clauseSeparator => '，';

  @override
  String get sentenceSeparator => '。';

  @override
  String listSentence(String items) {
    return '$items。';
  }

  @override
  String get categoryIllustration => '插画';

  @override
  String get categoryComic => '漫画';

  @override
  String get categoryMeme => '表情包';

  @override
  String get categoryScreenshot => '截图';

  @override
  String get categoryPhoto => '实拍';

  @override
  String get categoryOther => '其他';

  @override
  String modelDownloadProgress(int percent, double received, double total) {
    final intl.NumberFormat receivedNumberFormat =
        intl.NumberFormat.decimalPatternDigits(
          locale: localeName,
          decimalDigits: 1,
        );
    final String receivedString = receivedNumberFormat.format(received);
    final intl.NumberFormat totalNumberFormat =
        intl.NumberFormat.decimalPatternDigits(
          locale: localeName,
          decimalDigits: 1,
        );
    final String totalString = totalNumberFormat.format(total);

    return '$percent% · $receivedString / $totalString MB';
  }

  @override
  String photoTimestamp(DateTime date, DateTime time) {
    final intl.DateFormat dateDateFormat = intl.DateFormat.yMMMd(localeName);
    final String dateString = dateDateFormat.format(date);
    final intl.DateFormat timeDateFormat = intl.DateFormat.jms(localeName);
    final String timeString = timeDateFormat.format(time);

    return '$dateString $timeString';
  }
}

/// The translations for Chinese, using the Han script (`zh_Hans`).
class AppLocalizationsZhHans extends AppLocalizationsZh {
  AppLocalizationsZhHans() : super('zh_Hans');
}
