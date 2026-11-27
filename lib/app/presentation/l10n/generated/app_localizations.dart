// ignore_for_file: deprecated_member_use
import 'dart:async';

import 'package:flutter/foundation.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_localizations/flutter_localizations.dart';
import 'package:intl/intl.dart' as intl;

import 'app_localizations_en.dart';
import 'app_localizations_ja.dart';
import 'app_localizations_zh.dart';

// ignore_for_file: type=lint

/// Callers can lookup localized strings with an instance of AppLocalizations
/// returned by `AppLocalizations.of(context)`.
///
/// Applications need to include `AppLocalizations.delegate()` in their app's
/// `localizationDelegates` list, and the locales they support in the app's
/// `supportedLocales` list. For example:
///
/// ```dart
/// import 'generated/app_localizations.dart';
///
/// return MaterialApp(
///   localizationsDelegates: AppLocalizations.localizationsDelegates,
///   supportedLocales: AppLocalizations.supportedLocales,
///   home: MyApplicationHome(),
/// );
/// ```
///
/// ## Update pubspec.yaml
///
/// Please make sure to update your pubspec.yaml to include the following
/// packages:
///
/// ```yaml
/// dependencies:
///   # Internationalization support.
///   flutter_localizations:
///     sdk: flutter
///   intl: any # Use the pinned version from flutter_localizations
///
///   # Rest of dependencies
/// ```
///
/// ## iOS Applications
///
/// iOS applications define key application metadata, including supported
/// locales, in an Info.plist file that is built into the application bundle.
/// To configure the locales supported by your app, you’ll need to edit this
/// file.
///
/// First, open your project’s ios/Runner.xcworkspace Xcode workspace file.
/// Then, in the Project Navigator, open the Info.plist file under the Runner
/// project’s Runner folder.
///
/// Next, select the Information Property List item, select Add Item from the
/// Editor menu, then select Localizations from the pop-up menu.
///
/// Select and expand the newly-created Localizations item then, for each
/// locale your application supports, add a new item and select the locale
/// you wish to add from the pop-up menu in the Value field. This list should
/// be consistent with the languages listed in the AppLocalizations.supportedLocales
/// property.
abstract class AppLocalizations {
  AppLocalizations(String locale)
    : localeName = intl.Intl.canonicalizedLocale(locale.toString());

  final String localeName;

  static AppLocalizations of(BuildContext context) {
    return Localizations.of<AppLocalizations>(context, AppLocalizations)!;
  }

  static const LocalizationsDelegate<AppLocalizations> delegate =
      _AppLocalizationsDelegate();

  /// A list of this localizations delegate along with the default localizations
  /// delegates.
  ///
  /// Returns a list of localizations delegates containing this delegate along with
  /// GlobalMaterialLocalizations.delegate, GlobalCupertinoLocalizations.delegate,
  /// and GlobalWidgetsLocalizations.delegate.
  ///
  /// Additional delegates can be added by appending to this list in
  /// MaterialApp. This list does not have to be used at all if a custom list
  /// of delegates is preferred or required.
  static const List<LocalizationsDelegate<dynamic>> localizationsDelegates =
      <LocalizationsDelegate<dynamic>>[
        delegate,
        GlobalMaterialLocalizations.delegate,
        GlobalCupertinoLocalizations.delegate,
        GlobalWidgetsLocalizations.delegate,
      ];

  /// A list of this localizations delegate's supported locales.
  static const List<Locale> supportedLocales = <Locale>[
    Locale('en'),
    Locale('ja'),
    Locale('zh'),
    Locale.fromSubtags(languageCode: 'zh', scriptCode: 'Hans'),
  ];

  /// No description provided for @diagnosticsTitle.
  ///
  /// In zh, this message translates to:
  /// **'诊断信息'**
  String get diagnosticsTitle;

  /// No description provided for @diagnosticsHelp.
  ///
  /// In zh, this message translates to:
  /// **'保存运行信息，用于排查问题'**
  String get diagnosticsHelp;

  /// No description provided for @diagnosticsExport.
  ///
  /// In zh, this message translates to:
  /// **'导出'**
  String get diagnosticsExport;

  /// No description provided for @diagnosticsExportAction.
  ///
  /// In zh, this message translates to:
  /// **'导出诊断信息'**
  String get diagnosticsExportAction;

  /// No description provided for @diagnosticsFileType.
  ///
  /// In zh, this message translates to:
  /// **'诊断压缩包'**
  String get diagnosticsFileType;

  /// No description provided for @diagnosticsCollecting.
  ///
  /// In zh, this message translates to:
  /// **'正在导出诊断信息'**
  String get diagnosticsCollecting;

  /// No description provided for @diagnosticsSaved.
  ///
  /// In zh, this message translates to:
  /// **'诊断信息已保存'**
  String get diagnosticsSaved;

  /// No description provided for @diagnosticsFailed.
  ///
  /// In zh, this message translates to:
  /// **'无法导出诊断信息'**
  String get diagnosticsFailed;

  /// No description provided for @diagnosticsChooseAgain.
  ///
  /// In zh, this message translates to:
  /// **'重新选择位置'**
  String get diagnosticsChooseAgain;

  /// No description provided for @diagnosticsOpenFailed.
  ///
  /// In zh, this message translates to:
  /// **'无法打开保存位置'**
  String get diagnosticsOpenFailed;

  /// No description provided for @diagnosticsCancelled.
  ///
  /// In zh, this message translates to:
  /// **'已取消导出'**
  String get diagnosticsCancelled;

  /// No description provided for @updateSettings.
  ///
  /// In zh, this message translates to:
  /// **'更新设置'**
  String get updateSettings;

  /// No description provided for @updatePolicy.
  ///
  /// In zh, this message translates to:
  /// **'自动更新'**
  String get updatePolicy;

  /// No description provided for @updateChannel.
  ///
  /// In zh, this message translates to:
  /// **'更新渠道'**
  String get updateChannel;

  /// No description provided for @updateManual.
  ///
  /// In zh, this message translates to:
  /// **'手动检查'**
  String get updateManual;

  /// No description provided for @updateCheckAutomatically.
  ///
  /// In zh, this message translates to:
  /// **'自动检查更新'**
  String get updateCheckAutomatically;

  /// No description provided for @updateDownloadAutomatically.
  ///
  /// In zh, this message translates to:
  /// **'自动下载更新'**
  String get updateDownloadAutomatically;

  /// No description provided for @updateStable.
  ///
  /// In zh, this message translates to:
  /// **'正式版'**
  String get updateStable;

  /// No description provided for @updatePreview.
  ///
  /// In zh, this message translates to:
  /// **'预览版'**
  String get updatePreview;

  /// No description provided for @updateCheck.
  ///
  /// In zh, this message translates to:
  /// **'检查更新'**
  String get updateCheck;

  /// No description provided for @updateView.
  ///
  /// In zh, this message translates to:
  /// **'查看更新'**
  String get updateView;

  /// No description provided for @updateRestart.
  ///
  /// In zh, this message translates to:
  /// **'重启并更新'**
  String get updateRestart;

  /// No description provided for @updateTitle.
  ///
  /// In zh, this message translates to:
  /// **'更新 Ame'**
  String get updateTitle;

  /// No description provided for @updateAvailable.
  ///
  /// In zh, this message translates to:
  /// **'发现新版本'**
  String get updateAvailable;

  /// No description provided for @updateCurrent.
  ///
  /// In zh, this message translates to:
  /// **'已是最新版本'**
  String get updateCurrent;

  /// No description provided for @updateChecking.
  ///
  /// In zh, this message translates to:
  /// **'正在检查更新'**
  String get updateChecking;

  /// No description provided for @updateDownloading.
  ///
  /// In zh, this message translates to:
  /// **'正在下载更新'**
  String get updateDownloading;

  /// No description provided for @updateVerifying.
  ///
  /// In zh, this message translates to:
  /// **'正在验证更新'**
  String get updateVerifying;

  /// No description provided for @updateStopping.
  ///
  /// In zh, this message translates to:
  /// **'正在停止下载'**
  String get updateStopping;

  /// No description provided for @updateReady.
  ///
  /// In zh, this message translates to:
  /// **'更新已准备就绪'**
  String get updateReady;

  /// No description provided for @updateRestarting.
  ///
  /// In zh, this message translates to:
  /// **'正在重启 Ame'**
  String get updateRestarting;

  /// No description provided for @updateDownload.
  ///
  /// In zh, this message translates to:
  /// **'下载更新'**
  String get updateDownload;

  /// No description provided for @updateLater.
  ///
  /// In zh, this message translates to:
  /// **'稍后'**
  String get updateLater;

  /// No description provided for @updateStop.
  ///
  /// In zh, this message translates to:
  /// **'停止下载'**
  String get updateStop;

  /// No description provided for @updateReadyHelp.
  ///
  /// In zh, this message translates to:
  /// **'重启 Ame 后安装更新。'**
  String get updateReadyHelp;

  /// No description provided for @updateUnavailable.
  ///
  /// In zh, this message translates to:
  /// **'请使用安装版或便携版检查更新。'**
  String get updateUnavailable;

  /// No description provided for @updateOpenStore.
  ///
  /// In zh, this message translates to:
  /// **'打开 Microsoft Store'**
  String get updateOpenStore;

  /// No description provided for @updateStoreManaged.
  ///
  /// In zh, this message translates to:
  /// **'更新由 Microsoft Store 管理。'**
  String get updateStoreManaged;

  /// No description provided for @updateStoreOpenFailure.
  ///
  /// In zh, this message translates to:
  /// **'无法打开 Microsoft Store，请稍后重试。'**
  String get updateStoreOpenFailure;

  /// No description provided for @updateNetworkFailure.
  ///
  /// In zh, this message translates to:
  /// **'暂时无法检查或下载更新，请稍后重试。'**
  String get updateNetworkFailure;

  /// No description provided for @updateReleaseFailure.
  ///
  /// In zh, this message translates to:
  /// **'这个版本的更新信息不完整，请稍后重试。'**
  String get updateReleaseFailure;

  /// No description provided for @updatePackageFailure.
  ///
  /// In zh, this message translates to:
  /// **'下载的更新包不完整，请重新下载。'**
  String get updatePackageFailure;

  /// No description provided for @updateStorageFailure.
  ///
  /// In zh, this message translates to:
  /// **'无法保存更新，请检查可用空间和文件夹权限。'**
  String get updateStorageFailure;

  /// No description provided for @updateInstallationFailure.
  ///
  /// In zh, this message translates to:
  /// **'未能安装更新，当前版本仍可使用。'**
  String get updateInstallationFailure;

  /// No description provided for @updateShutdownFailure.
  ///
  /// In zh, this message translates to:
  /// **'还有操作未能结束，请稍后重试。'**
  String get updateShutdownFailure;

  /// No description provided for @updateInstalled.
  ///
  /// In zh, this message translates to:
  /// **'Ame 已更新'**
  String get updateInstalled;

  /// No description provided for @updateCancelled.
  ///
  /// In zh, this message translates to:
  /// **'已取消安装更新'**
  String get updateCancelled;

  /// No description provided for @updateFailed.
  ///
  /// In zh, this message translates to:
  /// **'更新失败'**
  String get updateFailed;

  /// No description provided for @settingsTitle.
  ///
  /// In zh, this message translates to:
  /// **'设置'**
  String get settingsTitle;

  /// No description provided for @settingsPersonalization.
  ///
  /// In zh, this message translates to:
  /// **'个性化'**
  String get settingsPersonalization;

  /// No description provided for @settingsLanguage.
  ///
  /// In zh, this message translates to:
  /// **'应用语言'**
  String get settingsLanguage;

  /// No description provided for @settingsLanguageHelp.
  ///
  /// In zh, this message translates to:
  /// **'选择 Ame 界面使用的语言'**
  String get settingsLanguageHelp;

  /// No description provided for @languageSystem.
  ///
  /// In zh, this message translates to:
  /// **'跟随系统'**
  String get languageSystem;

  /// No description provided for @languageChinese.
  ///
  /// In zh, this message translates to:
  /// **'简体中文'**
  String get languageChinese;

  /// No description provided for @languageEnglish.
  ///
  /// In zh, this message translates to:
  /// **'English'**
  String get languageEnglish;

  /// No description provided for @languageJapanese.
  ///
  /// In zh, this message translates to:
  /// **'日本語'**
  String get languageJapanese;

  /// No description provided for @settingsTheme.
  ///
  /// In zh, this message translates to:
  /// **'应用主题'**
  String get settingsTheme;

  /// No description provided for @settingsThemeHelp.
  ///
  /// In zh, this message translates to:
  /// **'选择明暗外观，主题色跟随 Windows'**
  String get settingsThemeHelp;

  /// No description provided for @themeLight.
  ///
  /// In zh, this message translates to:
  /// **'浅色'**
  String get themeLight;

  /// No description provided for @themeDark.
  ///
  /// In zh, this message translates to:
  /// **'深色'**
  String get themeDark;

  /// No description provided for @settingsBrowsing.
  ///
  /// In zh, this message translates to:
  /// **'浏览'**
  String get settingsBrowsing;

  /// No description provided for @settingsWheel.
  ///
  /// In zh, this message translates to:
  /// **'鼠标滚轮'**
  String get settingsWheel;

  /// No description provided for @settingsWheelHelp.
  ///
  /// In zh, this message translates to:
  /// **'查看单张图片时滚动滚轮的行为'**
  String get settingsWheelHelp;

  /// No description provided for @settingsWheelZoom.
  ///
  /// In zh, this message translates to:
  /// **'放大或缩小'**
  String get settingsWheelZoom;

  /// No description provided for @settingsWheelNavigate.
  ///
  /// In zh, this message translates to:
  /// **'上一张或下一张'**
  String get settingsWheelNavigate;

  /// No description provided for @settingsOpenPhoto.
  ///
  /// In zh, this message translates to:
  /// **'打开图片'**
  String get settingsOpenPhoto;

  /// No description provided for @settingsOpenPhotoHelp.
  ///
  /// In zh, this message translates to:
  /// **'自动模式缩小大图，小图保持实际大小'**
  String get settingsOpenPhotoHelp;

  /// No description provided for @viewerAutomatic.
  ///
  /// In zh, this message translates to:
  /// **'自动'**
  String get viewerAutomatic;

  /// No description provided for @viewerFitWindow.
  ///
  /// In zh, this message translates to:
  /// **'适应窗口'**
  String get viewerFitWindow;

  /// No description provided for @viewerActualSize.
  ///
  /// In zh, this message translates to:
  /// **'实际大小'**
  String get viewerActualSize;

  /// No description provided for @settingsThumbnailSpeed.
  ///
  /// In zh, this message translates to:
  /// **'缩略图加载速度'**
  String get settingsThumbnailSpeed;

  /// No description provided for @settingsThumbnailSpeedHelp.
  ///
  /// In zh, this message translates to:
  /// **'档位越大，同时加载越多，占用资源也越高'**
  String get settingsThumbnailSpeedHelp;

  /// No description provided for @sizeSmall.
  ///
  /// In zh, this message translates to:
  /// **'小'**
  String get sizeSmall;

  /// No description provided for @sizeMedium.
  ///
  /// In zh, this message translates to:
  /// **'中'**
  String get sizeMedium;

  /// No description provided for @sizeLarge.
  ///
  /// In zh, this message translates to:
  /// **'大'**
  String get sizeLarge;

  /// No description provided for @settingsAbout.
  ///
  /// In zh, this message translates to:
  /// **'关于'**
  String get settingsAbout;

  /// No description provided for @settingsVersion.
  ///
  /// In zh, this message translates to:
  /// **'版本'**
  String get settingsVersion;

  /// No description provided for @settingsLicenses.
  ///
  /// In zh, this message translates to:
  /// **'开源软件声明'**
  String get settingsLicenses;

  /// No description provided for @settingsLicensesHelp.
  ///
  /// In zh, this message translates to:
  /// **'查看 Ame 使用的开源软件与许可'**
  String get settingsLicensesHelp;

  /// No description provided for @settingsSaveFailed.
  ///
  /// In zh, this message translates to:
  /// **'无法保存设置'**
  String get settingsSaveFailed;

  /// No description provided for @startupFailureTitle.
  ///
  /// In zh, this message translates to:
  /// **'Cedarflake Ame 无法启动'**
  String get startupFailureTitle;

  /// No description provided for @copyDiagnostics.
  ///
  /// In zh, this message translates to:
  /// **'复制诊断信息'**
  String get copyDiagnostics;

  /// No description provided for @diagnosticsCopied.
  ///
  /// In zh, this message translates to:
  /// **'诊断信息已复制'**
  String get diagnosticsCopied;

  /// No description provided for @startupUnsupportedCatalog.
  ///
  /// In zh, this message translates to:
  /// **'当前版本不支持这份图库数据。请导出诊断信息以便排查。'**
  String get startupUnsupportedCatalog;

  /// No description provided for @startupDatabaseError.
  ///
  /// In zh, this message translates to:
  /// **'图库数据暂时无法读取。请确认磁盘可用后重新启动；如果仍然失败，请导出诊断信息以便排查。'**
  String get startupDatabaseError;

  /// No description provided for @startupCatalogError.
  ///
  /// In zh, this message translates to:
  /// **'图库加载失败。请重新启动 Ame；如果仍然失败，请导出诊断信息以便排查。'**
  String get startupCatalogError;

  /// No description provided for @startupComponentError.
  ///
  /// In zh, this message translates to:
  /// **'应用组件加载失败。请重新启动 Ame；如果仍然失败，请导出诊断信息以便排查。'**
  String get startupComponentError;

  /// No description provided for @choose.
  ///
  /// In zh, this message translates to:
  /// **'选择'**
  String get choose;

  /// No description provided for @notSet.
  ///
  /// In zh, this message translates to:
  /// **'未设置'**
  String get notSet;

  /// No description provided for @notifications.
  ///
  /// In zh, this message translates to:
  /// **'通知'**
  String get notifications;

  /// No description provided for @notificationsEmpty.
  ///
  /// In zh, this message translates to:
  /// **'暂无通知'**
  String get notificationsEmpty;

  /// No description provided for @notificationHistory.
  ///
  /// In zh, this message translates to:
  /// **'历史记录'**
  String get notificationHistory;

  /// No description provided for @acknowledge.
  ///
  /// In zh, this message translates to:
  /// **'知道了'**
  String get acknowledge;

  /// No description provided for @notificationDetails.
  ///
  /// In zh, this message translates to:
  /// **'通知详情'**
  String get notificationDetails;

  /// No description provided for @sourceDirectory.
  ///
  /// In zh, this message translates to:
  /// **'目录'**
  String get sourceDirectory;

  /// No description provided for @technicalCode.
  ///
  /// In zh, this message translates to:
  /// **'技术代码'**
  String get technicalCode;

  /// No description provided for @close.
  ///
  /// In zh, this message translates to:
  /// **'关闭'**
  String get close;

  /// No description provided for @operationFailedDetails.
  ///
  /// In zh, this message translates to:
  /// **'操作未完成，详细信息已保留。'**
  String get operationFailedDetails;

  /// No description provided for @retry.
  ///
  /// In zh, this message translates to:
  /// **'重试'**
  String get retry;

  /// No description provided for @errorCode.
  ///
  /// In zh, this message translates to:
  /// **'错误代码'**
  String get errorCode;

  /// No description provided for @openLocation.
  ///
  /// In zh, this message translates to:
  /// **'打开位置'**
  String get openLocation;

  /// No description provided for @openLocationFailed.
  ///
  /// In zh, this message translates to:
  /// **'无法打开这个位置，请重试。'**
  String get openLocationFailed;

  /// No description provided for @loading.
  ///
  /// In zh, this message translates to:
  /// **'正在加载'**
  String get loading;

  /// No description provided for @elapsedTime.
  ///
  /// In zh, this message translates to:
  /// **'已持续 {duration}'**
  String elapsedTime(String duration);

  /// No description provided for @occurredAt.
  ///
  /// In zh, this message translates to:
  /// **'发生时间'**
  String get occurredAt;

  /// No description provided for @elapsedSeconds.
  ///
  /// In zh, this message translates to:
  /// **'{seconds} 秒'**
  String elapsedSeconds(int seconds);

  /// No description provided for @elapsedMinutes.
  ///
  /// In zh, this message translates to:
  /// **'{minutes} 分 {seconds} 秒'**
  String elapsedMinutes(int minutes, int seconds);

  /// No description provided for @elapsedHours.
  ///
  /// In zh, this message translates to:
  /// **'{hours} 小时 {minutes} 分'**
  String elapsedHours(int hours, int minutes);

  /// No description provided for @notificationsUnread.
  ///
  /// In zh, this message translates to:
  /// **'通知，有未读消息'**
  String get notificationsUnread;

  /// No description provided for @notificationsRead.
  ///
  /// In zh, this message translates to:
  /// **'通知，无未读消息'**
  String get notificationsRead;

  /// No description provided for @windowMinimize.
  ///
  /// In zh, this message translates to:
  /// **'最小化'**
  String get windowMinimize;

  /// No description provided for @windowRestore.
  ///
  /// In zh, this message translates to:
  /// **'还原'**
  String get windowRestore;

  /// No description provided for @windowMaximize.
  ///
  /// In zh, this message translates to:
  /// **'最大化'**
  String get windowMaximize;

  /// No description provided for @chooseFolder.
  ///
  /// In zh, this message translates to:
  /// **'选择文件夹'**
  String get chooseFolder;

  /// No description provided for @albumCreated.
  ///
  /// In zh, this message translates to:
  /// **'已新建相册'**
  String get albumCreated;

  /// No description provided for @albumRenamed.
  ///
  /// In zh, this message translates to:
  /// **'已重命名相册'**
  String get albumRenamed;

  /// No description provided for @albumIconChanged.
  ///
  /// In zh, this message translates to:
  /// **'已更改相册图标'**
  String get albumIconChanged;

  /// No description provided for @albumDeleted.
  ///
  /// In zh, this message translates to:
  /// **'已删除相册'**
  String get albumDeleted;

  /// No description provided for @removedFromAlbum.
  ///
  /// In zh, this message translates to:
  /// **'已移出相册'**
  String get removedFromAlbum;

  /// No description provided for @movedToAlbum.
  ///
  /// In zh, this message translates to:
  /// **'已移至“{name}”'**
  String movedToAlbum(String name);

  /// No description provided for @addedToAlbum.
  ///
  /// In zh, this message translates to:
  /// **'已加入“{name}”'**
  String addedToAlbum(String name);

  /// No description provided for @archiveExported.
  ///
  /// In zh, this message translates to:
  /// **'整理记录已导出'**
  String get archiveExported;

  /// No description provided for @backupSaved.
  ///
  /// In zh, this message translates to:
  /// **'备份已保存到所选位置。'**
  String get backupSaved;

  /// No description provided for @archiveRestored.
  ///
  /// In zh, this message translates to:
  /// **'整理记录已恢复'**
  String get archiveRestored;

  /// No description provided for @unlinkedRecordsKept.
  ///
  /// In zh, this message translates to:
  /// **'暂未关联的记录已保留。'**
  String get unlinkedRecordsKept;

  /// No description provided for @favoriteAdded.
  ///
  /// In zh, this message translates to:
  /// **'已加入收藏夹'**
  String get favoriteAdded;

  /// No description provided for @favoriteRemoved.
  ///
  /// In zh, this message translates to:
  /// **'已取消收藏'**
  String get favoriteRemoved;

  /// No description provided for @unlinkedRecordsCleared.
  ///
  /// In zh, this message translates to:
  /// **'已清理未关联记录'**
  String get unlinkedRecordsCleared;

  /// No description provided for @recordsCleared.
  ///
  /// In zh, this message translates to:
  /// **'已清理 {count} 项记录。原图片和相册均已保留。'**
  String recordsCleared(int count);

  /// No description provided for @changesSaveFailed.
  ///
  /// In zh, this message translates to:
  /// **'未能保存更改'**
  String get changesSaveFailed;

  /// No description provided for @organizationRecordChanged.
  ///
  /// In zh, this message translates to:
  /// **'图片或整理记录已更改，当前记录已保留。'**
  String get organizationRecordChanged;

  /// No description provided for @organizationHistoryChanged.
  ///
  /// In zh, this message translates to:
  /// **'操作记录已更新，请重新打开菜单。'**
  String get organizationHistoryChanged;

  /// No description provided for @organizationSelectionChanged.
  ///
  /// In zh, this message translates to:
  /// **'图片或整理信息已更新，请重试。'**
  String get organizationSelectionChanged;

  /// No description provided for @organizationSelectionExpired.
  ///
  /// In zh, this message translates to:
  /// **'图片或整理信息已更新，请重新选择图片。'**
  String get organizationSelectionExpired;

  /// No description provided for @changesUnconfirmed.
  ///
  /// In zh, this message translates to:
  /// **'更改尚未确认保存，请重试。'**
  String get changesUnconfirmed;

  /// No description provided for @undoCompleted.
  ///
  /// In zh, this message translates to:
  /// **'已撤销'**
  String get undoCompleted;

  /// No description provided for @redoCompleted.
  ///
  /// In zh, this message translates to:
  /// **'已重做'**
  String get redoCompleted;

  /// No description provided for @undoFailed.
  ///
  /// In zh, this message translates to:
  /// **'无法撤销'**
  String get undoFailed;

  /// No description provided for @redoFailed.
  ///
  /// In zh, this message translates to:
  /// **'无法重做'**
  String get redoFailed;

  /// No description provided for @undo.
  ///
  /// In zh, this message translates to:
  /// **'撤销'**
  String get undo;

  /// No description provided for @changesSaved.
  ///
  /// In zh, this message translates to:
  /// **'更改已保存'**
  String get changesSaved;

  /// No description provided for @libraryRefreshPending.
  ///
  /// In zh, this message translates to:
  /// **'图库尚未刷新，请重试。'**
  String get libraryRefreshPending;

  /// No description provided for @locationRefreshFailed.
  ///
  /// In zh, this message translates to:
  /// **'无法刷新图库位置信息，请重试。'**
  String get locationRefreshFailed;

  /// No description provided for @displayRefreshSuperseded.
  ///
  /// In zh, this message translates to:
  /// **'图库已保存，但当前显示刷新已被其他操作替换，请重试刷新显示。'**
  String get displayRefreshSuperseded;

  /// No description provided for @deferredOrganization.
  ///
  /// In zh, this message translates to:
  /// **'已移至稍后整理'**
  String get deferredOrganization;

  /// No description provided for @ignored.
  ///
  /// In zh, this message translates to:
  /// **'已忽略'**
  String get ignored;

  /// No description provided for @restoredToInbox.
  ///
  /// In zh, this message translates to:
  /// **'已恢复待整理'**
  String get restoredToInbox;

  /// No description provided for @relocationUnsupported.
  ///
  /// In zh, this message translates to:
  /// **'当前扫描服务不支持重新定位图库。'**
  String get relocationUnsupported;

  /// No description provided for @albumRemovalRemaining.
  ///
  /// In zh, this message translates to:
  /// **'相册信息已更新，请重试剩余项目。'**
  String get albumRemovalRemaining;

  /// No description provided for @albumRemovalFailed.
  ///
  /// In zh, this message translates to:
  /// **'未能删除“{name}”，请重试。'**
  String albumRemovalFailed(String name);

  /// No description provided for @operationStopped.
  ///
  /// In zh, this message translates to:
  /// **'操作已停止。'**
  String get operationStopped;

  /// No description provided for @operationIncomplete.
  ///
  /// In zh, this message translates to:
  /// **'未能完成操作，请重试。'**
  String get operationIncomplete;

  /// No description provided for @folderStillUpdating.
  ///
  /// In zh, this message translates to:
  /// **'文件夹状态已更改或仍在更新，请稍后重试。'**
  String get folderStillUpdating;

  /// No description provided for @folderRemovedRefreshPending.
  ///
  /// In zh, this message translates to:
  /// **'文件夹已移除，图库尚未刷新。请关闭此窗口后重试图库刷新。'**
  String get folderRemovedRefreshPending;

  /// No description provided for @folderRemovalFailed.
  ///
  /// In zh, this message translates to:
  /// **'未能移除文件夹，请重试。'**
  String get folderRemovalFailed;

  /// No description provided for @scanEndedWithoutResult.
  ///
  /// In zh, this message translates to:
  /// **'图库更新未返回完成结果'**
  String get scanEndedWithoutResult;

  /// No description provided for @updatePausedRestart.
  ///
  /// In zh, this message translates to:
  /// **'图库更新已暂停，请重新开始更新。'**
  String get updatePausedRestart;

  /// No description provided for @sourceChangedDuringUpdate.
  ///
  /// In zh, this message translates to:
  /// **'更新期间源文件发生变化，请重试。'**
  String get sourceChangedDuringUpdate;

  /// No description provided for @updateSourceChanged.
  ///
  /// In zh, this message translates to:
  /// **'待更新的来源已发生变化，请重新选择来源。'**
  String get updateSourceChanged;

  /// No description provided for @addToAlbum.
  ///
  /// In zh, this message translates to:
  /// **'加入相册'**
  String get addToAlbum;

  /// No description provided for @moveToAnotherAlbum.
  ///
  /// In zh, this message translates to:
  /// **'移至其他相册'**
  String get moveToAnotherAlbum;

  /// No description provided for @addOrMoveToAlbum.
  ///
  /// In zh, this message translates to:
  /// **'加入或移至相册'**
  String get addOrMoveToAlbum;

  /// No description provided for @filterFormat.
  ///
  /// In zh, this message translates to:
  /// **'格式：{format}'**
  String filterFormat(String format);

  /// No description provided for @filterWidth.
  ///
  /// In zh, this message translates to:
  /// **'宽度：{range} px'**
  String filterWidth(String range);

  /// No description provided for @filterHeight.
  ///
  /// In zh, this message translates to:
  /// **'高度：{range} px'**
  String filterHeight(String range);

  /// No description provided for @filterCamera.
  ///
  /// In zh, this message translates to:
  /// **'相机：{camera}'**
  String filterCamera(String camera);

  /// No description provided for @filterFileSize.
  ///
  /// In zh, this message translates to:
  /// **'文件大小：{range} {unit}'**
  String filterFileSize(String range, String unit);

  /// No description provided for @organizationEnterFailed.
  ///
  /// In zh, this message translates to:
  /// **'无法进入整理模式，请重试。'**
  String get organizationEnterFailed;

  /// No description provided for @galleryReturnFailed.
  ///
  /// In zh, this message translates to:
  /// **'无法返回图库，请重试。'**
  String get galleryReturnFailed;

  /// No description provided for @organizationScopeFailed.
  ///
  /// In zh, this message translates to:
  /// **'无法切换整理范围，请重试。'**
  String get organizationScopeFailed;

  /// No description provided for @conflictOpenFailed.
  ///
  /// In zh, this message translates to:
  /// **'无法打开归属冲突的图片，请重试。'**
  String get conflictOpenFailed;

  /// No description provided for @organizationPositionUnsaved.
  ///
  /// In zh, this message translates to:
  /// **'整理位置尚未保存，请重试。'**
  String get organizationPositionUnsaved;

  /// No description provided for @organizationPositionUnreadable.
  ///
  /// In zh, this message translates to:
  /// **'无法读取整理位置，请重试。'**
  String get organizationPositionUnreadable;

  /// No description provided for @organizationResumeFailed.
  ///
  /// In zh, this message translates to:
  /// **'无法恢复整理位置，请重试。'**
  String get organizationResumeFailed;

  /// No description provided for @organizationNextFailed.
  ///
  /// In zh, this message translates to:
  /// **'更改已保存，无法打开下一张图片，请重试。'**
  String get organizationNextFailed;

  /// No description provided for @organizationLibraryUpdating.
  ///
  /// In zh, this message translates to:
  /// **'图库正在更新，请重试切换图片。'**
  String get organizationLibraryUpdating;

  /// No description provided for @organizationViewChanged.
  ///
  /// In zh, this message translates to:
  /// **'图库窗口已变化，请重试切换图片。'**
  String get organizationViewChanged;

  /// No description provided for @unknown.
  ///
  /// In zh, this message translates to:
  /// **'未知'**
  String get unknown;

  /// No description provided for @smartAnalysisInterrupted.
  ///
  /// In zh, this message translates to:
  /// **'智能相册分析已中断'**
  String get smartAnalysisInterrupted;

  /// No description provided for @smartResultsAvailable.
  ///
  /// In zh, this message translates to:
  /// **'已有分类结果仍可浏览。'**
  String get smartResultsAvailable;

  /// No description provided for @albumConflict.
  ///
  /// In zh, this message translates to:
  /// **'相册归属冲突'**
  String get albumConflict;

  /// No description provided for @conflictsDeferred.
  ///
  /// In zh, this message translates to:
  /// **'{count} 组图片需要选择相册，已放入稍后整理。'**
  String conflictsDeferred(int count);

  /// No description provided for @viewAction.
  ///
  /// In zh, this message translates to:
  /// **'查看'**
  String get viewAction;

  /// No description provided for @duplicateWorkspace.
  ///
  /// In zh, this message translates to:
  /// **'重复图片查找'**
  String get duplicateWorkspace;

  /// No description provided for @openInExplorer.
  ///
  /// In zh, this message translates to:
  /// **'在文件资源管理器中打开'**
  String get openInExplorer;

  /// No description provided for @openInExplorerFailed.
  ///
  /// In zh, this message translates to:
  /// **'无法在文件资源管理器中打开'**
  String get openInExplorerFailed;

  /// No description provided for @stopping.
  ///
  /// In zh, this message translates to:
  /// **'正在停止'**
  String get stopping;

  /// No description provided for @duplicateRunning.
  ///
  /// In zh, this message translates to:
  /// **'正在查重'**
  String get duplicateRunning;

  /// No description provided for @duplicateIncomplete.
  ///
  /// In zh, this message translates to:
  /// **'未完成查重'**
  String get duplicateIncomplete;

  /// No description provided for @waitingForBackground.
  ///
  /// In zh, this message translates to:
  /// **'等待后台'**
  String get waitingForBackground;

  /// No description provided for @paused.
  ///
  /// In zh, this message translates to:
  /// **'已暂停'**
  String get paused;

  /// No description provided for @duplicateEmptyAfter.
  ///
  /// In zh, this message translates to:
  /// **'已检查的图片中没有发现重复项'**
  String get duplicateEmptyAfter;

  /// No description provided for @duplicateWaitResults.
  ///
  /// In zh, this message translates to:
  /// **'找到的重复图片会显示在这里。'**
  String get duplicateWaitResults;

  /// No description provided for @duplicateLocationsChanged.
  ///
  /// In zh, this message translates to:
  /// **'该组某些图片的位置已发生变化'**
  String get duplicateLocationsChanged;

  /// No description provided for @resultsUnavailable.
  ///
  /// In zh, this message translates to:
  /// **'暂时无法读取结果'**
  String get resultsUnavailable;

  /// No description provided for @viewDuplicates.
  ///
  /// In zh, this message translates to:
  /// **'查看重复图片'**
  String get viewDuplicates;

  /// No description provided for @backToDuplicates.
  ///
  /// In zh, this message translates to:
  /// **'返回重复图片'**
  String get backToDuplicates;

  /// No description provided for @duplicateGroups.
  ///
  /// In zh, this message translates to:
  /// **'重复组'**
  String get duplicateGroups;

  /// No description provided for @ignoredGroupsEmpty.
  ///
  /// In zh, this message translates to:
  /// **'没有已忽略的组'**
  String get ignoredGroupsEmpty;

  /// No description provided for @similarGroups.
  ///
  /// In zh, this message translates to:
  /// **'相似组'**
  String get similarGroups;

  /// No description provided for @similarityLevel.
  ///
  /// In zh, this message translates to:
  /// **'相似程度'**
  String get similarityLevel;

  /// No description provided for @similarityStrict.
  ///
  /// In zh, this message translates to:
  /// **'严格'**
  String get similarityStrict;

  /// No description provided for @similarityStandard.
  ///
  /// In zh, this message translates to:
  /// **'标准'**
  String get similarityStandard;

  /// No description provided for @similarityBroad.
  ///
  /// In zh, this message translates to:
  /// **'宽松'**
  String get similarityBroad;

  /// No description provided for @viewSimilar.
  ///
  /// In zh, this message translates to:
  /// **'查看相似图片'**
  String get viewSimilar;

  /// No description provided for @similarEmpty.
  ///
  /// In zh, this message translates to:
  /// **'已检查的图片中没有发现相似组'**
  String get similarEmpty;

  /// No description provided for @similarWaitResults.
  ///
  /// In zh, this message translates to:
  /// **'找到的相似图片会显示在这里'**
  String get similarWaitResults;

  /// No description provided for @comparePhotos.
  ///
  /// In zh, this message translates to:
  /// **'对比图片'**
  String get comparePhotos;

  /// No description provided for @chooseComparison.
  ///
  /// In zh, this message translates to:
  /// **'选择图片进行对比'**
  String get chooseComparison;

  /// No description provided for @syncComparison.
  ///
  /// In zh, this message translates to:
  /// **'同步缩放和移动'**
  String get syncComparison;

  /// No description provided for @resetComparison.
  ///
  /// In zh, this message translates to:
  /// **'重置对比视图'**
  String get resetComparison;

  /// No description provided for @zoomIn.
  ///
  /// In zh, this message translates to:
  /// **'放大'**
  String get zoomIn;

  /// No description provided for @zoomOut.
  ///
  /// In zh, this message translates to:
  /// **'缩小'**
  String get zoomOut;

  /// No description provided for @setPreferred.
  ///
  /// In zh, this message translates to:
  /// **'设为首选'**
  String get setPreferred;

  /// No description provided for @selectPhoto.
  ///
  /// In zh, this message translates to:
  /// **'选择图片'**
  String get selectPhoto;

  /// No description provided for @deselectPhoto.
  ///
  /// In zh, this message translates to:
  /// **'取消选择'**
  String get deselectPhoto;

  /// No description provided for @clearPreferred.
  ///
  /// In zh, this message translates to:
  /// **'取消首选'**
  String get clearPreferred;

  /// No description provided for @ignoreGroup.
  ///
  /// In zh, this message translates to:
  /// **'忽略此组'**
  String get ignoreGroup;

  /// No description provided for @restoreGroup.
  ///
  /// In zh, this message translates to:
  /// **'恢复此组'**
  String get restoreGroup;

  /// No description provided for @selectionSaveFailed.
  ///
  /// In zh, this message translates to:
  /// **'未能保存选择，请重试'**
  String get selectionSaveFailed;

  /// No description provided for @groupCount.
  ///
  /// In zh, this message translates to:
  /// **'{count} 个组'**
  String groupCount(int count);

  /// No description provided for @photoCount.
  ///
  /// In zh, this message translates to:
  /// **'{count} 张图片'**
  String photoCount(int count);

  /// No description provided for @deselectNamedGroup.
  ///
  /// In zh, this message translates to:
  /// **'取消选择图片组：{name}'**
  String deselectNamedGroup(String name);

  /// No description provided for @selectNamedGroup.
  ///
  /// In zh, this message translates to:
  /// **'选择图片组：{name}'**
  String selectNamedGroup(String name);

  /// No description provided for @filter.
  ///
  /// In zh, this message translates to:
  /// **'筛选'**
  String get filter;

  /// No description provided for @showSubfolders.
  ///
  /// In zh, this message translates to:
  /// **'显示子目录'**
  String get showSubfolders;

  /// No description provided for @hideSubfolders.
  ///
  /// In zh, this message translates to:
  /// **'隐藏子目录'**
  String get hideSubfolders;

  /// No description provided for @allPhotosFilter.
  ///
  /// In zh, this message translates to:
  /// **'全部图片'**
  String get allPhotosFilter;

  /// No description provided for @locationCount.
  ///
  /// In zh, this message translates to:
  /// **'{count} 个位置'**
  String locationCount(int count);

  /// No description provided for @duplicatePhotoCount.
  ///
  /// In zh, this message translates to:
  /// **'{count} 张重复图片'**
  String duplicatePhotoCount(int count);

  /// No description provided for @physicalFileCount.
  ///
  /// In zh, this message translates to:
  /// **'{count} 份文件'**
  String physicalFileCount(int count);

  /// No description provided for @cloudOnly.
  ///
  /// In zh, this message translates to:
  /// **'仅在云端'**
  String get cloudOnly;

  /// No description provided for @folderOffline.
  ///
  /// In zh, this message translates to:
  /// **'文件夹离线'**
  String get folderOffline;

  /// No description provided for @temporarilyInaccessible.
  ///
  /// In zh, this message translates to:
  /// **'暂时无法访问'**
  String get temporarilyInaccessible;

  /// No description provided for @deleteChangedFile.
  ///
  /// In zh, this message translates to:
  /// **'文件已更改，未删除。请查看最新图片后重新选择并确认。'**
  String get deleteChangedFile;

  /// No description provided for @noPhotosToDelete.
  ///
  /// In zh, this message translates to:
  /// **'没有可删除的图片'**
  String get noPhotosToDelete;

  /// No description provided for @aboutToDeletePhotos.
  ///
  /// In zh, this message translates to:
  /// **'即将删除 {count} 张图片'**
  String aboutToDeletePhotos(int count);

  /// No description provided for @confirmDeleteFiles.
  ///
  /// In zh, this message translates to:
  /// **'删除 {count} 个文件？'**
  String confirmDeleteFiles(int count);

  /// No description provided for @confirmDeleteWholeGroup.
  ///
  /// In zh, this message translates to:
  /// **'删除这组图片的全部 {count} 个文件？'**
  String confirmDeleteWholeGroup(int count);

  /// No description provided for @confirmDeleteGroups.
  ///
  /// In zh, this message translates to:
  /// **'删除 {groups} 组图片，共 {files} 个文件？'**
  String confirmDeleteGroups(int groups, int files);

  /// No description provided for @destinationFolderDirect.
  ///
  /// In zh, this message translates to:
  /// **'直接放入目标文件夹'**
  String get destinationFolderDirect;

  /// No description provided for @byCategory.
  ///
  /// In zh, this message translates to:
  /// **'按分类'**
  String get byCategory;

  /// No description provided for @byAlbum.
  ///
  /// In zh, this message translates to:
  /// **'按相册'**
  String get byAlbum;

  /// No description provided for @byDate.
  ///
  /// In zh, this message translates to:
  /// **'按日期'**
  String get byDate;

  /// No description provided for @bySourceFolder.
  ///
  /// In zh, this message translates to:
  /// **'按来源文件夹'**
  String get bySourceFolder;

  /// No description provided for @byAlbumYear.
  ///
  /// In zh, this message translates to:
  /// **'按相册与年份'**
  String get byAlbumYear;

  /// No description provided for @dateFolderRule.
  ///
  /// In zh, this message translates to:
  /// **'按图库日期建立 YYYY-MM 文件夹；缺少日期的文件会跳过。'**
  String get dateFolderRule;

  /// No description provided for @albumYearRule.
  ///
  /// In zh, this message translates to:
  /// **'按图库日期放入相册名／年份文件夹，例如：旅行／2024。'**
  String get albumYearRule;

  /// No description provided for @unknownDestinationRule.
  ///
  /// In zh, this message translates to:
  /// **'无法确定目标的文件会跳过，并在目标文件夹中生成结果清单。'**
  String get unknownDestinationRule;

  /// No description provided for @copy.
  ///
  /// In zh, this message translates to:
  /// **'复制'**
  String get copy;

  /// No description provided for @move.
  ///
  /// In zh, this message translates to:
  /// **'移动'**
  String get move;

  /// No description provided for @rename.
  ///
  /// In zh, this message translates to:
  /// **'重命名'**
  String get rename;

  /// No description provided for @deleteAction.
  ///
  /// In zh, this message translates to:
  /// **'删除'**
  String get deleteAction;

  /// No description provided for @filesCompleted.
  ///
  /// In zh, this message translates to:
  /// **'{action} {count} 个文件'**
  String filesCompleted(String action, int count);

  /// No description provided for @deleted.
  ///
  /// In zh, this message translates to:
  /// **'已删除'**
  String get deleted;

  /// No description provided for @completed.
  ///
  /// In zh, this message translates to:
  /// **'已完成'**
  String get completed;

  /// No description provided for @filesIncomplete.
  ///
  /// In zh, this message translates to:
  /// **'未完成 {count} 个'**
  String filesIncomplete(int count);

  /// No description provided for @filesStopped.
  ///
  /// In zh, this message translates to:
  /// **'已停止 {count} 个'**
  String filesStopped(int count);

  /// No description provided for @filesSkipped.
  ///
  /// In zh, this message translates to:
  /// **'已跳过 {count} 个'**
  String filesSkipped(int count);

  /// No description provided for @noFilesProcessed.
  ///
  /// In zh, this message translates to:
  /// **'未处理任何文件。'**
  String get noFilesProcessed;

  /// No description provided for @reportNotNeeded.
  ///
  /// In zh, this message translates to:
  /// **'无需生成结果清单'**
  String get reportNotNeeded;

  /// No description provided for @reportSaved.
  ///
  /// In zh, this message translates to:
  /// **'结果清单已保存'**
  String get reportSaved;

  /// No description provided for @reportFailed.
  ///
  /// In zh, this message translates to:
  /// **'结果清单未能生成'**
  String get reportFailed;

  /// No description provided for @checkFileOperation.
  ///
  /// In zh, this message translates to:
  /// **'核对文件操作'**
  String get checkFileOperation;

  /// No description provided for @generateReport.
  ///
  /// In zh, this message translates to:
  /// **'生成结果清单'**
  String get generateReport;

  /// No description provided for @preparingOperation.
  ///
  /// In zh, this message translates to:
  /// **'正在准备{action}'**
  String preparingOperation(String action);

  /// No description provided for @runningOperation.
  ///
  /// In zh, this message translates to:
  /// **'正在{action}'**
  String runningOperation(String action);

  /// No description provided for @operationEnded.
  ///
  /// In zh, this message translates to:
  /// **'{action}已结束'**
  String operationEnded(String action);

  /// No description provided for @operationStoppedLabel.
  ///
  /// In zh, this message translates to:
  /// **'{action}已停止'**
  String operationStoppedLabel(String action);

  /// No description provided for @fileOperationNeedsAttention.
  ///
  /// In zh, this message translates to:
  /// **'文件操作需要处理'**
  String get fileOperationNeedsAttention;

  /// No description provided for @notStarted.
  ///
  /// In zh, this message translates to:
  /// **'尚未开始'**
  String get notStarted;

  /// No description provided for @verifyResults.
  ///
  /// In zh, this message translates to:
  /// **'需要核对结果'**
  String get verifyResults;

  /// No description provided for @moveIncomplete.
  ///
  /// In zh, this message translates to:
  /// **'移动未完成'**
  String get moveIncomplete;

  /// No description provided for @filesProcessedRefreshPending.
  ///
  /// In zh, this message translates to:
  /// **'文件已处理，图库尚未更新'**
  String get filesProcessedRefreshPending;

  /// No description provided for @incomplete.
  ///
  /// In zh, this message translates to:
  /// **'未完成'**
  String get incomplete;

  /// No description provided for @stopped.
  ///
  /// In zh, this message translates to:
  /// **'已停止'**
  String get stopped;

  /// No description provided for @resultsToVerify.
  ///
  /// In zh, this message translates to:
  /// **'结果待核对'**
  String get resultsToVerify;

  /// No description provided for @duplicateVerificationPending.
  ///
  /// In zh, this message translates to:
  /// **'重复组尚未核对完成，请稍后重试。'**
  String get duplicateVerificationPending;

  /// No description provided for @selectedPhotosChanged.
  ///
  /// In zh, this message translates to:
  /// **'所选图片已变化，请查看当前图库后重新选择。'**
  String get selectedPhotosChanged;

  /// No description provided for @selectedFilesChanged.
  ///
  /// In zh, this message translates to:
  /// **'所选文件已变化，未执行操作。请查看最新文件后重新选择。'**
  String get selectedFilesChanged;

  /// No description provided for @deleteFilesUnverified.
  ///
  /// In zh, this message translates to:
  /// **'文件已变化或无法核对，未删除。请查看最新图片后重新选择并确认。'**
  String get deleteFilesUnverified;

  /// No description provided for @filesUnverified.
  ///
  /// In zh, this message translates to:
  /// **'文件已变化或无法核对，未执行操作。请查看最新文件后重新选择。'**
  String get filesUnverified;

  /// No description provided for @multipleFileLocations.
  ///
  /// In zh, this message translates to:
  /// **'所选图片有多个文件位置，请取消多选后，分别在图片右键菜单中操作。'**
  String get multipleFileLocations;

  /// No description provided for @destinationMatchesSource.
  ///
  /// In zh, this message translates to:
  /// **'目标位置与原文件相同，请选择其他文件夹或名称。'**
  String get destinationMatchesSource;

  /// No description provided for @destinationUnavailable.
  ///
  /// In zh, this message translates to:
  /// **'无法打开目标文件夹，请重新选择。'**
  String get destinationUnavailable;

  /// No description provided for @selectedFilesUnreadable.
  ///
  /// In zh, this message translates to:
  /// **'无法读取所选文件，请刷新图库后重新选择。'**
  String get selectedFilesUnreadable;

  /// No description provided for @retainedPhotoChanged.
  ///
  /// In zh, this message translates to:
  /// **'要保留的图片已变化或无法读取，尚未删除文件。请重新选择保留图片。'**
  String get retainedPhotoChanged;

  /// No description provided for @fileNameUnavailable.
  ///
  /// In zh, this message translates to:
  /// **'文件名不可用，请换一个名称。'**
  String get fileNameUnavailable;

  /// No description provided for @fileOperationPrepareFailed.
  ///
  /// In zh, this message translates to:
  /// **'未能准备文件操作，请重新选择后重试。'**
  String get fileOperationPrepareFailed;

  /// No description provided for @fileOperationBusy.
  ///
  /// In zh, this message translates to:
  /// **'还有文件操作正在处理，请稍后重试。'**
  String get fileOperationBusy;

  /// No description provided for @fileHistoryCheckRequired.
  ///
  /// In zh, this message translates to:
  /// **'请在文件操作记录中核对结果后再继续。'**
  String get fileHistoryCheckRequired;

  /// No description provided for @fileOperationCancelled.
  ///
  /// In zh, this message translates to:
  /// **'这次操作已取消。'**
  String get fileOperationCancelled;

  /// No description provided for @reportWriteFailed.
  ///
  /// In zh, this message translates to:
  /// **'文件操作结果已保留，但结果清单未能写入。可在记录中重新生成清单。'**
  String get reportWriteFailed;

  /// No description provided for @fileProcessingUncertain.
  ///
  /// In zh, this message translates to:
  /// **'部分文件可能尚未处理，请查看文件操作记录。'**
  String get fileProcessingUncertain;

  /// No description provided for @moreFilters.
  ///
  /// In zh, this message translates to:
  /// **'更多筛选'**
  String get moreFilters;

  /// No description provided for @filterPhotos.
  ///
  /// In zh, this message translates to:
  /// **'筛选图片'**
  String get filterPhotos;

  /// No description provided for @clearFilters.
  ///
  /// In zh, this message translates to:
  /// **'清除筛选'**
  String get clearFilters;

  /// No description provided for @cancel.
  ///
  /// In zh, this message translates to:
  /// **'取消'**
  String get cancel;

  /// No description provided for @apply.
  ///
  /// In zh, this message translates to:
  /// **'应用'**
  String get apply;

  /// No description provided for @date.
  ///
  /// In zh, this message translates to:
  /// **'日期'**
  String get date;

  /// No description provided for @dateField.
  ///
  /// In zh, this message translates to:
  /// **'日期类型'**
  String get dateField;

  /// No description provided for @startDate.
  ///
  /// In zh, this message translates to:
  /// **'开始日期'**
  String get startDate;

  /// No description provided for @endDate.
  ///
  /// In zh, this message translates to:
  /// **'结束日期'**
  String get endDate;

  /// No description provided for @fileFormat.
  ///
  /// In zh, this message translates to:
  /// **'文件格式'**
  String get fileFormat;

  /// No description provided for @fileSize.
  ///
  /// In zh, this message translates to:
  /// **'文件大小'**
  String get fileSize;

  /// No description provided for @width.
  ///
  /// In zh, this message translates to:
  /// **'宽度'**
  String get width;

  /// No description provided for @height.
  ///
  /// In zh, this message translates to:
  /// **'高度'**
  String get height;

  /// No description provided for @minimum.
  ///
  /// In zh, this message translates to:
  /// **'最小值'**
  String get minimum;

  /// No description provided for @maximum.
  ///
  /// In zh, this message translates to:
  /// **'最大值'**
  String get maximum;

  /// No description provided for @unit.
  ///
  /// In zh, this message translates to:
  /// **'单位'**
  String get unit;

  /// No description provided for @dimensions.
  ///
  /// In zh, this message translates to:
  /// **'图片尺寸'**
  String get dimensions;

  /// No description provided for @minimumWidth.
  ///
  /// In zh, this message translates to:
  /// **'最小宽度'**
  String get minimumWidth;

  /// No description provided for @maximumWidth.
  ///
  /// In zh, this message translates to:
  /// **'最大宽度'**
  String get maximumWidth;

  /// No description provided for @minimumHeight.
  ///
  /// In zh, this message translates to:
  /// **'最小高度'**
  String get minimumHeight;

  /// No description provided for @maximumHeight.
  ///
  /// In zh, this message translates to:
  /// **'最大高度'**
  String get maximumHeight;

  /// No description provided for @cameraInformation.
  ///
  /// In zh, this message translates to:
  /// **'相机信息'**
  String get cameraInformation;

  /// No description provided for @cameraHint.
  ///
  /// In zh, this message translates to:
  /// **'品牌或型号'**
  String get cameraHint;

  /// No description provided for @invalidExtension.
  ///
  /// In zh, this message translates to:
  /// **'最多选择 16 种格式，每种限 16 位字母或数字'**
  String get invalidExtension;

  /// No description provided for @invalidNumber.
  ///
  /// In zh, this message translates to:
  /// **'请输入有效的非负数'**
  String get invalidNumber;

  /// No description provided for @invalidInteger.
  ///
  /// In zh, this message translates to:
  /// **'请输入有效的非负整数'**
  String get invalidInteger;

  /// No description provided for @fractionalBytes.
  ///
  /// In zh, this message translates to:
  /// **'换算后的大小需为整数字节'**
  String get fractionalBytes;

  /// No description provided for @numberTooLarge.
  ///
  /// In zh, this message translates to:
  /// **'数值过大'**
  String get numberTooLarge;

  /// No description provided for @reversedRange.
  ///
  /// In zh, this message translates to:
  /// **'最大值不能小于最小值'**
  String get reversedRange;

  /// No description provided for @invalidDate.
  ///
  /// In zh, this message translates to:
  /// **'请输入有效日期，如 2026-01-31'**
  String get invalidDate;

  /// No description provided for @reversedDates.
  ///
  /// In zh, this message translates to:
  /// **'结束日期不能早于开始日期'**
  String get reversedDates;

  /// No description provided for @invalidCamera.
  ///
  /// In zh, this message translates to:
  /// **'请输入不超过 128 个字符的品牌或型号'**
  String get invalidCamera;

  /// No description provided for @captureDate.
  ///
  /// In zh, this message translates to:
  /// **'拍摄日期'**
  String get captureDate;

  /// No description provided for @createdDate.
  ///
  /// In zh, this message translates to:
  /// **'创建日期'**
  String get createdDate;

  /// No description provided for @modifiedDate.
  ///
  /// In zh, this message translates to:
  /// **'修改日期'**
  String get modifiedDate;

  /// No description provided for @favorites.
  ///
  /// In zh, this message translates to:
  /// **'收藏夹'**
  String get favorites;

  /// No description provided for @albums.
  ///
  /// In zh, this message translates to:
  /// **'相册'**
  String get albums;

  /// No description provided for @smartAlbums.
  ///
  /// In zh, this message translates to:
  /// **'智能相册'**
  String get smartAlbums;

  /// No description provided for @modelDownloading.
  ///
  /// In zh, this message translates to:
  /// **'正在下载智能相册模型'**
  String get modelDownloading;

  /// No description provided for @modelVerifying.
  ///
  /// In zh, this message translates to:
  /// **'正在校验模型'**
  String get modelVerifying;

  /// No description provided for @modelEnabling.
  ///
  /// In zh, this message translates to:
  /// **'正在启用智能相册'**
  String get modelEnabling;

  /// No description provided for @modelEnabled.
  ///
  /// In zh, this message translates to:
  /// **'已启用智能相册'**
  String get modelEnabled;

  /// No description provided for @modelEnableCancelled.
  ///
  /// In zh, this message translates to:
  /// **'已取消启用智能相册'**
  String get modelEnableCancelled;

  /// No description provided for @modelEnableFailed.
  ///
  /// In zh, this message translates to:
  /// **'智能相册未能启用'**
  String get modelEnableFailed;

  /// No description provided for @modelPreparing.
  ///
  /// In zh, this message translates to:
  /// **'正在准备智能相册模型'**
  String get modelPreparing;

  /// No description provided for @modelConfigUnreadable.
  ///
  /// In zh, this message translates to:
  /// **'暂时无法读取模型配置，请重试。'**
  String get modelConfigUnreadable;

  /// No description provided for @modelDownloadRetained.
  ///
  /// In zh, this message translates to:
  /// **'下载进度已保留，可以重试。'**
  String get modelDownloadRetained;

  /// No description provided for @modelEnableRetry.
  ///
  /// In zh, this message translates to:
  /// **'请重试启用，已有的完整模型文件会复用。'**
  String get modelEnableRetry;

  /// No description provided for @albumNameInvalid.
  ///
  /// In zh, this message translates to:
  /// **'请输入 1 至 128 个字符的相册名称。'**
  String get albumNameInvalid;

  /// No description provided for @albumNameTaken.
  ///
  /// In zh, this message translates to:
  /// **'已有同名相册，请换一个名字。'**
  String get albumNameTaken;

  /// No description provided for @albumMissing.
  ///
  /// In zh, this message translates to:
  /// **'这个相册已不存在，请重新选择。'**
  String get albumMissing;

  /// No description provided for @organizationDataChanged.
  ///
  /// In zh, this message translates to:
  /// **'图片或相册信息已更新，请检查后重试。'**
  String get organizationDataChanged;

  /// No description provided for @organizationPositionChanged.
  ///
  /// In zh, this message translates to:
  /// **'整理位置已改变，请重新打开这张图片。'**
  String get organizationPositionChanged;

  /// No description provided for @organizationReselectPhotos.
  ///
  /// In zh, this message translates to:
  /// **'图片或整理信息已更新，请取消后重新选择图片。'**
  String get organizationReselectPhotos;

  /// No description provided for @noActionablePhotos.
  ///
  /// In zh, this message translates to:
  /// **'没有可操作的图片，请重新选择。'**
  String get noActionablePhotos;

  /// No description provided for @previousOperationRunning.
  ///
  /// In zh, this message translates to:
  /// **'上一项操作尚未结束，请稍后重试。'**
  String get previousOperationRunning;

  /// No description provided for @chooseConflictAlbum.
  ///
  /// In zh, this message translates to:
  /// **'相册归属有冲突，请选择一个相册。'**
  String get chooseConflictAlbum;

  /// No description provided for @photoInformationUpdating.
  ///
  /// In zh, this message translates to:
  /// **'图片信息正在更新，请稍后重试。'**
  String get photoInformationUpdating;

  /// No description provided for @saveResultUnknown.
  ///
  /// In zh, this message translates to:
  /// **'尚未确认保存结果，请重试。'**
  String get saveResultUnknown;

  /// No description provided for @stoppingCurrentTask.
  ///
  /// In zh, this message translates to:
  /// **'正在停止当前任务'**
  String get stoppingCurrentTask;

  /// No description provided for @resumeOrCancelPaused.
  ///
  /// In zh, this message translates to:
  /// **'请先继续或取消已暂停的任务'**
  String get resumeOrCancelPaused;

  /// No description provided for @loadingLibrary.
  ///
  /// In zh, this message translates to:
  /// **'正在加载图库…'**
  String get loadingLibrary;

  /// No description provided for @selectingFolder.
  ///
  /// In zh, this message translates to:
  /// **'正在选择目录…'**
  String get selectingFolder;

  /// No description provided for @checkingFolderPhotos.
  ///
  /// In zh, this message translates to:
  /// **'正在检查“{name}”的图片…'**
  String checkingFolderPhotos(String name);

  /// No description provided for @updatingFolder.
  ///
  /// In zh, this message translates to:
  /// **'正在更新“{name}”…'**
  String updatingFolder(String name);

  /// No description provided for @addingFolder.
  ///
  /// In zh, this message translates to:
  /// **'正在添加“{name}”…'**
  String addingFolder(String name);

  /// No description provided for @pausingEllipsis.
  ///
  /// In zh, this message translates to:
  /// **'正在暂停…'**
  String get pausingEllipsis;

  /// No description provided for @stoppingEllipsis.
  ///
  /// In zh, this message translates to:
  /// **'正在停止…'**
  String get stoppingEllipsis;

  /// No description provided for @loadingPhotos.
  ///
  /// In zh, this message translates to:
  /// **'正在加载图片…'**
  String get loadingPhotos;

  /// No description provided for @updateLibraryCancelled.
  ///
  /// In zh, this message translates to:
  /// **'已取消更新图库'**
  String get updateLibraryCancelled;

  /// No description provided for @addFolderCancelled.
  ///
  /// In zh, this message translates to:
  /// **'已取消添加目录'**
  String get addFolderCancelled;

  /// No description provided for @updateLibraryPaused.
  ///
  /// In zh, this message translates to:
  /// **'已暂停更新图库'**
  String get updateLibraryPaused;

  /// No description provided for @addFolderPaused.
  ///
  /// In zh, this message translates to:
  /// **'已暂停添加目录'**
  String get addFolderPaused;

  /// No description provided for @filesChangedRetry.
  ///
  /// In zh, this message translates to:
  /// **'文件已变化，请重试'**
  String get filesChangedRetry;

  /// No description provided for @updateLibraryFailed.
  ///
  /// In zh, this message translates to:
  /// **'更新图库失败'**
  String get updateLibraryFailed;

  /// No description provided for @addFolderFailed.
  ///
  /// In zh, this message translates to:
  /// **'添加目录失败'**
  String get addFolderFailed;

  /// No description provided for @updateComplete.
  ///
  /// In zh, this message translates to:
  /// **'更新完成'**
  String get updateComplete;

  /// No description provided for @addComplete.
  ///
  /// In zh, this message translates to:
  /// **'添加完成'**
  String get addComplete;

  /// No description provided for @totalPhotos.
  ///
  /// In zh, this message translates to:
  /// **'共 {count} 张图片'**
  String totalPhotos(int count);

  /// No description provided for @photosAdded.
  ///
  /// In zh, this message translates to:
  /// **'已添加 {count} 张图片'**
  String photosAdded(int count);

  /// No description provided for @photosValidated.
  ///
  /// In zh, this message translates to:
  /// **'已检查 {checked} / {total} 张图片 · 已检查 {files} 个文件'**
  String photosValidated(int checked, int total, int files);

  /// No description provided for @filesCheckedPhotosFound.
  ///
  /// In zh, this message translates to:
  /// **'已检查 {files} 个文件 · 已找到 {photos} 张图片'**
  String filesCheckedPhotosFound(int files, int photos);

  /// No description provided for @chooseFolderAgain.
  ///
  /// In zh, this message translates to:
  /// **'重新选择文件夹'**
  String get chooseFolderAgain;

  /// No description provided for @retryRemoval.
  ///
  /// In zh, this message translates to:
  /// **'重试移除'**
  String get retryRemoval;

  /// No description provided for @photos.
  ///
  /// In zh, this message translates to:
  /// **'图片'**
  String get photos;

  /// No description provided for @removeFromLater.
  ///
  /// In zh, this message translates to:
  /// **'取消稍后整理'**
  String get removeFromLater;

  /// No description provided for @organizeLater.
  ///
  /// In zh, this message translates to:
  /// **'稍后整理'**
  String get organizeLater;

  /// No description provided for @unignore.
  ///
  /// In zh, this message translates to:
  /// **'取消忽略'**
  String get unignore;

  /// No description provided for @ignoreSelected.
  ///
  /// In zh, this message translates to:
  /// **'忽略所选'**
  String get ignoreSelected;

  /// No description provided for @ignorePhoto.
  ///
  /// In zh, this message translates to:
  /// **'忽略此图'**
  String get ignorePhoto;

  /// No description provided for @photoAnalysisPaused.
  ///
  /// In zh, this message translates to:
  /// **'图片分析已暂停'**
  String get photoAnalysisPaused;

  /// No description provided for @searchUnavailable.
  ///
  /// In zh, this message translates to:
  /// **'搜索暂时不可用'**
  String get searchUnavailable;

  /// No description provided for @retryAnalysis.
  ///
  /// In zh, this message translates to:
  /// **'重试分析'**
  String get retryAnalysis;

  /// No description provided for @searchContent.
  ///
  /// In zh, this message translates to:
  /// **'画面内容'**
  String get searchContent;

  /// No description provided for @searchNamePath.
  ///
  /// In zh, this message translates to:
  /// **'文件名和路径'**
  String get searchNamePath;

  /// No description provided for @searchCurrentScope.
  ///
  /// In zh, this message translates to:
  /// **'当前范围'**
  String get searchCurrentScope;

  /// No description provided for @searchEntireLibrary.
  ///
  /// In zh, this message translates to:
  /// **'整个图库'**
  String get searchEntireLibrary;

  /// No description provided for @findRelatedPhotos.
  ///
  /// In zh, this message translates to:
  /// **'查找相关图片'**
  String get findRelatedPhotos;

  /// No description provided for @updateSearchResults.
  ///
  /// In zh, this message translates to:
  /// **'更新搜索结果'**
  String get updateSearchResults;

  /// No description provided for @retrySearch.
  ///
  /// In zh, this message translates to:
  /// **'重试搜索'**
  String get retrySearch;

  /// No description provided for @contentSearchHint.
  ///
  /// In zh, this message translates to:
  /// **'描述要找的图片'**
  String get contentSearchHint;

  /// No description provided for @fileSearchHint.
  ///
  /// In zh, this message translates to:
  /// **'搜索文件名或路径'**
  String get fileSearchHint;

  /// No description provided for @contentSearchEmptyHint.
  ///
  /// In zh, this message translates to:
  /// **'可以换个描述、调整筛选，或搜索整个图库。'**
  String get contentSearchEmptyHint;

  /// No description provided for @filterSearchEmptyHint.
  ///
  /// In zh, this message translates to:
  /// **'可以调整或清除筛选。'**
  String get filterSearchEmptyHint;

  /// No description provided for @searchNeedsModel.
  ///
  /// In zh, this message translates to:
  /// **'请先在设置中安装图片分析模型。'**
  String get searchNeedsModel;

  /// No description provided for @searchDescriptionTooLong.
  ///
  /// In zh, this message translates to:
  /// **'请输入不超过 512 个字符的描述。'**
  String get searchDescriptionTooLong;

  /// No description provided for @searchPartialAnalysis.
  ///
  /// In zh, this message translates to:
  /// **'部分图片尚未完成分析，已保留可用的搜索结果。请重试分析。'**
  String get searchPartialAnalysis;

  /// No description provided for @searchLibraryChanged.
  ///
  /// In zh, this message translates to:
  /// **'图库刚有更新，请重试搜索。'**
  String get searchLibraryChanged;

  /// No description provided for @searchPhotoChanged.
  ///
  /// In zh, this message translates to:
  /// **'这张图片已变化或无法读取，请重新选择。'**
  String get searchPhotoChanged;

  /// No description provided for @searchAnalysisUnavailable.
  ///
  /// In zh, this message translates to:
  /// **'图片分析当前不可用，请重试搜索。'**
  String get searchAnalysisUnavailable;

  /// No description provided for @searchIndexDamaged.
  ///
  /// In zh, this message translates to:
  /// **'搜索数据已损坏，请重试以重新建立。'**
  String get searchIndexDamaged;

  /// No description provided for @searchStorageUnavailable.
  ///
  /// In zh, this message translates to:
  /// **'暂时无法读写搜索数据，请重试。'**
  String get searchStorageUnavailable;

  /// No description provided for @searchFailedRetained.
  ///
  /// In zh, this message translates to:
  /// **'暂时无法搜索，已保留原来的图片列表。请重试。'**
  String get searchFailedRetained;

  /// No description provided for @searchCoverage.
  ///
  /// In zh, this message translates to:
  /// **'按相关度排列 · 已分析 {indexed}/{total} 张{status}'**
  String searchCoverage(int indexed, int total, String status);

  /// No description provided for @analysisStorageFullSuffix.
  ///
  /// In zh, this message translates to:
  /// **' · 分析存储已满'**
  String get analysisStorageFullSuffix;

  /// No description provided for @partialResults.
  ///
  /// In zh, this message translates to:
  /// **'部分结果'**
  String get partialResults;

  /// No description provided for @showFilmstrip.
  ///
  /// In zh, this message translates to:
  /// **'显示缩略图'**
  String get showFilmstrip;

  /// No description provided for @hideFilmstrip.
  ///
  /// In zh, this message translates to:
  /// **'隐藏缩略图'**
  String get hideFilmstrip;

  /// No description provided for @scrollFilmstripLeft.
  ///
  /// In zh, this message translates to:
  /// **'向左滚动'**
  String get scrollFilmstripLeft;

  /// No description provided for @scrollFilmstripRight.
  ///
  /// In zh, this message translates to:
  /// **'向右滚动'**
  String get scrollFilmstripRight;

  /// No description provided for @fitImageToWindow.
  ///
  /// In zh, this message translates to:
  /// **'适应窗口（0 / Ctrl+0）'**
  String get fitImageToWindow;

  /// No description provided for @showImageActualSize.
  ///
  /// In zh, this message translates to:
  /// **'实际大小（1 / Ctrl+1）'**
  String get showImageActualSize;

  /// No description provided for @enterViewerFullscreen.
  ///
  /// In zh, this message translates to:
  /// **'全屏（F11）'**
  String get enterViewerFullscreen;

  /// No description provided for @exitViewerFullscreen.
  ///
  /// In zh, this message translates to:
  /// **'退出全屏（Esc / F11）'**
  String get exitViewerFullscreen;

  /// No description provided for @clearSearch.
  ///
  /// In zh, this message translates to:
  /// **'清除搜索'**
  String get clearSearch;

  /// No description provided for @library.
  ///
  /// In zh, this message translates to:
  /// **'图库'**
  String get library;

  /// No description provided for @allImages.
  ///
  /// In zh, this message translates to:
  /// **'所有图片'**
  String get allImages;

  /// No description provided for @addFolder.
  ///
  /// In zh, this message translates to:
  /// **'添加目录到图库'**
  String get addFolder;

  /// No description provided for @noFolder.
  ///
  /// In zh, this message translates to:
  /// **'还没有添加目录'**
  String get noFolder;

  /// No description provided for @expandFolder.
  ///
  /// In zh, this message translates to:
  /// **'展开目录'**
  String get expandFolder;

  /// No description provided for @collapseFolder.
  ///
  /// In zh, this message translates to:
  /// **'折叠目录'**
  String get collapseFolder;

  /// No description provided for @loadingFolders.
  ///
  /// In zh, this message translates to:
  /// **'正在加载目录…'**
  String get loadingFolders;

  /// No description provided for @showMoreFolders.
  ///
  /// In zh, this message translates to:
  /// **'显示更多目录'**
  String get showMoreFolders;

  /// No description provided for @retryFolders.
  ///
  /// In zh, this message translates to:
  /// **'重新加载目录'**
  String get retryFolders;

  /// No description provided for @synchronized.
  ///
  /// In zh, this message translates to:
  /// **'已同步'**
  String get synchronized;

  /// No description provided for @synchronizing.
  ///
  /// In zh, this message translates to:
  /// **'正在更新'**
  String get synchronizing;

  /// No description provided for @backgroundVerification.
  ///
  /// In zh, this message translates to:
  /// **'正在核对'**
  String get backgroundVerification;

  /// No description provided for @needsReconciliation.
  ///
  /// In zh, this message translates to:
  /// **'更新受阻'**
  String get needsReconciliation;

  /// No description provided for @synchronizationUnconfirmed.
  ///
  /// In zh, this message translates to:
  /// **'尚未核对'**
  String get synchronizationUnconfirmed;

  /// No description provided for @synchronizationRetryPending.
  ///
  /// In zh, this message translates to:
  /// **'等待重试'**
  String get synchronizationRetryPending;

  /// No description provided for @continuityLiveOnly.
  ///
  /// In zh, this message translates to:
  /// **'自动更新不可用'**
  String get continuityLiveOnly;

  /// No description provided for @continuityUnavailable.
  ///
  /// In zh, this message translates to:
  /// **'无法自动更新'**
  String get continuityUnavailable;

  /// No description provided for @sourceUnavailable.
  ///
  /// In zh, this message translates to:
  /// **'无法访问'**
  String get sourceUnavailable;

  /// No description provided for @sourceAvailable.
  ///
  /// In zh, this message translates to:
  /// **'可用'**
  String get sourceAvailable;

  /// No description provided for @sourceMissing.
  ///
  /// In zh, this message translates to:
  /// **'目录不存在'**
  String get sourceMissing;

  /// No description provided for @sourceOffline.
  ///
  /// In zh, this message translates to:
  /// **'当前离线'**
  String get sourceOffline;

  /// No description provided for @sourceUnknown.
  ///
  /// In zh, this message translates to:
  /// **'状态未知'**
  String get sourceUnknown;

  /// No description provided for @addingSource.
  ///
  /// In zh, this message translates to:
  /// **'正在添加'**
  String get addingSource;

  /// No description provided for @refreshingSourceDisplay.
  ///
  /// In zh, this message translates to:
  /// **'正在刷新'**
  String get refreshingSourceDisplay;

  /// No description provided for @sourceAdded.
  ///
  /// In zh, this message translates to:
  /// **'已添加'**
  String get sourceAdded;

  /// No description provided for @sourceDisplayReloadPending.
  ///
  /// In zh, this message translates to:
  /// **'需要重新加载'**
  String get sourceDisplayReloadPending;

  /// No description provided for @sourceScanResultReadFailed.
  ///
  /// In zh, this message translates to:
  /// **'读取结果失败'**
  String get sourceScanResultReadFailed;

  /// No description provided for @sourceReadingScanResult.
  ///
  /// In zh, this message translates to:
  /// **'正在读取结果'**
  String get sourceReadingScanResult;

  /// No description provided for @layout.
  ///
  /// In zh, this message translates to:
  /// **'布局'**
  String get layout;

  /// No description provided for @sort.
  ///
  /// In zh, this message translates to:
  /// **'排序'**
  String get sort;

  /// No description provided for @fileName.
  ///
  /// In zh, this message translates to:
  /// **'名称'**
  String get fileName;

  /// No description provided for @newestAddedFirst.
  ///
  /// In zh, this message translates to:
  /// **'新加入优先'**
  String get newestAddedFirst;

  /// No description provided for @ascending.
  ///
  /// In zh, this message translates to:
  /// **'升序'**
  String get ascending;

  /// No description provided for @descending.
  ///
  /// In zh, this message translates to:
  /// **'降序'**
  String get descending;

  /// No description provided for @equalHeight.
  ///
  /// In zh, this message translates to:
  /// **'等高'**
  String get equalHeight;

  /// No description provided for @square.
  ///
  /// In zh, this message translates to:
  /// **'方形'**
  String get square;

  /// No description provided for @sizeMediumLong.
  ///
  /// In zh, this message translates to:
  /// **'中等'**
  String get sizeMediumLong;

  /// No description provided for @more.
  ///
  /// In zh, this message translates to:
  /// **'更多'**
  String get more;

  /// No description provided for @selectAll.
  ///
  /// In zh, this message translates to:
  /// **'全选'**
  String get selectAll;

  /// No description provided for @unknownCaptureDate.
  ///
  /// In zh, this message translates to:
  /// **'拍摄日期未知'**
  String get unknownCaptureDate;

  /// No description provided for @open.
  ///
  /// In zh, this message translates to:
  /// **'打开'**
  String get open;

  /// No description provided for @viewInformation.
  ///
  /// In zh, this message translates to:
  /// **'查看信息'**
  String get viewInformation;

  /// No description provided for @copyPath.
  ///
  /// In zh, this message translates to:
  /// **'复制路径'**
  String get copyPath;

  /// No description provided for @updateLibrary.
  ///
  /// In zh, this message translates to:
  /// **'更新图库'**
  String get updateLibrary;

  /// No description provided for @relocateLibrary.
  ///
  /// In zh, this message translates to:
  /// **'重新定位目录'**
  String get relocateLibrary;

  /// No description provided for @removeFromAme.
  ///
  /// In zh, this message translates to:
  /// **'从 Ame 中移除'**
  String get removeFromAme;

  /// No description provided for @updatingLibrary.
  ///
  /// In zh, this message translates to:
  /// **'正在更新图库…'**
  String get updatingLibrary;

  /// No description provided for @emptyLibraryTitle.
  ///
  /// In zh, this message translates to:
  /// **'建立你的图库'**
  String get emptyLibraryTitle;

  /// No description provided for @emptyLibraryBody.
  ///
  /// In zh, this message translates to:
  /// **'选择要在 Ame 中一起浏览的图片吧。'**
  String get emptyLibraryBody;

  /// No description provided for @importPhotos.
  ///
  /// In zh, this message translates to:
  /// **'导入'**
  String get importPhotos;

  /// No description provided for @firstImportIncomplete.
  ///
  /// In zh, this message translates to:
  /// **'尚未完成添加'**
  String get firstImportIncomplete;

  /// No description provided for @firstImportInProgress.
  ///
  /// In zh, this message translates to:
  /// **'正在添加图片'**
  String get firstImportInProgress;

  /// No description provided for @firstImportInProgressDetail.
  ///
  /// In zh, this message translates to:
  /// **'添加完成后，图片就会显示在这里。'**
  String get firstImportInProgressDetail;

  /// No description provided for @pausingSource.
  ///
  /// In zh, this message translates to:
  /// **'正在暂停'**
  String get pausingSource;

  /// No description provided for @firstImportPaused.
  ///
  /// In zh, this message translates to:
  /// **'已暂停添加'**
  String get firstImportPaused;

  /// No description provided for @firstImportPausedDetail.
  ///
  /// In zh, this message translates to:
  /// **'进度已保存，可以稍后继续。'**
  String get firstImportPausedDetail;

  /// No description provided for @cancellingSource.
  ///
  /// In zh, this message translates to:
  /// **'正在取消'**
  String get cancellingSource;

  /// No description provided for @firstImportCancelled.
  ///
  /// In zh, this message translates to:
  /// **'已取消添加'**
  String get firstImportCancelled;

  /// No description provided for @firstImportRetryDetail.
  ///
  /// In zh, this message translates to:
  /// **'这个目录暂时无法添加到图库。'**
  String get firstImportRetryDetail;

  /// No description provided for @firstImportGalleryDetail.
  ///
  /// In zh, this message translates to:
  /// **'需要时可以重新添加这个目录。'**
  String get firstImportGalleryDetail;

  /// No description provided for @scanConfirmationPending.
  ///
  /// In zh, this message translates to:
  /// **'扫描结果读取失败'**
  String get scanConfirmationPending;

  /// No description provided for @scanConfirmationDetail.
  ///
  /// In zh, this message translates to:
  /// **'暂时无法显示扫描到的图片。'**
  String get scanConfirmationDetail;

  /// No description provided for @readingScanResult.
  ///
  /// In zh, this message translates to:
  /// **'正在读取扫描结果…'**
  String get readingScanResult;

  /// No description provided for @rereadScanResult.
  ///
  /// In zh, this message translates to:
  /// **'重新读取'**
  String get rereadScanResult;

  /// No description provided for @galleryDisplayReloadPending.
  ///
  /// In zh, this message translates to:
  /// **'图片加载失败'**
  String get galleryDisplayReloadPending;

  /// No description provided for @galleryDisplayReloadDetail.
  ///
  /// In zh, this message translates to:
  /// **'图片已添加，暂时无法显示。'**
  String get galleryDisplayReloadDetail;

  /// No description provided for @addFolderAwaitingScanResult.
  ///
  /// In zh, this message translates to:
  /// **'扫描结果读取失败，暂时无法添加目录。'**
  String get addFolderAwaitingScanResult;

  /// No description provided for @addFolderAwaitingGalleryReload.
  ///
  /// In zh, this message translates to:
  /// **'图库加载失败，暂时无法添加目录。'**
  String get addFolderAwaitingGalleryReload;

  /// No description provided for @noSearchResults.
  ///
  /// In zh, this message translates to:
  /// **'没有找到匹配的图片'**
  String get noSearchResults;

  /// No description provided for @noSearchResultsHint.
  ///
  /// In zh, this message translates to:
  /// **'请尝试其他名称或路径。'**
  String get noSearchResultsHint;

  /// No description provided for @noSourceResults.
  ///
  /// In zh, this message translates to:
  /// **'此目录中没有可显示的图片'**
  String get noSourceResults;

  /// No description provided for @noSourceResultsHint.
  ///
  /// In zh, this message translates to:
  /// **'可以选择其他目录，或返回图库。'**
  String get noSourceResultsHint;

  /// No description provided for @retryLoading.
  ///
  /// In zh, this message translates to:
  /// **'重新加载'**
  String get retryLoading;

  /// No description provided for @retryPreview.
  ///
  /// In zh, this message translates to:
  /// **'重新加载预览'**
  String get retryPreview;

  /// No description provided for @retryingPreview.
  ///
  /// In zh, this message translates to:
  /// **'正在加载预览…'**
  String get retryingPreview;

  /// No description provided for @previewFormatUnrecognized.
  ///
  /// In zh, this message translates to:
  /// **'无法识别图片'**
  String get previewFormatUnrecognized;

  /// No description provided for @previewUpdateRequired.
  ///
  /// In zh, this message translates to:
  /// **'请先更新图库，再重新加载预览。'**
  String get previewUpdateRequired;

  /// No description provided for @backToLibrary.
  ///
  /// In zh, this message translates to:
  /// **'返回图库'**
  String get backToLibrary;

  /// No description provided for @originalImageUnavailable.
  ///
  /// In zh, this message translates to:
  /// **'无法打开原图'**
  String get originalImageUnavailable;

  /// No description provided for @originalImagePreviewFallback.
  ///
  /// In zh, this message translates to:
  /// **'原图无法打开，已暂时显示预览图'**
  String get originalImagePreviewFallback;

  /// No description provided for @retryOriginalImage.
  ///
  /// In zh, this message translates to:
  /// **'重新打开'**
  String get retryOriginalImage;

  /// No description provided for @viewerSourceChanged.
  ///
  /// In zh, this message translates to:
  /// **'图片已更新，重新加载以查看最新内容。'**
  String get viewerSourceChanged;

  /// No description provided for @viewerSourceMissing.
  ///
  /// In zh, this message translates to:
  /// **'这张图片已不在图库中。'**
  String get viewerSourceMissing;

  /// No description provided for @removingFromAme.
  ///
  /// In zh, this message translates to:
  /// **'正在从 Ame 中移除“{name}”…'**
  String removingFromAme(String name);

  /// No description provided for @removingFolderDetail.
  ///
  /// In zh, this message translates to:
  /// **'原目录和图片都会保留。'**
  String get removingFolderDetail;

  /// No description provided for @removeFolderFailed.
  ///
  /// In zh, this message translates to:
  /// **'移除目录失败'**
  String get removeFolderFailed;

  /// No description provided for @refreshingAfterRemoval.
  ///
  /// In zh, this message translates to:
  /// **'已移除“{name}”，正在重新加载…'**
  String refreshingAfterRemoval(String name);

  /// No description provided for @refreshingAfterRemovalDetail.
  ///
  /// In zh, this message translates to:
  /// **'原目录和图片已保留。'**
  String get refreshingAfterRemovalDetail;

  /// No description provided for @removedFolderRefreshFailed.
  ///
  /// In zh, this message translates to:
  /// **'目录已移除，图片加载失败'**
  String get removedFolderRefreshFailed;

  /// No description provided for @scanOverlapsStorage.
  ///
  /// In zh, this message translates to:
  /// **'这个文件夹与 Ame 的数据存储位置重叠，请选择其他文件夹。'**
  String get scanOverlapsStorage;

  /// No description provided for @scanBusy.
  ///
  /// In zh, this message translates to:
  /// **'另一个目录正在更新，结束后再继续。'**
  String get scanBusy;

  /// No description provided for @scanCancelFailed.
  ///
  /// In zh, this message translates to:
  /// **'取消失败，请重试。'**
  String get scanCancelFailed;

  /// No description provided for @parentScanIncomplete.
  ///
  /// In zh, this message translates to:
  /// **'上级目录没有扫描完整。请检查目录能否打开，以及扫描数量是否达到上限。已添加的子目录仍会保留。'**
  String get parentScanIncomplete;

  /// No description provided for @childScanPending.
  ///
  /// In zh, this message translates to:
  /// **'子目录还有未完成的扫描。请先继续或取消该任务，再添加父目录。'**
  String get childScanPending;

  /// No description provided for @childUpdateRequired.
  ///
  /// In zh, this message translates to:
  /// **'先更新已添加的子目录，再添加上级目录。'**
  String get childUpdateRequired;

  /// No description provided for @childMergeFailed.
  ///
  /// In zh, this message translates to:
  /// **'无法合并已添加的子目录。请检查目录位置后重试。'**
  String get childMergeFailed;

  /// No description provided for @scanFolderFailed.
  ///
  /// In zh, this message translates to:
  /// **'无法更新这个目录，请检查它能否打开。'**
  String get scanFolderFailed;

  /// No description provided for @synchronizationRefreshFailureMessage.
  ///
  /// In zh, this message translates to:
  /// **'重新加载以查看最新图片。'**
  String get synchronizationRefreshFailureMessage;

  /// No description provided for @manualUpdateNotice.
  ///
  /// In zh, this message translates to:
  /// **'部分目录需要手动更新'**
  String get manualUpdateNotice;

  /// No description provided for @manualUpdateDetail.
  ///
  /// In zh, this message translates to:
  /// **'重新打开 Ame 后，在目录菜单中选择“更新图库”，补上关闭期间的变化。'**
  String get manualUpdateDetail;

  /// No description provided for @synchronizationBlockedFolder.
  ///
  /// In zh, this message translates to:
  /// **'“{name}”更新受阻'**
  String synchronizationBlockedFolder(String name);

  /// No description provided for @synchronizationUncheckedFolder.
  ///
  /// In zh, this message translates to:
  /// **'“{name}”图库尚未核对'**
  String synchronizationUncheckedFolder(String name);

  /// No description provided for @firstImportRequiredFolder.
  ///
  /// In zh, this message translates to:
  /// **'“{name}”尚未完成添加'**
  String firstImportRequiredFolder(String name);

  /// No description provided for @synchronizationSourceUnavailable.
  ///
  /// In zh, this message translates to:
  /// **'打不开这个目录。'**
  String get synchronizationSourceUnavailable;

  /// No description provided for @synchronizationSourceUnhealthy.
  ///
  /// In zh, this message translates to:
  /// **'自动更新已中断。'**
  String get synchronizationSourceUnhealthy;

  /// No description provided for @synchronizationSourceStarting.
  ///
  /// In zh, this message translates to:
  /// **'正在准备自动更新。'**
  String get synchronizationSourceStarting;

  /// No description provided for @synchronizationEvidenceGap.
  ///
  /// In zh, this message translates to:
  /// **'部分文件变化还没有更新到图库。'**
  String get synchronizationEvidenceGap;

  /// No description provided for @synchronizationCapacityExceeded.
  ///
  /// In zh, this message translates to:
  /// **'文件变化较多，图库还未更新完成。'**
  String get synchronizationCapacityExceeded;

  /// No description provided for @synchronizationNeedsReconciliation.
  ///
  /// In zh, this message translates to:
  /// **'图库还没有完成更新。'**
  String get synchronizationNeedsReconciliation;

  /// No description provided for @synchronizationCompleted.
  ///
  /// In zh, this message translates to:
  /// **'目录中的变化已更新到图库。'**
  String get synchronizationCompleted;

  /// No description provided for @synchronizationAutomaticRetry.
  ///
  /// In zh, this message translates to:
  /// **'部分内容还未更新，稍后会自动重试。'**
  String get synchronizationAutomaticRetry;

  /// No description provided for @synchronizationRecoveryBlocked.
  ///
  /// In zh, this message translates to:
  /// **'在目录菜单点击“更新图库”重试。'**
  String get synchronizationRecoveryBlocked;

  /// No description provided for @synchronizationLegacyRecoveryAuthorityMissing.
  ///
  /// In zh, this message translates to:
  /// **'在目录菜单点击“更新图库”，恢复自动更新。'**
  String get synchronizationLegacyRecoveryAuthorityMissing;

  /// No description provided for @synchronizationMonitoringAccessDenied.
  ///
  /// In zh, this message translates to:
  /// **'没有权限读取这个目录的变化，请检查访问权限。'**
  String get synchronizationMonitoringAccessDenied;

  /// No description provided for @synchronizationMonitoringEventIncomplete.
  ///
  /// In zh, this message translates to:
  /// **'无法读取部分文件的变化。'**
  String get synchronizationMonitoringEventIncomplete;

  /// No description provided for @synchronizationRecoveryFailed.
  ///
  /// In zh, this message translates to:
  /// **'目录检查未完成。'**
  String get synchronizationRecoveryFailed;

  /// No description provided for @synchronizationPersistenceFailed.
  ///
  /// In zh, this message translates to:
  /// **'图库更新未能保存。'**
  String get synchronizationPersistenceFailed;

  /// No description provided for @synchronizationPreparing.
  ///
  /// In zh, this message translates to:
  /// **'准备更新'**
  String get synchronizationPreparing;

  /// No description provided for @synchronizationCheckingFolder.
  ///
  /// In zh, this message translates to:
  /// **'正在检查目录'**
  String get synchronizationCheckingFolder;

  /// No description provided for @synchronizationCheckingChanges.
  ///
  /// In zh, this message translates to:
  /// **'正在检查变化'**
  String get synchronizationCheckingChanges;

  /// No description provided for @synchronizationUpdatingLibrary.
  ///
  /// In zh, this message translates to:
  /// **'正在更新图库'**
  String get synchronizationUpdatingLibrary;

  /// No description provided for @synchronizationRetryLater.
  ///
  /// In zh, this message translates to:
  /// **'稍后自动重试'**
  String get synchronizationRetryLater;

  /// No description provided for @synchronizationScanningFolder.
  ///
  /// In zh, this message translates to:
  /// **'正在扫描目录'**
  String get synchronizationScanningFolder;

  /// No description provided for @synchronizationUpdateIncomplete.
  ///
  /// In zh, this message translates to:
  /// **'更新未完成'**
  String get synchronizationUpdateIncomplete;

  /// No description provided for @synchronizationComplete.
  ///
  /// In zh, this message translates to:
  /// **'同步完成'**
  String get synchronizationComplete;

  /// No description provided for @synchronizationWaitingFolder.
  ///
  /// In zh, this message translates to:
  /// **'等待目录恢复'**
  String get synchronizationWaitingFolder;

  /// No description provided for @libraryUpdateBlocked.
  ///
  /// In zh, this message translates to:
  /// **'图库更新受阻'**
  String get libraryUpdateBlocked;

  /// No description provided for @pendingChanges.
  ///
  /// In zh, this message translates to:
  /// **'{count} 项等待处理'**
  String pendingChanges(String count);

  /// No description provided for @retryingChanges.
  ///
  /// In zh, this message translates to:
  /// **'{count} 项等待重试'**
  String retryingChanges(String count);

  /// No description provided for @incompleteChanges.
  ///
  /// In zh, this message translates to:
  /// **'{count} 项更新尚未完成'**
  String incompleteChanges(String count);

  /// No description provided for @changesToCheck.
  ///
  /// In zh, this message translates to:
  /// **'还有文件变化待检查'**
  String get changesToCheck;

  /// No description provided for @waitingForUpdate.
  ///
  /// In zh, this message translates to:
  /// **'等待更新'**
  String get waitingForUpdate;

  /// No description provided for @checkingPhotos.
  ///
  /// In zh, this message translates to:
  /// **'正在检查图片'**
  String get checkingPhotos;

  /// No description provided for @loadingPhotosShort.
  ///
  /// In zh, this message translates to:
  /// **'正在加载图片'**
  String get loadingPhotosShort;

  /// No description provided for @cancelled.
  ///
  /// In zh, this message translates to:
  /// **'已取消'**
  String get cancelled;

  /// No description provided for @filesChanged.
  ///
  /// In zh, this message translates to:
  /// **'文件已变化'**
  String get filesChanged;

  /// No description provided for @scanCompleted.
  ///
  /// In zh, this message translates to:
  /// **'扫描已完成。'**
  String get scanCompleted;

  /// No description provided for @reloadRoot.
  ///
  /// In zh, this message translates to:
  /// **'重新加载 {name}'**
  String reloadRoot(String name);

  /// No description provided for @updateRoot.
  ///
  /// In zh, this message translates to:
  /// **'更新图库 {name}'**
  String updateRoot(String name);

  /// No description provided for @retryRoot.
  ///
  /// In zh, this message translates to:
  /// **'重试更新 {name}'**
  String retryRoot(String name);

  /// No description provided for @progressPhotos.
  ///
  /// In zh, this message translates to:
  /// **'{phase}，{checked} / {total} 张图片，{percent}%'**
  String progressPhotos(String phase, int checked, int total, int percent);

  /// No description provided for @progressFiles.
  ///
  /// In zh, this message translates to:
  /// **'{phase}，已检查 {files} 个文件，已找到 {photos} 张图片'**
  String progressFiles(String phase, int files, int photos);

  /// No description provided for @duplicates.
  ///
  /// In zh, this message translates to:
  /// **'去重'**
  String get duplicates;

  /// No description provided for @collapseSidebar.
  ///
  /// In zh, this message translates to:
  /// **'收起侧栏'**
  String get collapseSidebar;

  /// No description provided for @expandSidebar.
  ///
  /// In zh, this message translates to:
  /// **'展开侧栏'**
  String get expandSidebar;

  /// No description provided for @saveSidebarFailed.
  ///
  /// In zh, this message translates to:
  /// **'无法保存侧栏设置'**
  String get saveSidebarFailed;

  /// No description provided for @changePhotoFailed.
  ///
  /// In zh, this message translates to:
  /// **'无法切换图片'**
  String get changePhotoFailed;

  /// No description provided for @removedFolders.
  ///
  /// In zh, this message translates to:
  /// **'已从 Ame 中移除 {count} 个文件夹'**
  String removedFolders(int count);

  /// No description provided for @saveGallerySettingsFailed.
  ///
  /// In zh, this message translates to:
  /// **'无法保存图库显示设置'**
  String get saveGallerySettingsFailed;

  /// No description provided for @filePathCopied.
  ///
  /// In zh, this message translates to:
  /// **'已复制文件路径'**
  String get filePathCopied;

  /// No description provided for @copyPathFailed.
  ///
  /// In zh, this message translates to:
  /// **'无法复制路径'**
  String get copyPathFailed;

  /// No description provided for @confirmRemoveFolder.
  ///
  /// In zh, this message translates to:
  /// **'从 Ame 中移除目录？'**
  String get confirmRemoveFolder;

  /// No description provided for @confirmRemoveFolderDetail.
  ///
  /// In zh, this message translates to:
  /// **'“{name}”中的图片将不再显示在 Ame 中。原目录和图片都会保留。'**
  String confirmRemoveFolderDetail(String name);

  /// No description provided for @removedFolder.
  ///
  /// In zh, this message translates to:
  /// **'已从 Ame 中移除“{name}”'**
  String removedFolder(String name);

  /// No description provided for @seekDateFailed.
  ///
  /// In zh, this message translates to:
  /// **'无法跳转到所选日期'**
  String get seekDateFailed;

  /// No description provided for @scrollUp.
  ///
  /// In zh, this message translates to:
  /// **'向上滚动'**
  String get scrollUp;

  /// No description provided for @scrollPosition.
  ///
  /// In zh, this message translates to:
  /// **'滚动位置 {percent}%'**
  String scrollPosition(int percent);

  /// No description provided for @scrollDown.
  ///
  /// In zh, this message translates to:
  /// **'向下滚动'**
  String get scrollDown;

  /// No description provided for @createAlbum.
  ///
  /// In zh, this message translates to:
  /// **'新建相册'**
  String get createAlbum;

  /// No description provided for @renameAlbum.
  ///
  /// In zh, this message translates to:
  /// **'重命名相册'**
  String get renameAlbum;

  /// No description provided for @changeAlbumIcon.
  ///
  /// In zh, this message translates to:
  /// **'更改 {name} 的图标'**
  String changeAlbumIcon(String name);

  /// No description provided for @confirmDeleteAlbum.
  ///
  /// In zh, this message translates to:
  /// **'删除相册？'**
  String get confirmDeleteAlbum;

  /// No description provided for @save.
  ///
  /// In zh, this message translates to:
  /// **'保存'**
  String get save;

  /// No description provided for @saving.
  ///
  /// In zh, this message translates to:
  /// **'正在保存'**
  String get saving;

  /// No description provided for @deleteAlbumImpact.
  ///
  /// In zh, this message translates to:
  /// **'其中的图片组将进入已忽略。图片文件和收藏状态会保留。'**
  String get deleteAlbumImpact;

  /// No description provided for @albumName.
  ///
  /// In zh, this message translates to:
  /// **'相册名称'**
  String get albumName;

  /// No description provided for @chooseAlbumIcon.
  ///
  /// In zh, this message translates to:
  /// **'选择相册图标'**
  String get chooseAlbumIcon;

  /// No description provided for @albumIconTravel.
  ///
  /// In zh, this message translates to:
  /// **'旅行'**
  String get albumIconTravel;

  /// No description provided for @albumIconNature.
  ///
  /// In zh, this message translates to:
  /// **'自然'**
  String get albumIconNature;

  /// No description provided for @albumIconPets.
  ///
  /// In zh, this message translates to:
  /// **'宠物'**
  String get albumIconPets;

  /// No description provided for @albumIconPeople.
  ///
  /// In zh, this message translates to:
  /// **'人物'**
  String get albumIconPeople;

  /// No description provided for @albumIconFood.
  ///
  /// In zh, this message translates to:
  /// **'美食'**
  String get albumIconFood;

  /// No description provided for @albumIconArt.
  ///
  /// In zh, this message translates to:
  /// **'艺术'**
  String get albumIconArt;

  /// No description provided for @albumIconWork.
  ///
  /// In zh, this message translates to:
  /// **'工作'**
  String get albumIconWork;

  /// No description provided for @albumIconStudy.
  ///
  /// In zh, this message translates to:
  /// **'学习'**
  String get albumIconStudy;

  /// No description provided for @albumIconGames.
  ///
  /// In zh, this message translates to:
  /// **'游戏'**
  String get albumIconGames;

  /// No description provided for @albumIconMusic.
  ///
  /// In zh, this message translates to:
  /// **'音乐'**
  String get albumIconMusic;

  /// No description provided for @albumIconMemories.
  ///
  /// In zh, this message translates to:
  /// **'纪念'**
  String get albumIconMemories;

  /// No description provided for @confirmDeleteAlbums.
  ///
  /// In zh, this message translates to:
  /// **'删除 {count} 个相册？'**
  String confirmDeleteAlbums(int count);

  /// No description provided for @deleteAlbumsImpact.
  ///
  /// In zh, this message translates to:
  /// **'这些相册中的图片组将进入已忽略。图片文件和收藏状态会保留。'**
  String get deleteAlbumsImpact;

  /// No description provided for @deleteAlbum.
  ///
  /// In zh, this message translates to:
  /// **'删除相册'**
  String get deleteAlbum;

  /// No description provided for @selectPageAlbums.
  ///
  /// In zh, this message translates to:
  /// **'全选本页相册'**
  String get selectPageAlbums;

  /// No description provided for @selectAllAlbums.
  ///
  /// In zh, this message translates to:
  /// **'全选相册'**
  String get selectAllAlbums;

  /// No description provided for @deleteSelectedAlbums.
  ///
  /// In zh, this message translates to:
  /// **'删除所选相册'**
  String get deleteSelectedAlbums;

  /// No description provided for @albumsEmpty.
  ///
  /// In zh, this message translates to:
  /// **'还没有创建相册'**
  String get albumsEmpty;

  /// No description provided for @previousPage.
  ///
  /// In zh, this message translates to:
  /// **'上一页'**
  String get previousPage;

  /// No description provided for @nextPage.
  ///
  /// In zh, this message translates to:
  /// **'下一页'**
  String get nextPage;

  /// No description provided for @albumsUnavailable.
  ///
  /// In zh, this message translates to:
  /// **'相册暂时无法加载'**
  String get albumsUnavailable;

  /// No description provided for @changeIcon.
  ///
  /// In zh, this message translates to:
  /// **'更改图标'**
  String get changeIcon;

  /// No description provided for @chooseAlbum.
  ///
  /// In zh, this message translates to:
  /// **'选择相册'**
  String get chooseAlbum;

  /// No description provided for @updatingAlbums.
  ///
  /// In zh, this message translates to:
  /// **'正在更新相册'**
  String get updatingAlbums;

  /// No description provided for @createFirstAlbum.
  ///
  /// In zh, this message translates to:
  /// **'还没有相册，请先新建一个。'**
  String get createFirstAlbum;

  /// No description provided for @loadingAlbums.
  ///
  /// In zh, this message translates to:
  /// **'正在读取相册'**
  String get loadingAlbums;

  /// No description provided for @albumsLoadFailed.
  ///
  /// In zh, this message translates to:
  /// **'相册暂时无法加载。'**
  String get albumsLoadFailed;

  /// No description provided for @removeFromAlbum.
  ///
  /// In zh, this message translates to:
  /// **'移出相册'**
  String get removeFromAlbum;

  /// No description provided for @albumGroupConflict.
  ///
  /// In zh, this message translates to:
  /// **'这组图片的相册归属有冲突，请选择一个相册。'**
  String get albumGroupConflict;

  /// No description provided for @albumDetailsUnavailable.
  ///
  /// In zh, this message translates to:
  /// **'相册信息暂时无法读取，点击重试。'**
  String get albumDetailsUnavailable;

  /// No description provided for @albumsSidebarHint.
  ///
  /// In zh, this message translates to:
  /// **'在侧栏中选择收藏夹或相册，也可以新建相册。'**
  String get albumsSidebarHint;

  /// No description provided for @file.
  ///
  /// In zh, this message translates to:
  /// **'文件'**
  String get file;

  /// No description provided for @path.
  ///
  /// In zh, this message translates to:
  /// **'路径'**
  String get path;

  /// No description provided for @fileType.
  ///
  /// In zh, this message translates to:
  /// **'类型'**
  String get fileType;

  /// No description provided for @imageDimensions.
  ///
  /// In zh, this message translates to:
  /// **'尺寸'**
  String get imageDimensions;

  /// No description provided for @size.
  ///
  /// In zh, this message translates to:
  /// **'大小'**
  String get size;

  /// No description provided for @cameraMake.
  ///
  /// In zh, this message translates to:
  /// **'相机品牌'**
  String get cameraMake;

  /// No description provided for @cameraModel.
  ///
  /// In zh, this message translates to:
  /// **'相机型号'**
  String get cameraModel;

  /// No description provided for @timeTaken.
  ///
  /// In zh, this message translates to:
  /// **'拍摄时间'**
  String get timeTaken;

  /// No description provided for @timeSource.
  ///
  /// In zh, this message translates to:
  /// **'时间来源'**
  String get timeSource;

  /// No description provided for @timeCreated.
  ///
  /// In zh, this message translates to:
  /// **'创建时间'**
  String get timeCreated;

  /// No description provided for @timeModified.
  ///
  /// In zh, this message translates to:
  /// **'修改时间'**
  String get timeModified;

  /// No description provided for @cameraOriginalTime.
  ///
  /// In zh, this message translates to:
  /// **'相机原始拍摄时间'**
  String get cameraOriginalTime;

  /// No description provided for @digitizedTime.
  ///
  /// In zh, this message translates to:
  /// **'数字化时间'**
  String get digitizedTime;

  /// No description provided for @photoRecordedTime.
  ///
  /// In zh, this message translates to:
  /// **'图片记录时间'**
  String get photoRecordedTime;

  /// No description provided for @loadingAlbumAssignments.
  ///
  /// In zh, this message translates to:
  /// **'正在读取相册归属'**
  String get loadingAlbumAssignments;

  /// No description provided for @conflictAlbumsUnavailable.
  ///
  /// In zh, this message translates to:
  /// **'无法读取冲突涉及的相册。'**
  String get conflictAlbumsUnavailable;

  /// No description provided for @albumAssignmentsUpdated.
  ///
  /// In zh, this message translates to:
  /// **'相册归属已更新。'**
  String get albumAssignmentsUpdated;

  /// No description provided for @conflictAlbums.
  ///
  /// In zh, this message translates to:
  /// **'冲突涉及的相册'**
  String get conflictAlbums;

  /// No description provided for @deletedAlbumName.
  ///
  /// In zh, this message translates to:
  /// **'{name}（已删除）'**
  String deletedAlbumName(String name);

  /// No description provided for @similarKind.
  ///
  /// In zh, this message translates to:
  /// **'相似'**
  String get similarKind;

  /// No description provided for @duplicateKind.
  ///
  /// In zh, this message translates to:
  /// **'重复'**
  String get duplicateKind;

  /// No description provided for @deletePhotoKind.
  ///
  /// In zh, this message translates to:
  /// **'删除{kind}图片'**
  String deletePhotoKind(String kind);

  /// No description provided for @saveSelectionFailed.
  ///
  /// In zh, this message translates to:
  /// **'未能保存选择'**
  String get saveSelectionFailed;

  /// No description provided for @tryAgain.
  ///
  /// In zh, this message translates to:
  /// **'请重试。'**
  String get tryAgain;

  /// No description provided for @similarPhotoCount.
  ///
  /// In zh, this message translates to:
  /// **'{count} 张相似图片'**
  String similarPhotoCount(int count);

  /// No description provided for @updatingFileLocations.
  ///
  /// In zh, this message translates to:
  /// **'正在更新文件位置'**
  String get updatingFileLocations;

  /// No description provided for @deleteSimilarPhotos.
  ///
  /// In zh, this message translates to:
  /// **'删除相似图片'**
  String get deleteSimilarPhotos;

  /// No description provided for @deleteDuplicatePhotos.
  ///
  /// In zh, this message translates to:
  /// **'删除重复图片'**
  String get deleteDuplicatePhotos;

  /// No description provided for @deleteSelected.
  ///
  /// In zh, this message translates to:
  /// **'删除所选'**
  String get deleteSelected;

  /// No description provided for @loadingFileLocations.
  ///
  /// In zh, this message translates to:
  /// **'正在读取文件位置'**
  String get loadingFileLocations;

  /// No description provided for @checkingPhotoContent.
  ///
  /// In zh, this message translates to:
  /// **'正在确认图片内容'**
  String get checkingPhotoContent;

  /// No description provided for @favoritesUnavailable.
  ///
  /// In zh, this message translates to:
  /// **'暂时无法读取收藏状态'**
  String get favoritesUnavailable;

  /// No description provided for @savingChanges.
  ///
  /// In zh, this message translates to:
  /// **'正在保存更改'**
  String get savingChanges;

  /// No description provided for @loadingFavoriteStatus.
  ///
  /// In zh, this message translates to:
  /// **'正在读取收藏状态'**
  String get loadingFavoriteStatus;

  /// No description provided for @removeFavorite.
  ///
  /// In zh, this message translates to:
  /// **'取消收藏'**
  String get removeFavorite;

  /// No description provided for @addFavorite.
  ///
  /// In zh, this message translates to:
  /// **'加入收藏夹'**
  String get addFavorite;

  /// No description provided for @readingPhotos.
  ///
  /// In zh, this message translates to:
  /// **'正在读取图片'**
  String get readingPhotos;

  /// No description provided for @photoOperation.
  ///
  /// In zh, this message translates to:
  /// **'{action}图片'**
  String photoOperation(String action);

  /// No description provided for @fileCount.
  ///
  /// In zh, this message translates to:
  /// **'{count} 个文件'**
  String fileCount(int count);

  /// No description provided for @destinationFolder.
  ///
  /// In zh, this message translates to:
  /// **'目标文件夹'**
  String get destinationFolder;

  /// No description provided for @change.
  ///
  /// In zh, this message translates to:
  /// **'更改'**
  String get change;

  /// No description provided for @folderStructure.
  ///
  /// In zh, this message translates to:
  /// **'文件夹结构'**
  String get folderStructure;

  /// No description provided for @fileNameLabel.
  ///
  /// In zh, this message translates to:
  /// **'文件名'**
  String get fileNameLabel;

  /// No description provided for @permanentDeleteWarning.
  ///
  /// In zh, this message translates to:
  /// **'文件将被永久删除，无法从回收站恢复。'**
  String get permanentDeleteWarning;

  /// No description provided for @recycleWarning.
  ///
  /// In zh, this message translates to:
  /// **'文件将移入 Windows 回收站。'**
  String get recycleWarning;

  /// No description provided for @deletePermanently.
  ///
  /// In zh, this message translates to:
  /// **'永久删除'**
  String get deletePermanently;

  /// No description provided for @back.
  ///
  /// In zh, this message translates to:
  /// **'返回'**
  String get back;

  /// No description provided for @operationReport.
  ///
  /// In zh, this message translates to:
  /// **'结果清单'**
  String get operationReport;

  /// No description provided for @affectedFiles.
  ///
  /// In zh, this message translates to:
  /// **'涉及文件'**
  String get affectedFiles;

  /// No description provided for @selectedCountShort.
  ///
  /// In zh, this message translates to:
  /// **'已选择 {count} 个'**
  String selectedCountShort(int count);

  /// No description provided for @reportFile.
  ///
  /// In zh, this message translates to:
  /// **'清单文件'**
  String get reportFile;

  /// No description provided for @reportHistoryUnavailable.
  ///
  /// In zh, this message translates to:
  /// **'暂时无法读取结果清单记录，请重试。'**
  String get reportHistoryUnavailable;

  /// No description provided for @loadingReport.
  ///
  /// In zh, this message translates to:
  /// **'正在读取结果清单'**
  String get loadingReport;

  /// No description provided for @noExportHistory.
  ///
  /// In zh, this message translates to:
  /// **'暂无导出记录'**
  String get noExportHistory;

  /// No description provided for @reportPending.
  ///
  /// In zh, this message translates to:
  /// **'清单尚未生成'**
  String get reportPending;

  /// No description provided for @noReportNeeded.
  ///
  /// In zh, this message translates to:
  /// **'没有特殊结果，无需清单'**
  String get noReportNeeded;

  /// No description provided for @reportWriteFailedStatus.
  ///
  /// In zh, this message translates to:
  /// **'清单写入失败，文件操作结果已保留'**
  String get reportWriteFailedStatus;

  /// No description provided for @createReport.
  ///
  /// In zh, this message translates to:
  /// **'生成清单'**
  String get createReport;

  /// No description provided for @regenerate.
  ///
  /// In zh, this message translates to:
  /// **'重新生成'**
  String get regenerate;

  /// No description provided for @updatingReport.
  ///
  /// In zh, this message translates to:
  /// **'正在更新结果清单'**
  String get updatingReport;

  /// No description provided for @latestRecords.
  ///
  /// In zh, this message translates to:
  /// **'最新记录'**
  String get latestRecords;

  /// No description provided for @earlierRecords.
  ///
  /// In zh, this message translates to:
  /// **'更早记录'**
  String get earlierRecords;

  /// No description provided for @stop.
  ///
  /// In zh, this message translates to:
  /// **'停止'**
  String get stop;

  /// No description provided for @viewHistory.
  ///
  /// In zh, this message translates to:
  /// **'查看记录'**
  String get viewHistory;

  /// No description provided for @noExceptionsReport.
  ///
  /// In zh, this message translates to:
  /// **'没有特殊结果，无需生成清单。'**
  String get noExceptionsReport;

  /// No description provided for @loadingOperationHistory.
  ///
  /// In zh, this message translates to:
  /// **'正在读取操作记录'**
  String get loadingOperationHistory;

  /// No description provided for @verifiedFiles.
  ///
  /// In zh, this message translates to:
  /// **'已核对 {checked} / {total} 个文件'**
  String verifiedFiles(int checked, int total);

  /// No description provided for @reportSavedDestination.
  ///
  /// In zh, this message translates to:
  /// **'结果清单已保存到目标文件夹。'**
  String get reportSavedDestination;

  /// No description provided for @processedFiles.
  ///
  /// In zh, this message translates to:
  /// **'已处理 {processed} / {total} 个文件'**
  String processedFiles(int processed, int total);

  /// No description provided for @fileHistoryUnavailable.
  ///
  /// In zh, this message translates to:
  /// **'暂时无法读取文件操作记录，请重试。'**
  String get fileHistoryUnavailable;

  /// No description provided for @confirmContinueMove.
  ///
  /// In zh, this message translates to:
  /// **'继续移动这个文件？'**
  String get confirmContinueMove;

  /// No description provided for @continueMove.
  ///
  /// In zh, this message translates to:
  /// **'继续移动'**
  String get continueMove;

  /// No description provided for @fileOperationHistory.
  ///
  /// In zh, this message translates to:
  /// **'文件操作记录'**
  String get fileOperationHistory;

  /// No description provided for @updatingFileRecords.
  ///
  /// In zh, this message translates to:
  /// **'正在更新文件记录'**
  String get updatingFileRecords;

  /// No description provided for @fileRecords.
  ///
  /// In zh, this message translates to:
  /// **'文件记录'**
  String get fileRecords;

  /// No description provided for @loadingFileRecords.
  ///
  /// In zh, this message translates to:
  /// **'正在读取文件记录'**
  String get loadingFileRecords;

  /// No description provided for @noFileOperations.
  ///
  /// In zh, this message translates to:
  /// **'暂无文件操作记录'**
  String get noFileOperations;

  /// No description provided for @fileOperationDetails.
  ///
  /// In zh, this message translates to:
  /// **'文件操作详情'**
  String get fileOperationDetails;

  /// No description provided for @checkResults.
  ///
  /// In zh, this message translates to:
  /// **'核对结果'**
  String get checkResults;

  /// No description provided for @recycleBinOpenFailed.
  ///
  /// In zh, this message translates to:
  /// **'无法打开回收站，请从 Windows 桌面打开。'**
  String get recycleBinOpenFailed;

  /// No description provided for @openRecycleBin.
  ///
  /// In zh, this message translates to:
  /// **'打开回收站'**
  String get openRecycleBin;

  /// No description provided for @duplicatesOnly.
  ///
  /// In zh, this message translates to:
  /// **'仅显示重复'**
  String get duplicatesOnly;

  /// No description provided for @similarOnly.
  ///
  /// In zh, this message translates to:
  /// **'仅显示相似'**
  String get similarOnly;

  /// No description provided for @mergeDuplicates.
  ///
  /// In zh, this message translates to:
  /// **'合并重复'**
  String get mergeDuplicates;

  /// No description provided for @removeFilter.
  ///
  /// In zh, this message translates to:
  /// **'移除{label}'**
  String removeFilter(String label);

  /// No description provided for @selectedItems.
  ///
  /// In zh, this message translates to:
  /// **'已选择 {count} 个项目'**
  String selectedItems(int count);

  /// No description provided for @duplicateGroupCount.
  ///
  /// In zh, this message translates to:
  /// **'{count} 个重复组'**
  String duplicateGroupCount(int count);

  /// No description provided for @similarGroupCount.
  ///
  /// In zh, this message translates to:
  /// **'{count} 个相似组'**
  String similarGroupCount(int count);

  /// No description provided for @deduplicatedCount.
  ///
  /// In zh, this message translates to:
  /// **'{summary} · 去重后 {count} 张'**
  String deduplicatedCount(String summary, int count);

  /// No description provided for @organizationCounts.
  ///
  /// In zh, this message translates to:
  /// **'{total} 张图片 · 已整理 {organized} · 待整理 {pending} · 已忽略 {ignored}'**
  String organizationCounts(int total, int organized, int pending, int ignored);

  /// No description provided for @organizationMode.
  ///
  /// In zh, this message translates to:
  /// **'整理模式'**
  String get organizationMode;

  /// No description provided for @exitOrganization.
  ///
  /// In zh, this message translates to:
  /// **'退出整理'**
  String get exitOrganization;

  /// No description provided for @noPendingPhotos.
  ///
  /// In zh, this message translates to:
  /// **'没有待整理图片'**
  String get noPendingPhotos;

  /// No description provided for @noDeferredPhotos.
  ///
  /// In zh, this message translates to:
  /// **'没有稍后整理的图片'**
  String get noDeferredPhotos;

  /// No description provided for @noIgnoredPhotos.
  ///
  /// In zh, this message translates to:
  /// **'没有已忽略的图片'**
  String get noIgnoredPhotos;

  /// No description provided for @organizationEmptyHint.
  ///
  /// In zh, this message translates to:
  /// **'可以切换整理队列，或在更多菜单中退出整理模式。'**
  String get organizationEmptyHint;

  /// No description provided for @noCategoryPhotos.
  ///
  /// In zh, this message translates to:
  /// **'还没有这个分类的图片'**
  String get noCategoryPhotos;

  /// No description provided for @categoryEmptyHint.
  ///
  /// In zh, this message translates to:
  /// **'可以切换分类或返回图库。'**
  String get categoryEmptyHint;

  /// No description provided for @noAlbumPhotos.
  ///
  /// In zh, this message translates to:
  /// **'这个相册里还没有图片'**
  String get noAlbumPhotos;

  /// No description provided for @albumEmptyHint.
  ///
  /// In zh, this message translates to:
  /// **'在图库中打开图片后，可以将它加入相册。'**
  String get albumEmptyHint;

  /// No description provided for @noFavoritePhotos.
  ///
  /// In zh, this message translates to:
  /// **'收藏夹里还没有图片'**
  String get noFavoritePhotos;

  /// No description provided for @favoritesEmptyHint.
  ///
  /// In zh, this message translates to:
  /// **'打开图片后，可以将它加入收藏夹。'**
  String get favoritesEmptyHint;

  /// No description provided for @filmstripSettingFailed.
  ///
  /// In zh, this message translates to:
  /// **'无法保存缩略图设置，请重试'**
  String get filmstripSettingFailed;

  /// No description provided for @photoPosition.
  ///
  /// In zh, this message translates to:
  /// **'{position} / {total} 张'**
  String photoPosition(int position, int total);

  /// No description provided for @clearFilter.
  ///
  /// In zh, this message translates to:
  /// **'清除{label}'**
  String clearFilter(String label);

  /// No description provided for @hide.
  ///
  /// In zh, this message translates to:
  /// **'隐藏'**
  String get hide;

  /// No description provided for @downloadSmartModel.
  ///
  /// In zh, this message translates to:
  /// **'下载智能相册模型'**
  String get downloadSmartModel;

  /// No description provided for @smartModelDownloadDetail.
  ///
  /// In zh, this message translates to:
  /// **'需要下载约 504 MB 的模型，之后在本机分析图片。已有的完整模型文件会复用。'**
  String get smartModelDownloadDetail;

  /// No description provided for @downloadAndEnable.
  ///
  /// In zh, this message translates to:
  /// **'下载并启用'**
  String get downloadAndEnable;

  /// No description provided for @folders.
  ///
  /// In zh, this message translates to:
  /// **'文件夹'**
  String get folders;

  /// No description provided for @removeSelectedFolders.
  ///
  /// In zh, this message translates to:
  /// **'从 Ame 中移除所选文件夹'**
  String get removeSelectedFolders;

  /// No description provided for @resizeSidebar.
  ///
  /// In zh, this message translates to:
  /// **'调整侧栏宽度'**
  String get resizeSidebar;

  /// No description provided for @pixelCount.
  ///
  /// In zh, this message translates to:
  /// **'{count} 像素'**
  String pixelCount(int count);

  /// No description provided for @undoAction.
  ///
  /// In zh, this message translates to:
  /// **'撤销操作'**
  String get undoAction;

  /// No description provided for @redoAction.
  ///
  /// In zh, this message translates to:
  /// **'重做操作'**
  String get redoAction;

  /// No description provided for @historyLoadRetry.
  ///
  /// In zh, this message translates to:
  /// **'操作记录未能加载，点击重试'**
  String get historyLoadRetry;

  /// No description provided for @organizationOptions.
  ///
  /// In zh, this message translates to:
  /// **'整理选项'**
  String get organizationOptions;

  /// No description provided for @albumsAndFavoritesUnavailable.
  ///
  /// In zh, this message translates to:
  /// **'暂时无法读取相册和收藏'**
  String get albumsAndFavoritesUnavailable;

  /// No description provided for @reopenMenuRetry.
  ///
  /// In zh, this message translates to:
  /// **'请重新打开菜单重试。'**
  String get reopenMenuRetry;

  /// No description provided for @copyToFolder.
  ///
  /// In zh, this message translates to:
  /// **'复制到文件夹'**
  String get copyToFolder;

  /// No description provided for @moveToFolder.
  ///
  /// In zh, this message translates to:
  /// **'移动到文件夹'**
  String get moveToFolder;

  /// No description provided for @photoListFailed.
  ///
  /// In zh, this message translates to:
  /// **'图片列表加载失败'**
  String get photoListFailed;

  /// No description provided for @continueOrganizing.
  ///
  /// In zh, this message translates to:
  /// **'继续整理'**
  String get continueOrganizing;

  /// No description provided for @toOrganize.
  ///
  /// In zh, this message translates to:
  /// **'待整理'**
  String get toOrganize;

  /// No description provided for @photosRelatedTo.
  ///
  /// In zh, this message translates to:
  /// **'与 {name} 相关的图片'**
  String photosRelatedTo(String name);

  /// No description provided for @selectedAlbumConflict.
  ///
  /// In zh, this message translates to:
  /// **'所选图片中有相册归属冲突，请选择一个相册。'**
  String get selectedAlbumConflict;

  /// No description provided for @actionFailed.
  ///
  /// In zh, this message translates to:
  /// **'无法完成操作'**
  String get actionFailed;

  /// No description provided for @favorite.
  ///
  /// In zh, this message translates to:
  /// **'收藏'**
  String get favorite;

  /// No description provided for @preparing.
  ///
  /// In zh, this message translates to:
  /// **'正在准备'**
  String get preparing;

  /// No description provided for @processing.
  ///
  /// In zh, this message translates to:
  /// **'正在处理'**
  String get processing;

  /// No description provided for @itemsCompleted.
  ///
  /// In zh, this message translates to:
  /// **'已完成 {completed} / {total} 项。'**
  String itemsCompleted(int completed, int total);

  /// No description provided for @sortOrderSaveFailed.
  ///
  /// In zh, this message translates to:
  /// **'排列顺序未能保存，请重新拖动。'**
  String get sortOrderSaveFailed;

  /// No description provided for @confirmRemoveFolders.
  ///
  /// In zh, this message translates to:
  /// **'从 Ame 中移除 {count} 个文件夹？'**
  String confirmRemoveFolders(int count);

  /// No description provided for @removeFoldersImpact.
  ///
  /// In zh, this message translates to:
  /// **'这些文件夹中的图片将不再显示在 Ame 中，仅属于这些文件夹的相册归属、收藏和整理记录也会移除。其他来源仍有的重复图片会保留记录。原目录和图片文件都会保留。'**
  String get removeFoldersImpact;

  /// No description provided for @selectedCount.
  ///
  /// In zh, this message translates to:
  /// **'已选择 {count}'**
  String selectedCount(int count);

  /// No description provided for @selectedLong.
  ///
  /// In zh, this message translates to:
  /// **'已选择'**
  String get selectedLong;

  /// No description provided for @selectedShort.
  ///
  /// In zh, this message translates to:
  /// **'已选'**
  String get selectedShort;

  /// No description provided for @deselectAll.
  ///
  /// In zh, this message translates to:
  /// **'取消全选'**
  String get deselectAll;

  /// No description provided for @selectAllNamed.
  ///
  /// In zh, this message translates to:
  /// **'全选{label}'**
  String selectAllNamed(String label);

  /// Tooltip and accessible name for collapsing a sidebar section.
  ///
  /// In zh, this message translates to:
  /// **'收起{section}'**
  String collapseSection(String section);

  /// Tooltip and accessible name for expanding a sidebar section.
  ///
  /// In zh, this message translates to:
  /// **'展开{section}'**
  String expandSection(String section);

  /// No description provided for @deselectNamed.
  ///
  /// In zh, this message translates to:
  /// **'取消选择{name}'**
  String deselectNamed(String name);

  /// No description provided for @selectNamed.
  ///
  /// In zh, this message translates to:
  /// **'选择{name}'**
  String selectNamed(String name);

  /// No description provided for @noClassificationResults.
  ///
  /// In zh, this message translates to:
  /// **'还没有分类结果'**
  String get noClassificationResults;

  /// No description provided for @downloadModel.
  ///
  /// In zh, this message translates to:
  /// **'下载模型'**
  String get downloadModel;

  /// No description provided for @loadingSmartAlbums.
  ///
  /// In zh, this message translates to:
  /// **'正在加载智能相册'**
  String get loadingSmartAlbums;

  /// No description provided for @categoriesUnavailable.
  ///
  /// In zh, this message translates to:
  /// **'分类暂时无法加载'**
  String get categoriesUnavailable;

  /// No description provided for @recentlyAddedCount.
  ///
  /// In zh, this message translates to:
  /// **'新加入 · {count} 张'**
  String recentlyAddedCount(int count);

  /// No description provided for @previouslyAddedCount.
  ///
  /// In zh, this message translates to:
  /// **'此前加入 · {count} 张'**
  String previouslyAddedCount(int count);

  /// No description provided for @recentlyAdded.
  ///
  /// In zh, this message translates to:
  /// **'新加入'**
  String get recentlyAdded;

  /// No description provided for @previouslyAdded.
  ///
  /// In zh, this message translates to:
  /// **'此前'**
  String get previouslyAdded;

  /// No description provided for @pause.
  ///
  /// In zh, this message translates to:
  /// **'暂停'**
  String get pause;

  /// No description provided for @resume.
  ///
  /// In zh, this message translates to:
  /// **'继续'**
  String get resume;

  /// No description provided for @libraryUpdates.
  ///
  /// In zh, this message translates to:
  /// **'图库更新'**
  String get libraryUpdates;

  /// No description provided for @clearFinishedTasks.
  ///
  /// In zh, this message translates to:
  /// **'清除已结束任务'**
  String get clearFinishedTasks;

  /// No description provided for @cancelRootUpdate.
  ///
  /// In zh, this message translates to:
  /// **'取消更新 {name}'**
  String cancelRootUpdate(String name);

  /// No description provided for @closeRootUpdate.
  ///
  /// In zh, this message translates to:
  /// **'关闭 {name} 更新任务'**
  String closeRootUpdate(String name);

  /// No description provided for @rootUpdateProgress.
  ///
  /// In zh, this message translates to:
  /// **'{name} 更新进度'**
  String rootUpdateProgress(String name);

  /// No description provided for @categoryLabel.
  ///
  /// In zh, this message translates to:
  /// **'分类：{name}'**
  String categoryLabel(String name);

  /// No description provided for @previousPhotoShortcut.
  ///
  /// In zh, this message translates to:
  /// **'上一张（←）'**
  String get previousPhotoShortcut;

  /// No description provided for @nextPhotoShortcut.
  ///
  /// In zh, this message translates to:
  /// **'下一张（→）'**
  String get nextPhotoShortcut;

  /// No description provided for @photoCopied.
  ///
  /// In zh, this message translates to:
  /// **'图片已复制到剪贴板'**
  String get photoCopied;

  /// No description provided for @photoTooLargeToCopy.
  ///
  /// In zh, this message translates to:
  /// **'图片过大，无法复制到剪贴板'**
  String get photoTooLargeToCopy;

  /// No description provided for @copyPhotoFailed.
  ///
  /// In zh, this message translates to:
  /// **'无法复制图片，请重试'**
  String get copyPhotoFailed;

  /// No description provided for @reloadThumbnails.
  ///
  /// In zh, this message translates to:
  /// **'重新加载缩略图'**
  String get reloadThumbnails;

  /// No description provided for @toggleFullscreenFailed.
  ///
  /// In zh, this message translates to:
  /// **'无法切换全屏，请重试'**
  String get toggleFullscreenFailed;

  /// No description provided for @readingPhotoStatus.
  ///
  /// In zh, this message translates to:
  /// **'正在读取图片状态'**
  String get readingPhotoStatus;

  /// No description provided for @rereadPhotoStatus.
  ///
  /// In zh, this message translates to:
  /// **'重新读取图片状态'**
  String get rereadPhotoStatus;

  /// No description provided for @copyPhoto.
  ///
  /// In zh, this message translates to:
  /// **'复制图片'**
  String get copyPhoto;

  /// No description provided for @rotateClockwise.
  ///
  /// In zh, this message translates to:
  /// **'顺时针旋转 90°'**
  String get rotateClockwise;

  /// No description provided for @zoomOutShortcut.
  ///
  /// In zh, this message translates to:
  /// **'缩小（- / Ctrl+-）'**
  String get zoomOutShortcut;

  /// No description provided for @zoomInShortcut.
  ///
  /// In zh, this message translates to:
  /// **'放大（+ / Ctrl++）'**
  String get zoomInShortcut;

  /// No description provided for @archiveFileType.
  ///
  /// In zh, this message translates to:
  /// **'Ame 整理记录'**
  String get archiveFileType;

  /// No description provided for @exportAction.
  ///
  /// In zh, this message translates to:
  /// **'导出'**
  String get exportAction;

  /// No description provided for @chooseBackup.
  ///
  /// In zh, this message translates to:
  /// **'选择备份'**
  String get chooseBackup;

  /// No description provided for @inspect.
  ///
  /// In zh, this message translates to:
  /// **'检查'**
  String get inspect;

  /// No description provided for @restore.
  ///
  /// In zh, this message translates to:
  /// **'恢复'**
  String get restore;

  /// No description provided for @preparingBackup.
  ///
  /// In zh, this message translates to:
  /// **'正在准备备份'**
  String get preparingBackup;

  /// No description provided for @exportingArchive.
  ///
  /// In zh, this message translates to:
  /// **'正在导出整理记录'**
  String get exportingArchive;

  /// No description provided for @checkingBackup.
  ///
  /// In zh, this message translates to:
  /// **'正在检查备份'**
  String get checkingBackup;

  /// No description provided for @linkingPhotos.
  ///
  /// In zh, this message translates to:
  /// **'正在关联图库中的图片'**
  String get linkingPhotos;

  /// No description provided for @restoringArchive.
  ///
  /// In zh, this message translates to:
  /// **'正在恢复整理记录'**
  String get restoringArchive;

  /// No description provided for @archiveInProgress.
  ///
  /// In zh, this message translates to:
  /// **'正在{operation}整理记录'**
  String archiveInProgress(String operation);

  /// No description provided for @archiveOutsideLibrary.
  ///
  /// In zh, this message translates to:
  /// **'请选择图库文件夹和 Ame 数据文件夹以外的位置。'**
  String get archiveOutsideLibrary;

  /// No description provided for @archiveFileExists.
  ///
  /// In zh, this message translates to:
  /// **'这个文件已存在，请使用其他文件名。'**
  String get archiveFileExists;

  /// No description provided for @archiveVersionUnsupported.
  ///
  /// In zh, this message translates to:
  /// **'暂不支持这份备份的版本，请选择兼容的备份。'**
  String get archiveVersionUnsupported;

  /// No description provided for @archiveInvalid.
  ///
  /// In zh, this message translates to:
  /// **'这份备份不完整或内容有误，请重新选择。'**
  String get archiveInvalid;

  /// No description provided for @archiveLibraryChanged.
  ///
  /// In zh, this message translates to:
  /// **'图库或整理记录已更新，请重新检查备份后确认恢复。'**
  String get archiveLibraryChanged;

  /// No description provided for @archiveChanged.
  ///
  /// In zh, this message translates to:
  /// **'所选备份已改变，请重新检查后确认恢复。'**
  String get archiveChanged;

  /// No description provided for @archiveAlreadyRunning.
  ///
  /// In zh, this message translates to:
  /// **'上一项备份操作尚未结束，请稍后重试。'**
  String get archiveAlreadyRunning;

  /// No description provided for @archiveUnreadable.
  ///
  /// In zh, this message translates to:
  /// **'无法读取这份备份，请确认文件在本机可用。'**
  String get archiveUnreadable;

  /// No description provided for @archiveIoFailed.
  ///
  /// In zh, this message translates to:
  /// **'无法读写备份文件，请检查所选文件和保存位置后重试。'**
  String get archiveIoFailed;

  /// No description provided for @archiveConflictsWithCurrent.
  ///
  /// In zh, this message translates to:
  /// **'这份备份与当前记录有冲突，当前记录已保留。请选择其他备份。'**
  String get archiveConflictsWithCurrent;

  /// No description provided for @archiveResultUnknown.
  ///
  /// In zh, this message translates to:
  /// **'尚未确认操作结果，请重试。'**
  String get archiveResultUnknown;

  /// No description provided for @settingsSaveRetry.
  ///
  /// In zh, this message translates to:
  /// **'设置未能保存，请重试。'**
  String get settingsSaveRetry;

  /// No description provided for @unreadable.
  ///
  /// In zh, this message translates to:
  /// **'无法读取'**
  String get unreadable;

  /// No description provided for @askBeforeAdding.
  ///
  /// In zh, this message translates to:
  /// **'加入相册前询问'**
  String get askBeforeAdding;

  /// No description provided for @askBeforeAddingHelp.
  ///
  /// In zh, this message translates to:
  /// **'每次加入前选择目标相册'**
  String get askBeforeAddingHelp;

  /// No description provided for @defaultAlbum.
  ///
  /// In zh, this message translates to:
  /// **'默认加入的相册'**
  String get defaultAlbum;

  /// No description provided for @defaultAlbumHelp.
  ///
  /// In zh, this message translates to:
  /// **'未设置时，会在加入前询问'**
  String get defaultAlbumHelp;

  /// No description provided for @gpuSettingFailed.
  ///
  /// In zh, this message translates to:
  /// **'GPU 加速设置未能完成'**
  String get gpuSettingFailed;

  /// No description provided for @gpuSettingRetry.
  ///
  /// In zh, this message translates to:
  /// **'请重试；如果仍无法分析图片，可以关闭 GPU 加速。'**
  String get gpuSettingRetry;

  /// No description provided for @gpuAcceleration.
  ///
  /// In zh, this message translates to:
  /// **'GPU 加速'**
  String get gpuAcceleration;

  /// No description provided for @loadingSettings.
  ///
  /// In zh, this message translates to:
  /// **'正在读取设置'**
  String get loadingSettings;

  /// No description provided for @settingsReadFailed.
  ///
  /// In zh, this message translates to:
  /// **'无法读取设置，请重试'**
  String get settingsReadFailed;

  /// No description provided for @switchingAnalysisDevice.
  ///
  /// In zh, this message translates to:
  /// **'正在切换分析设备'**
  String get switchingAnalysisDevice;

  /// No description provided for @smartAlbumsCpu.
  ///
  /// In zh, this message translates to:
  /// **'智能相册当前使用 CPU 分析图片'**
  String get smartAlbumsCpu;

  /// No description provided for @smartAlbumsDevice.
  ///
  /// In zh, this message translates to:
  /// **'智能相册当前图片分析设备：{name}'**
  String smartAlbumsDevice(String name);

  /// No description provided for @smartAlbumsPreferGpu.
  ///
  /// In zh, this message translates to:
  /// **'智能相册优先使用可用显卡分析图片'**
  String get smartAlbumsPreferGpu;

  /// No description provided for @restoreOrganizationRecords.
  ///
  /// In zh, this message translates to:
  /// **'恢复整理记录'**
  String get restoreOrganizationRecords;

  /// No description provided for @createdAt.
  ///
  /// In zh, this message translates to:
  /// **'创建时间：{timestamp}'**
  String createdAt(String timestamp);

  /// No description provided for @albumCount.
  ///
  /// In zh, this message translates to:
  /// **'{count} 个相册'**
  String albumCount(int count);

  /// No description provided for @photoGroupCount.
  ///
  /// In zh, this message translates to:
  /// **'{count} 组图片'**
  String photoGroupCount(int count);

  /// No description provided for @favoriteGroupCount.
  ///
  /// In zh, this message translates to:
  /// **'{count} 组收藏'**
  String favoriteGroupCount(int count);

  /// No description provided for @duplicateChoiceCount.
  ///
  /// In zh, this message translates to:
  /// **'{count} 项去重选择'**
  String duplicateChoiceCount(int count);

  /// No description provided for @matchedGroups.
  ///
  /// In zh, this message translates to:
  /// **'可关联 {count} 组'**
  String matchedGroups(int count);

  /// No description provided for @unmatchedGroups.
  ///
  /// In zh, this message translates to:
  /// **'暂未关联 {count} 组'**
  String unmatchedGroups(int count);

  /// No description provided for @albumConflictCount.
  ///
  /// In zh, this message translates to:
  /// **'{count} 组相册冲突'**
  String albumConflictCount(int count);

  /// No description provided for @unresolvedCopies.
  ///
  /// In zh, this message translates to:
  /// **'{count} 项首选副本暂无法关联'**
  String unresolvedCopies(int count);

  /// No description provided for @includesOrganizationPosition.
  ///
  /// In zh, this message translates to:
  /// **'包含继续整理的位置'**
  String get includesOrganizationPosition;

  /// No description provided for @restoreKeepsChoices.
  ///
  /// In zh, this message translates to:
  /// **'当前选择和整理位置会保留，不会移动或修改原图片。'**
  String get restoreKeepsChoices;

  /// No description provided for @unmatchedAwaitPhotos.
  ///
  /// In zh, this message translates to:
  /// **'暂未关联的记录会等待图片重新关联。'**
  String get unmatchedAwaitPhotos;

  /// No description provided for @filePickerFailed.
  ///
  /// In zh, this message translates to:
  /// **'无法打开文件选择窗口'**
  String get filePickerFailed;

  /// No description provided for @organizationRecords.
  ///
  /// In zh, this message translates to:
  /// **'整理记录'**
  String get organizationRecords;

  /// No description provided for @exportOrganizationRecords.
  ///
  /// In zh, this message translates to:
  /// **'导出整理记录'**
  String get exportOrganizationRecords;

  /// No description provided for @exportArchiveHelp.
  ///
  /// In zh, this message translates to:
  /// **'备份相册、收藏和整理进度，不包含原图片'**
  String get exportArchiveHelp;

  /// No description provided for @restoreArchiveHelp.
  ///
  /// In zh, this message translates to:
  /// **'从备份恢复，保留当前已作出的选择'**
  String get restoreArchiveHelp;

  /// No description provided for @viewBackup.
  ///
  /// In zh, this message translates to:
  /// **'查看备份'**
  String get viewBackup;

  /// No description provided for @chooseFile.
  ///
  /// In zh, this message translates to:
  /// **'选择文件'**
  String get chooseFile;

  /// No description provided for @noUnlinkedRecords.
  ///
  /// In zh, this message translates to:
  /// **'没有符合条件的未关联记录'**
  String get noUnlinkedRecords;

  /// No description provided for @unlinkedLessThanDay.
  ///
  /// In zh, this message translates to:
  /// **'未关联不足 1 天'**
  String get unlinkedLessThanDay;

  /// No description provided for @unlinkedDays.
  ///
  /// In zh, this message translates to:
  /// **'已未关联 {days} 天'**
  String unlinkedDays(int days);

  /// No description provided for @unlinkedRange.
  ///
  /// In zh, this message translates to:
  /// **'未关联 {shortest}～{longest} 天'**
  String unlinkedRange(String shortest, int longest);

  /// No description provided for @lessThanOne.
  ///
  /// In zh, this message translates to:
  /// **'不足 1'**
  String get lessThanOne;

  /// No description provided for @clearUnlinkedRecords.
  ///
  /// In zh, this message translates to:
  /// **'清理未关联记录'**
  String get clearUnlinkedRecords;

  /// No description provided for @checkingRecords.
  ///
  /// In zh, this message translates to:
  /// **'正在检查记录'**
  String get checkingRecords;

  /// No description provided for @cleanupScope.
  ///
  /// In zh, this message translates to:
  /// **'清理范围'**
  String get cleanupScope;

  /// No description provided for @unlinkedNinetyDays.
  ///
  /// In zh, this message translates to:
  /// **'未关联超过 90 天'**
  String get unlinkedNinetyDays;

  /// No description provided for @unlinkedThirtyDays.
  ///
  /// In zh, this message translates to:
  /// **'未关联超过 30 天'**
  String get unlinkedThirtyDays;

  /// No description provided for @allUnlinkedRecords.
  ///
  /// In zh, this message translates to:
  /// **'全部未关联记录'**
  String get allUnlinkedRecords;

  /// No description provided for @checkFailedRetry.
  ///
  /// In zh, this message translates to:
  /// **'未能完成检查，请重试。'**
  String get checkFailedRetry;

  /// No description provided for @checkCancelled.
  ///
  /// In zh, this message translates to:
  /// **'检查已取消'**
  String get checkCancelled;

  /// No description provided for @clearRecords.
  ///
  /// In zh, this message translates to:
  /// **'清理记录'**
  String get clearRecords;

  /// No description provided for @cleanupCounts.
  ///
  /// In zh, this message translates to:
  /// **'{groups} 组图片的记录 · {choices} 项去重选择'**
  String cleanupCounts(int groups, int choices);

  /// No description provided for @cleanupAlbumGroups.
  ///
  /// In zh, this message translates to:
  /// **'相册归属 {count} 组'**
  String cleanupAlbumGroups(int count);

  /// No description provided for @cleanupFavoriteGroups.
  ///
  /// In zh, this message translates to:
  /// **'收藏 {count} 组'**
  String cleanupFavoriteGroups(int count);

  /// No description provided for @cleanupProgressGroups.
  ///
  /// In zh, this message translates to:
  /// **'整理进度 {count} 组'**
  String cleanupProgressGroups(int count);

  /// No description provided for @cleanupClassificationGroups.
  ///
  /// In zh, this message translates to:
  /// **'分类记录 {count} 组'**
  String cleanupClassificationGroups(int count);

  /// No description provided for @cleanupRecordImpact.
  ///
  /// In zh, this message translates to:
  /// **'相册和原图片会保留。重新导入图片不会恢复这些记录。'**
  String get cleanupRecordImpact;

  /// No description provided for @cleanupUndoWarning.
  ///
  /// In zh, this message translates to:
  /// **'本次清理无法撤销。如需恢复记录，可以手动恢复此前导出的备份。'**
  String get cleanupUndoWarning;

  /// No description provided for @cleanupRecordsHelp.
  ///
  /// In zh, this message translates to:
  /// **'检查无法关联图片的相册、收藏和整理记录'**
  String get cleanupRecordsHelp;

  /// No description provided for @cleanUp.
  ///
  /// In zh, this message translates to:
  /// **'清理'**
  String get cleanUp;

  /// No description provided for @stopNotConfirmed.
  ///
  /// In zh, this message translates to:
  /// **'尚未停止，请再次尝试。'**
  String get stopNotConfirmed;

  /// No description provided for @clearingRecords.
  ///
  /// In zh, this message translates to:
  /// **'正在清理记录'**
  String get clearingRecords;

  /// No description provided for @checkAgain.
  ///
  /// In zh, this message translates to:
  /// **'重新检查'**
  String get checkAgain;

  /// No description provided for @recordsChanged.
  ///
  /// In zh, this message translates to:
  /// **'记录已发生变化，请重新检查后确认。'**
  String get recordsChanged;

  /// No description provided for @cleanupResultUnknown.
  ///
  /// In zh, this message translates to:
  /// **'未能确认清理结果，请重试。'**
  String get cleanupResultUnknown;

  /// No description provided for @confirmClearThumbnails.
  ///
  /// In zh, this message translates to:
  /// **'清理缩略图？'**
  String get confirmClearThumbnails;

  /// No description provided for @confirmClearOldThumbnails.
  ///
  /// In zh, this message translates to:
  /// **'清理旧缩略图目录？'**
  String get confirmClearOldThumbnails;

  /// No description provided for @oldThumbnailFolder.
  ///
  /// In zh, this message translates to:
  /// **'旧缩略图文件夹'**
  String get oldThumbnailFolder;

  /// No description provided for @clearThumbnailsImpact.
  ///
  /// In zh, this message translates to:
  /// **'这会删除可重新生成的缩略图缓存，不会删除或修改原图片。清理后，打开图库时可能需要一些时间重新生成缩略图。'**
  String get clearThumbnailsImpact;

  /// No description provided for @clearOldThumbnailsImpact.
  ///
  /// In zh, this message translates to:
  /// **'只会删除这个文件夹中由 Ame 管理的缩略图。原图片和其他文件会保留。'**
  String get clearOldThumbnailsImpact;

  /// No description provided for @startCleanup.
  ///
  /// In zh, this message translates to:
  /// **'开始清理'**
  String get startCleanup;

  /// No description provided for @cleanupAlreadyFinished.
  ///
  /// In zh, this message translates to:
  /// **'清理任务已经结束，无法再取消'**
  String get cleanupAlreadyFinished;

  /// No description provided for @storage.
  ///
  /// In zh, this message translates to:
  /// **'存储'**
  String get storage;

  /// No description provided for @loadingStorageSettings.
  ///
  /// In zh, this message translates to:
  /// **'正在读取存储设置'**
  String get loadingStorageSettings;

  /// No description provided for @checkingStoragePaths.
  ///
  /// In zh, this message translates to:
  /// **'正在检查图库数据与缩略图的保存位置'**
  String get checkingStoragePaths;

  /// No description provided for @storageSettingsUnavailable.
  ///
  /// In zh, this message translates to:
  /// **'无法读取存储设置'**
  String get storageSettingsUnavailable;

  /// No description provided for @storageRestartRequired.
  ///
  /// In zh, this message translates to:
  /// **'重启 Ame 后应用新的存储设置'**
  String get storageRestartRequired;

  /// No description provided for @existingFilesKept.
  ///
  /// In zh, this message translates to:
  /// **'现有文件不会被移动或删除'**
  String get existingFilesKept;

  /// No description provided for @catalogStorageLocation.
  ///
  /// In zh, this message translates to:
  /// **'图库数据位置'**
  String get catalogStorageLocation;

  /// No description provided for @catalogStorageHelp.
  ///
  /// In zh, this message translates to:
  /// **'保存图库索引和扫描结果\n{path}'**
  String catalogStorageHelp(String path);

  /// No description provided for @storageUsageWithDetail.
  ///
  /// In zh, this message translates to:
  /// **'当前占用 {size}{detail}'**
  String storageUsageWithDetail(String size, String detail);

  /// No description provided for @storageUsageDetail.
  ///
  /// In zh, this message translates to:
  /// **'，有效数据约 {live}，可回收 {reclaimable}'**
  String storageUsageDetail(String live, String reclaimable);

  /// No description provided for @catalogLocationLocked.
  ///
  /// In zh, this message translates to:
  /// **'已有图库时不可更改'**
  String get catalogLocationLocked;

  /// No description provided for @starting.
  ///
  /// In zh, this message translates to:
  /// **'正在启动'**
  String get starting;

  /// No description provided for @cleanupNotNeeded.
  ///
  /// In zh, this message translates to:
  /// **'无需清理'**
  String get cleanupNotNeeded;

  /// No description provided for @thumbnailLocation.
  ///
  /// In zh, this message translates to:
  /// **'缩略图位置'**
  String get thumbnailLocation;

  /// No description provided for @thumbnailStorageHelp.
  ///
  /// In zh, this message translates to:
  /// **'保存可随时重新生成的图片预览\n{path}'**
  String thumbnailStorageHelp(String path);

  /// No description provided for @retiredThumbnailDirectory.
  ///
  /// In zh, this message translates to:
  /// **'旧缩略图目录'**
  String get retiredThumbnailDirectory;

  /// No description provided for @retiredThumbnailsHelp.
  ///
  /// In zh, this message translates to:
  /// **'新目录已启用；旧目录只会在确认后清理 Ame 管理的缩略图\n{path}'**
  String retiredThumbnailsHelp(String path);

  /// No description provided for @cleaning.
  ///
  /// In zh, this message translates to:
  /// **'正在清理'**
  String get cleaning;

  /// No description provided for @cleanOldFolder.
  ///
  /// In zh, this message translates to:
  /// **'清理旧目录'**
  String get cleanOldFolder;

  /// No description provided for @thumbnailBudget.
  ///
  /// In zh, this message translates to:
  /// **'缩略图最大占用空间'**
  String get thumbnailBudget;

  /// No description provided for @thumbnailBudgetHelp.
  ///
  /// In zh, this message translates to:
  /// **'达到上限时自动回收当前不需要的缩略图，不会修改原图片'**
  String get thumbnailBudgetHelp;

  /// No description provided for @storageUsage.
  ///
  /// In zh, this message translates to:
  /// **'当前占用 {size}'**
  String storageUsage(String size);

  /// No description provided for @storageSaveFailed.
  ///
  /// In zh, this message translates to:
  /// **'未能保存存储设置'**
  String get storageSaveFailed;

  /// No description provided for @storageOverlapsSource.
  ///
  /// In zh, this message translates to:
  /// **'存储位置与已添加的图片文件夹重叠。请选择单独的文件夹。'**
  String get storageOverlapsSource;

  /// No description provided for @storageErrorDetails.
  ///
  /// In zh, this message translates to:
  /// **'查看错误详情'**
  String get storageErrorDetails;

  /// No description provided for @storageHideErrorDetails.
  ///
  /// In zh, this message translates to:
  /// **'收起错误详情'**
  String get storageHideErrorDetails;

  /// No description provided for @catalogMaintenanceStatusError.
  ///
  /// In zh, this message translates to:
  /// **'图库数据整理状态异常'**
  String get catalogMaintenanceStatusError;

  /// No description provided for @thumbnailCleanupStatusError.
  ///
  /// In zh, this message translates to:
  /// **'缩略图清理状态异常'**
  String get thumbnailCleanupStatusError;

  /// No description provided for @storageLocationPending.
  ///
  /// In zh, this message translates to:
  /// **'完成前不会改变当前正在使用的存储位置'**
  String get storageLocationPending;

  /// No description provided for @optimizeCatalog.
  ///
  /// In zh, this message translates to:
  /// **'整理图库数据'**
  String get optimizeCatalog;

  /// No description provided for @catalogWaiting.
  ///
  /// In zh, this message translates to:
  /// **'图库数据等待整理'**
  String get catalogWaiting;

  /// No description provided for @catalogInspecting.
  ///
  /// In zh, this message translates to:
  /// **'正在检查图库数据'**
  String get catalogInspecting;

  /// No description provided for @catalogWaitingIdle.
  ///
  /// In zh, this message translates to:
  /// **'等待空闲后整理图库数据'**
  String get catalogWaitingIdle;

  /// No description provided for @catalogCheckingSpace.
  ///
  /// In zh, this message translates to:
  /// **'正在检查整理空间'**
  String get catalogCheckingSpace;

  /// No description provided for @catalogOptimizing.
  ///
  /// In zh, this message translates to:
  /// **'正在优化图库数据库'**
  String get catalogOptimizing;

  /// No description provided for @catalogReclaiming.
  ///
  /// In zh, this message translates to:
  /// **'正在释放图库数据空间'**
  String get catalogReclaiming;

  /// No description provided for @catalogOptimized.
  ///
  /// In zh, this message translates to:
  /// **'图库数据整理完成'**
  String get catalogOptimized;

  /// No description provided for @catalogOptimizationCancelled.
  ///
  /// In zh, this message translates to:
  /// **'图库数据整理已取消'**
  String get catalogOptimizationCancelled;

  /// No description provided for @catalogOptimizationFailed.
  ///
  /// In zh, this message translates to:
  /// **'图库数据整理失败'**
  String get catalogOptimizationFailed;

  /// No description provided for @reclaimableSpace.
  ///
  /// In zh, this message translates to:
  /// **'约 {size} 空间可以安全回收'**
  String reclaimableSpace(String size);

  /// No description provided for @catalogAutoReclaim.
  ///
  /// In zh, this message translates to:
  /// **'删除图库后会在后台回收数据库空闲页，不会删除原图片'**
  String get catalogAutoReclaim;

  /// No description provided for @catalogRemovalApplied.
  ///
  /// In zh, this message translates to:
  /// **'移除结果已经生效，后台整理即将开始'**
  String get catalogRemovalApplied;

  /// No description provided for @catalogEstimating.
  ///
  /// In zh, this message translates to:
  /// **'正在计算有效数据与可回收空间'**
  String get catalogEstimating;

  /// No description provided for @catalogBusyWait.
  ///
  /// In zh, this message translates to:
  /// **'有更重要的图库操作正在进行，稍后自动继续'**
  String get catalogBusyWait;

  /// No description provided for @catalogCheckingTempSpace.
  ///
  /// In zh, this message translates to:
  /// **'正在确认数据库重建所需的临时磁盘空间'**
  String get catalogCheckingTempSpace;

  /// No description provided for @catalogConverting.
  ///
  /// In zh, this message translates to:
  /// **'正在执行一次性数据库转换，可回收约 {size}；此阶段无法可靠估算百分比'**
  String catalogConverting(String size);

  /// No description provided for @catalogReleasedRemaining.
  ///
  /// In zh, this message translates to:
  /// **'已释放 {released}，剩余约 {remaining}'**
  String catalogReleasedRemaining(String released, String remaining);

  /// No description provided for @catalogReleased.
  ///
  /// In zh, this message translates to:
  /// **'已释放 {released}；有效图库数据保持不变'**
  String catalogReleased(String released);

  /// No description provided for @catalogStopped.
  ///
  /// In zh, this message translates to:
  /// **'已停止整理；图库移除结果与有效数据不受影响，可稍后重试'**
  String get catalogStopped;

  /// No description provided for @catalogRetryLater.
  ///
  /// In zh, this message translates to:
  /// **'未能完成图库数据整理，可稍后重试'**
  String get catalogRetryLater;

  /// No description provided for @spaceRequired.
  ///
  /// In zh, this message translates to:
  /// **'需要 {requiredSize}，当前可用 {available}'**
  String spaceRequired(String requiredSize, String available);

  /// No description provided for @thumbnails.
  ///
  /// In zh, this message translates to:
  /// **'缩略图'**
  String get thumbnails;

  /// No description provided for @cleaningSubject.
  ///
  /// In zh, this message translates to:
  /// **'正在清理{subject}'**
  String cleaningSubject(String subject);

  /// No description provided for @cleanupSubjectComplete.
  ///
  /// In zh, this message translates to:
  /// **'{subject}清理完成'**
  String cleanupSubjectComplete(String subject);

  /// No description provided for @cleanupSubjectCancelled.
  ///
  /// In zh, this message translates to:
  /// **'{subject}清理已取消'**
  String cleanupSubjectCancelled(String subject);

  /// No description provided for @cleanupSubjectFailed.
  ///
  /// In zh, this message translates to:
  /// **'{subject}清理失败'**
  String cleanupSubjectFailed(String subject);

  /// No description provided for @clearThumbnails.
  ///
  /// In zh, this message translates to:
  /// **'清理缩略图'**
  String get clearThumbnails;

  /// No description provided for @thumbnailCleanupHelp.
  ///
  /// In zh, this message translates to:
  /// **'缩略图会在需要时重新生成，不会删除原图片'**
  String get thumbnailCleanupHelp;

  /// No description provided for @countingThumbnails.
  ///
  /// In zh, this message translates to:
  /// **'正在统计可清理的缩略图'**
  String get countingThumbnails;

  /// No description provided for @cleanupFileProgress.
  ///
  /// In zh, this message translates to:
  /// **'已处理 {processed} / {total} 个文件，释放 {size}'**
  String cleanupFileProgress(String processed, String total, String size);

  /// No description provided for @cleanupFilesRemoved.
  ///
  /// In zh, this message translates to:
  /// **'已移除 {count} 个文件，释放 {size}'**
  String cleanupFilesRemoved(String count, String size);

  /// No description provided for @cleanupBeforeStop.
  ///
  /// In zh, this message translates to:
  /// **'停止前已移除 {count} 个文件，已保留原图片'**
  String cleanupBeforeStop(String count);

  /// No description provided for @thumbnailCleanupFailed.
  ///
  /// In zh, this message translates to:
  /// **'未能完成缩略图清理'**
  String get thumbnailCleanupFailed;

  /// No description provided for @cleanupFailedFiles.
  ///
  /// In zh, this message translates to:
  /// **'{count} 个文件未能清理'**
  String cleanupFailedFiles(String count);

  /// No description provided for @chooseThisFolder.
  ///
  /// In zh, this message translates to:
  /// **'选择此目录'**
  String get chooseThisFolder;

  /// No description provided for @catalogCleanupAlreadyFinished.
  ///
  /// In zh, this message translates to:
  /// **'图库数据整理任务已经结束，无法再取消'**
  String get catalogCleanupAlreadyFinished;

  /// No description provided for @folderStatus.
  ///
  /// In zh, this message translates to:
  /// **'“{name}”{status}'**
  String folderStatus(String name, String status);

  /// No description provided for @galleryDay.
  ///
  /// In zh, this message translates to:
  /// **'{date}'**
  String galleryDay(DateTime date);

  /// No description provided for @galleryMonth.
  ///
  /// In zh, this message translates to:
  /// **'{date}'**
  String galleryMonth(DateTime date);

  /// No description provided for @eventDateTime.
  ///
  /// In zh, this message translates to:
  /// **'{date} {time}'**
  String eventDateTime(DateTime date, DateTime time);

  /// No description provided for @labelAndValue.
  ///
  /// In zh, this message translates to:
  /// **'{label}：{value}'**
  String labelAndValue(String label, String value);

  /// No description provided for @inlineListSeparator.
  ///
  /// In zh, this message translates to:
  /// **'、'**
  String get inlineListSeparator;

  /// No description provided for @clauseSeparator.
  ///
  /// In zh, this message translates to:
  /// **'，'**
  String get clauseSeparator;

  /// No description provided for @sentenceSeparator.
  ///
  /// In zh, this message translates to:
  /// **'。'**
  String get sentenceSeparator;

  /// No description provided for @listSentence.
  ///
  /// In zh, this message translates to:
  /// **'{items}。'**
  String listSentence(String items);

  /// Display name for the factory smart album.
  ///
  /// In zh, this message translates to:
  /// **'插画'**
  String get categoryIllustration;

  /// Display name for the factory smart album.
  ///
  /// In zh, this message translates to:
  /// **'漫画'**
  String get categoryComic;

  /// Display name for the factory smart album.
  ///
  /// In zh, this message translates to:
  /// **'表情包'**
  String get categoryMeme;

  /// Display name for the factory smart album.
  ///
  /// In zh, this message translates to:
  /// **'截图'**
  String get categoryScreenshot;

  /// Display name for the factory smart album.
  ///
  /// In zh, this message translates to:
  /// **'实拍'**
  String get categoryPhoto;

  /// Display name for the factory smart album.
  ///
  /// In zh, this message translates to:
  /// **'其他'**
  String get categoryOther;

  /// Automatic smart-album name for the cats topic with photo purpose.
  ///
  /// In zh, this message translates to:
  /// **'猫咪'**
  String get categoryCatsPhoto;

  /// Automatic smart-album name for the cats topic with meme purpose.
  ///
  /// In zh, this message translates to:
  /// **'猫咪表情'**
  String get categoryCatsMeme;

  /// Automatic smart-album name for the dogs topic with photo purpose.
  ///
  /// In zh, this message translates to:
  /// **'狗狗'**
  String get categoryDogsPhoto;

  /// Automatic smart-album name for the dogs topic with meme purpose.
  ///
  /// In zh, this message translates to:
  /// **'狗狗表情'**
  String get categoryDogsMeme;

  /// Automatic smart-album name for the rabbits topic with photo purpose.
  ///
  /// In zh, this message translates to:
  /// **'兔子'**
  String get categoryRabbitsPhoto;

  /// Automatic smart-album name for the birds topic with photo purpose.
  ///
  /// In zh, this message translates to:
  /// **'鸟类'**
  String get categoryBirdsPhoto;

  /// Automatic smart-album name for the wildlife topic with photo purpose.
  ///
  /// In zh, this message translates to:
  /// **'野生动物'**
  String get categoryWildlifePhoto;

  /// Automatic smart-album name for the animal characters topic with illustration purpose.
  ///
  /// In zh, this message translates to:
  /// **'动物角色插画'**
  String get categoryAnimalCharactersIllustration;

  /// Automatic smart-album name for the animal characters topic with meme purpose.
  ///
  /// In zh, this message translates to:
  /// **'动物角色表情'**
  String get categoryAnimalCharactersMeme;

  /// Automatic smart-album name for the animal eared characters topic with illustration purpose.
  ///
  /// In zh, this message translates to:
  /// **'兽耳角色插画'**
  String get categoryAnimalEaredCharactersIllustration;

  /// Automatic smart-album name for the animal eared characters topic with meme purpose.
  ///
  /// In zh, this message translates to:
  /// **'兽耳角色表情'**
  String get categoryAnimalEaredCharactersMeme;

  /// Automatic smart-album name for the portraits topic with photo purpose.
  ///
  /// In zh, this message translates to:
  /// **'人物肖像'**
  String get categoryPortraitsPhoto;

  /// Automatic smart-album name for the groups topic with photo purpose.
  ///
  /// In zh, this message translates to:
  /// **'多人合影'**
  String get categoryGroupsPhoto;

  /// Automatic smart-album name for the costumes topic with photo purpose.
  ///
  /// In zh, this message translates to:
  /// **'角色扮演'**
  String get categoryCostumesPhoto;

  /// Automatic smart-album name for the performances topic with photo purpose.
  ///
  /// In zh, this message translates to:
  /// **'舞台表演'**
  String get categoryPerformancesPhoto;

  /// Automatic smart-album name for the sports topic with photo purpose.
  ///
  /// In zh, this message translates to:
  /// **'体育运动'**
  String get categorySportsPhoto;

  /// Automatic smart-album name for the children playing topic with photo purpose.
  ///
  /// In zh, this message translates to:
  /// **'儿童游玩'**
  String get categoryChildrenPlayingPhoto;

  /// Automatic smart-album name for the toys topic with photo purpose.
  ///
  /// In zh, this message translates to:
  /// **'玩具手办'**
  String get categoryToysPhoto;

  /// Automatic smart-album name for the toys topic with meme purpose.
  ///
  /// In zh, this message translates to:
  /// **'玩偶表情'**
  String get categoryToysMeme;

  /// Automatic smart-album name for the food topic with photo purpose.
  ///
  /// In zh, this message translates to:
  /// **'餐饮美食'**
  String get categoryFoodPhoto;

  /// Automatic smart-album name for the mountains topic with photo purpose.
  ///
  /// In zh, this message translates to:
  /// **'山野风光'**
  String get categoryMountainsPhoto;

  /// Automatic smart-album name for the mountains topic with illustration purpose.
  ///
  /// In zh, this message translates to:
  /// **'山野插画'**
  String get categoryMountainsIllustration;

  /// Automatic smart-album name for the coast topic with photo purpose.
  ///
  /// In zh, this message translates to:
  /// **'海滨风光'**
  String get categoryCoastPhoto;

  /// Automatic smart-album name for the coast topic with illustration purpose.
  ///
  /// In zh, this message translates to:
  /// **'海滨插画'**
  String get categoryCoastIllustration;

  /// Automatic smart-album name for the forests topic with photo purpose.
  ///
  /// In zh, this message translates to:
  /// **'树林'**
  String get categoryForestsPhoto;

  /// Automatic smart-album name for the forests topic with illustration purpose.
  ///
  /// In zh, this message translates to:
  /// **'森林插画'**
  String get categoryForestsIllustration;

  /// Automatic smart-album name for the flowers topic with photo purpose.
  ///
  /// In zh, this message translates to:
  /// **'花草'**
  String get categoryFlowersPhoto;

  /// Automatic smart-album name for the city architecture topic with photo purpose.
  ///
  /// In zh, this message translates to:
  /// **'城市与建筑'**
  String get categoryCityArchitecturePhoto;

  /// Automatic smart-album name for the city architecture topic with illustration purpose.
  ///
  /// In zh, this message translates to:
  /// **'建筑插画'**
  String get categoryCityArchitectureIllustration;

  /// Automatic smart-album name for the interiors topic with photo purpose.
  ///
  /// In zh, this message translates to:
  /// **'室内空间'**
  String get categoryInteriorsPhoto;

  /// Automatic smart-album name for the construction topic with photo purpose.
  ///
  /// In zh, this message translates to:
  /// **'建筑施工'**
  String get categoryConstructionPhoto;

  /// Automatic smart-album name for the vehicles topic with photo purpose.
  ///
  /// In zh, this message translates to:
  /// **'车辆'**
  String get categoryVehiclesPhoto;

  /// Automatic smart-album name for the programming topic with screenshot purpose.
  ///
  /// In zh, this message translates to:
  /// **'代码截图'**
  String get categoryProgrammingScreenshot;

  /// Automatic smart-album name for the messaging topic with screenshot purpose.
  ///
  /// In zh, this message translates to:
  /// **'聊天记录'**
  String get categoryMessagingScreenshot;

  /// Automatic smart-album name for the gaming topic with screenshot purpose.
  ///
  /// In zh, this message translates to:
  /// **'游戏截图'**
  String get categoryGamingScreenshot;

  /// Automatic smart-album name for the document reading topic with screenshot purpose.
  ///
  /// In zh, this message translates to:
  /// **'文档阅读截图'**
  String get categoryDocumentReadingScreenshot;

  /// Automatic smart-album name for the document reading topic with other purpose.
  ///
  /// In zh, this message translates to:
  /// **'文档资料'**
  String get categoryDocumentReadingOther;

  /// Automatic smart-album name for the application interfaces topic with screenshot purpose.
  ///
  /// In zh, this message translates to:
  /// **'应用界面'**
  String get categoryApplicationInterfacesScreenshot;

  /// Automatic smart-album name for the maps topic with screenshot purpose.
  ///
  /// In zh, this message translates to:
  /// **'地图导航截图'**
  String get categoryMapsScreenshot;

  /// Automatic smart-album name for the maps topic with other purpose.
  ///
  /// In zh, this message translates to:
  /// **'地图资料'**
  String get categoryMapsOther;

  /// Automatic smart-album name for the tables charts topic with screenshot purpose.
  ///
  /// In zh, this message translates to:
  /// **'图表截图'**
  String get categoryTablesChartsScreenshot;

  /// Automatic smart-album name for the tables charts topic with other purpose.
  ///
  /// In zh, this message translates to:
  /// **'图表资料'**
  String get categoryTablesChartsOther;

  /// Automatic smart-album name for the notes topic with other purpose.
  ///
  /// In zh, this message translates to:
  /// **'手写笔记'**
  String get categoryNotesOther;

  /// Automatic smart-album name for the mathematics topic with other purpose.
  ///
  /// In zh, this message translates to:
  /// **'数学资料'**
  String get categoryMathematicsOther;

  /// Automatic smart-album name for the music topic with other purpose.
  ///
  /// In zh, this message translates to:
  /// **'乐谱'**
  String get categoryMusicOther;

  /// Automatic smart-album name for the shopping topic with screenshot purpose.
  ///
  /// In zh, this message translates to:
  /// **'购物页面'**
  String get categoryShoppingScreenshot;

  /// Automatic smart-album name for the receipts topic with other purpose.
  ///
  /// In zh, this message translates to:
  /// **'票据'**
  String get categoryReceiptsOther;

  /// Automatic smart-album name for the interface design topic with other purpose.
  ///
  /// In zh, this message translates to:
  /// **'界面设计'**
  String get categoryInterfaceDesignOther;

  /// Automatic smart-album name for the logos topic with other purpose.
  ///
  /// In zh, this message translates to:
  /// **'标志与图标'**
  String get categoryLogosOther;

  /// Automatic smart-album name for the film topic with screenshot purpose.
  ///
  /// In zh, this message translates to:
  /// **'影视画面'**
  String get categoryFilmScreenshot;

  /// Automatic smart-album name for the film topic with meme purpose.
  ///
  /// In zh, this message translates to:
  /// **'影视表情'**
  String get categoryFilmMeme;

  /// Download progress, decimal megabytes received and total.
  ///
  /// In zh, this message translates to:
  /// **'{percent}% · {received} / {total} MB'**
  String modelDownloadProgress(int percent, double received, double total);

  /// Capture or filesystem timestamp in the photo information sheet, including seconds.
  ///
  /// In zh, this message translates to:
  /// **'{date} {time}'**
  String photoTimestamp(DateTime date, DateTime time);
}

class _AppLocalizationsDelegate
    extends LocalizationsDelegate<AppLocalizations> {
  const _AppLocalizationsDelegate();

  @override
  Future<AppLocalizations> load(Locale locale) {
    return SynchronousFuture<AppLocalizations>(lookupAppLocalizations(locale));
  }

  @override
  bool isSupported(Locale locale) =>
      <String>['en', 'ja', 'zh'].contains(locale.languageCode);

  @override
  bool shouldReload(_AppLocalizationsDelegate old) => false;
}

AppLocalizations lookupAppLocalizations(Locale locale) {
  // Lookup logic when language+script codes are specified.
  switch (locale.languageCode) {
    case 'zh':
      {
        switch (locale.scriptCode) {
          case 'Hans':
            return AppLocalizationsZhHans();
        }
        break;
      }
  }

  // Lookup logic when only language code is specified.
  switch (locale.languageCode) {
    case 'en':
      return AppLocalizationsEn();
    case 'ja':
      return AppLocalizationsJa();
    case 'zh':
      return AppLocalizationsZh();
  }

  throw FlutterError(
    'AppLocalizations.delegate failed to load unsupported locale "$locale". This is likely '
    'an issue with the localizations generation tool. Please file an issue '
    'on GitHub with a reproducible sample app and the gen-l10n configuration '
    'that was used.',
  );
}
