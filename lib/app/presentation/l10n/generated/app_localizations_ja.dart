// ignore_for_file: deprecated_member_use

// ignore: unused_import
import 'package:intl/intl.dart' as intl;
import 'app_localizations.dart';

// ignore_for_file: type=lint

/// The translations for Japanese (`ja`).
class AppLocalizationsJa extends AppLocalizations {
  AppLocalizationsJa([String locale = 'ja']) : super(locale);

  @override
  String get settingsTitle => '設定';

  @override
  String get settingsPersonalization => '個人設定';

  @override
  String get settingsLanguage => 'アプリの言語';

  @override
  String get settingsLanguageHelp => 'Ame の表示言語を選びます。';

  @override
  String get languageSystem => 'システム設定に従う';

  @override
  String get languageChinese => '简体中文';

  @override
  String get languageEnglish => 'English';

  @override
  String get languageJapanese => '日本語';

  @override
  String get settingsTheme => 'アプリのテーマ';

  @override
  String get settingsThemeHelp => 'ライトまたはダークを選びます。アクセントカラーは Windows の設定に従います。';

  @override
  String get themeLight => 'ライト';

  @override
  String get themeDark => 'ダーク';

  @override
  String get settingsBrowsing => '閲覧';

  @override
  String get settingsWheel => 'マウスホイール';

  @override
  String get settingsWheelHelp => '写真の表示中にホイールを回したときの動作を選びます。';

  @override
  String get settingsWheelZoom => '拡大・縮小';

  @override
  String get settingsWheelNavigate => '前後の写真に移動';

  @override
  String get settingsOpenPhoto => '写真を開くときの表示';

  @override
  String get settingsOpenPhotoHelp => '「自動」では、大きな写真はウィンドウに合わせ、小さな写真は原寸で表示します。';

  @override
  String get viewerAutomatic => '自動';

  @override
  String get viewerFitWindow => 'ウィンドウに合わせる';

  @override
  String get viewerActualSize => '原寸';

  @override
  String get settingsThumbnailSpeed => 'サムネイルの読み込み速度';

  @override
  String get settingsThumbnailSpeedHelp =>
      '速度を上げると、同時に読み込むサムネイルが増え、パソコンの負荷が高くなります。';

  @override
  String get sizeSmall => '小';

  @override
  String get sizeMedium => '中';

  @override
  String get sizeLarge => '大';

  @override
  String get settingsAbout => 'このアプリについて';

  @override
  String get settingsVersion => 'バージョン';

  @override
  String get settingsLicenses => 'オープンソースライセンス';

  @override
  String get settingsLicensesHelp => 'Ame が使用するオープンソースソフトウェアとライセンスを表示します。';

  @override
  String get settingsSaveFailed => '設定を保存できませんでした';

  @override
  String get startupFailureTitle => 'Cedarflake Ame を起動できませんでした';

  @override
  String get copyDiagnostics => '診断情報をコピー';

  @override
  String get diagnosticsCopied => '診断情報をコピーしました';

  @override
  String get startupUnsupportedCatalog =>
      'このライブラリは未対応のバージョンで作成されています。問題を調べるには、診断情報をコピーしてください。';

  @override
  String get startupDatabaseError =>
      'ライブラリを読み込めませんでした。ドライブが利用できることを確認して、Ame を再起動してください。それでも開けない場合は診断情報をコピーしてください。';

  @override
  String get startupCatalogError =>
      'Ame を再起動してライブラリを開き直してください。それでも開けない場合は診断情報をコピーしてください。';

  @override
  String get startupComponentError =>
      'アプリの一部を読み込めませんでした。Ame を再起動してください。それでも起動しない場合は診断情報をコピーしてください。';

  @override
  String get choose => '選択';

  @override
  String get notSet => '未設定';

  @override
  String get notifications => '通知';

  @override
  String get notificationsEmpty => '通知はありません';

  @override
  String get notificationHistory => '履歴';

  @override
  String get acknowledge => '確認';

  @override
  String get notificationDetails => '通知の詳細';

  @override
  String get sourceDirectory => 'フォルダー';

  @override
  String get technicalCode => '診断コード';

  @override
  String get close => '閉じる';

  @override
  String get operationFailedDetails => '操作が完了しませんでした。詳細を確認できます。';

  @override
  String get retry => '再試行';

  @override
  String get errorCode => 'エラーコード';

  @override
  String get openLocation => '保存場所を開く';

  @override
  String get openLocationFailed => 'この場所を開けませんでした。もう一度お試しください。';

  @override
  String get loading => '読み込み中';

  @override
  String elapsedTime(String duration) {
    return '経過時間 $duration';
  }

  @override
  String get occurredAt => '発生時刻';

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
    return '$hours 時間 $minutes 分';
  }

  @override
  String get notificationsUnread => '通知、未読あり';

  @override
  String get notificationsRead => '通知、未読なし';

  @override
  String get windowMinimize => '最小化';

  @override
  String get windowRestore => '元のサイズに戻す';

  @override
  String get windowMaximize => '最大化';

  @override
  String get chooseFolder => 'フォルダーを選択';

  @override
  String get albumCreated => 'アルバムを作成しました';

  @override
  String get albumRenamed => 'アルバム名を変更しました';

  @override
  String get albumIconChanged => 'アルバムのアイコンを変更しました';

  @override
  String get albumDeleted => 'アルバムを削除しました';

  @override
  String get removedFromAlbum => 'アルバムから外しました';

  @override
  String movedToAlbum(String name) {
    return '「$name」に移動しました';
  }

  @override
  String addedToAlbum(String name) {
    return '「$name」に追加しました';
  }

  @override
  String get archiveExported => '整理記録をエクスポートしました';

  @override
  String get backupSaved => '選択した場所にバックアップを保存しました。';

  @override
  String get archiveRestored => '整理記録を復元しました';

  @override
  String get unlinkedRecordsKept => '写真に関連付けられていない記録も保持しました。';

  @override
  String get favoriteAdded => 'お気に入りに追加しました';

  @override
  String get favoriteRemoved => 'お気に入りから外しました';

  @override
  String get unlinkedRecordsCleared => '写真に関連付けられていない記録を消去しました';

  @override
  String recordsCleared(int count) {
    final intl.NumberFormat countNumberFormat =
        intl.NumberFormat.decimalPattern(localeName);
    final String countString = countNumberFormat.format(count);

    return '記録を $countString 件消去しました。写真とアルバムは保持されています。';
  }

  @override
  String get changesSaveFailed => '変更を保存できませんでした';

  @override
  String get organizationRecordChanged => '写真または整理記録が変更されました。現在の記録を保持しました。';

  @override
  String get organizationHistoryChanged => '操作履歴が変更されました。メニューを開き直してください。';

  @override
  String get organizationSelectionChanged => '写真または整理情報が変更されました。もう一度お試しください。';

  @override
  String get organizationSelectionExpired => '写真または整理情報が変更されました。写真を選び直してください。';

  @override
  String get changesUnconfirmed => '変更が保存されたか確認できませんでした。もう一度お試しください。';

  @override
  String get undoCompleted => '元に戻しました';

  @override
  String get redoCompleted => 'やり直しました';

  @override
  String get undoFailed => '元に戻せませんでした';

  @override
  String get redoFailed => 'やり直せませんでした';

  @override
  String get undo => '元に戻す';

  @override
  String get changesSaved => '変更を保存しました';

  @override
  String get libraryRefreshPending => 'ライブラリの表示にまだ反映されていません。もう一度お試しください。';

  @override
  String get locationRefreshFailed => 'ライブラリ内のファイルの場所を更新できませんでした。もう一度お試しください。';

  @override
  String get displayRefreshSuperseded =>
      'ライブラリは保存されました。別の操作で表示の更新が中断されました。表示を更新してください。';

  @override
  String get deferredOrganization => '「あとで整理」に移動しました';

  @override
  String get ignored => '除外済み';

  @override
  String get restoredToInbox => '「未整理」に戻しました';

  @override
  String get relocationUnsupported => '現在のスキャンサービスでは、このライブラリの場所を変更できません。';

  @override
  String get albumRemovalRemaining => 'アルバムの情報が変更されました。残りの項目でもう一度お試しください。';

  @override
  String albumRemovalFailed(String name) {
    return '「$name」を削除できませんでした。もう一度お試しください。';
  }

  @override
  String get operationStopped => '操作を停止しました。';

  @override
  String get operationIncomplete => '操作を完了できませんでした。もう一度お試しください。';

  @override
  String get folderStillUpdating =>
      'フォルダーの内容が変更されたか、まだ更新中です。しばらくしてからもう一度お試しください。';

  @override
  String get folderRemovedRefreshPending =>
      'フォルダーを外しました。このウィンドウを閉じてライブラリを更新してください。';

  @override
  String get folderRemovalFailed => 'フォルダーを外せませんでした。もう一度お試しください。';

  @override
  String get scanEndedWithoutResult => '結果を取得する前にライブラリの更新が終了しました。';

  @override
  String get updatePausedRestart => 'ライブラリの更新を一時停止しました。更新をもう一度開始してください。';

  @override
  String get sourceChangedDuringUpdate => '更新中に元のファイルが変更されました。もう一度お試しください。';

  @override
  String get updateSourceChanged => '読み込み元が変更されました。選び直してください。';

  @override
  String get addToAlbum => 'アルバムに追加';

  @override
  String get moveToAnotherAlbum => '別のアルバムに移動';

  @override
  String get addOrMoveToAlbum => 'アルバムに追加または移動';

  @override
  String filterFormat(String format) {
    return '形式：$format';
  }

  @override
  String filterWidth(String range) {
    return '幅：$range px';
  }

  @override
  String filterHeight(String range) {
    return '高さ：$range px';
  }

  @override
  String filterCamera(String camera) {
    return 'カメラ：$camera';
  }

  @override
  String filterFileSize(String range, String unit) {
    return 'ファイルサイズ：$range $unit';
  }

  @override
  String get organizationEnterFailed => '整理モードを開始できませんでした。もう一度お試しください。';

  @override
  String get galleryReturnFailed => 'ライブラリに戻れませんでした。もう一度お試しください。';

  @override
  String get organizationScopeFailed => '整理対象を切り替えられませんでした。もう一度お試しください。';

  @override
  String get conflictOpenFailed => 'アルバムの指定が競合している写真を開けませんでした。もう一度お試しください。';

  @override
  String get organizationPositionUnsaved => '整理を中断した位置を保存できませんでした。もう一度お試しください。';

  @override
  String get organizationPositionUnreadable => '保存した位置を読み込めませんでした。もう一度お試しください。';

  @override
  String get organizationResumeFailed => '前回の位置から整理を再開できませんでした。もう一度お試しください。';

  @override
  String get organizationNextFailed => '変更を保存しました。次の写真をもう一度開いてください。';

  @override
  String get organizationLibraryUpdating => 'ライブラリを更新中です。写真をもう一度切り替えてください。';

  @override
  String get organizationViewChanged => '表示内容が変わりました。写真をもう一度切り替えてください。';

  @override
  String get unknown => '不明';

  @override
  String get smartAnalysisInterrupted => 'スマートアルバムの解析が中断されました';

  @override
  String get smartResultsAvailable => 'これまでの結果は引き続き閲覧できます。';

  @override
  String get albumConflict => 'アルバムの競合';

  @override
  String conflictsDeferred(int count) {
    final intl.NumberFormat countNumberFormat =
        intl.NumberFormat.decimalPattern(localeName);
    final String countString = countNumberFormat.format(count);

    return 'アルバムを選ぶ必要がある $countString グループを「あとで整理」に移動しました。';
  }

  @override
  String get viewAction => '表示';

  @override
  String get duplicateWorkspace => '重複写真を探す';

  @override
  String get openInExplorer => 'エクスプローラーで開く';

  @override
  String get openInExplorerFailed => 'エクスプローラーを開けませんでした';

  @override
  String get stopping => '停止中';

  @override
  String get duplicateRunning => '重複写真を検索中';

  @override
  String get duplicateIncomplete => '重複写真の検索が完了しませんでした';

  @override
  String get waitingForBackground => 'バックグラウンド処理の完了を待っています';

  @override
  String get paused => '一時停止中';

  @override
  String get duplicateEmptyAfter => '確認した写真に重複はありません';

  @override
  String get duplicateWaitResults => '重複写真が見つかるとここに表示されます。';

  @override
  String get duplicateLocationsChanged => 'このグループ内の一部の写真は保存場所が変わりました';

  @override
  String get resultsUnavailable => '結果を読み込めませんでした';

  @override
  String get viewDuplicates => '重複写真を表示';

  @override
  String get backToDuplicates => '重複写真に戻る';

  @override
  String get duplicateGroups => '重複グループ';

  @override
  String get ignoredGroupsEmpty => '除外したグループはありません';

  @override
  String get similarGroups => '類似グループ';

  @override
  String get similarityLevel => '類似度';

  @override
  String get similarityStrict => '厳密';

  @override
  String get similarityStandard => '標準';

  @override
  String get similarityBroad => '広め';

  @override
  String get viewSimilar => '類似写真を表示';

  @override
  String get similarEmpty => '確認した写真に類似グループはありません';

  @override
  String get similarWaitResults => '類似写真が見つかるとここに表示されます';

  @override
  String get comparePhotos => '写真を比較';

  @override
  String get chooseComparison => '比較する写真を選択';

  @override
  String get syncComparison => '拡大・縮小と表示位置を同期';

  @override
  String get resetComparison => '比較表示をリセット';

  @override
  String get zoomIn => '拡大';

  @override
  String get zoomOut => '縮小';

  @override
  String get setPreferred => '優先する写真に設定';

  @override
  String get selectPhoto => '写真を選択';

  @override
  String get deselectPhoto => '写真の選択を解除';

  @override
  String get clearPreferred => '優先設定を解除';

  @override
  String get ignoreGroup => 'グループを除外';

  @override
  String get restoreGroup => 'グループを元に戻す';

  @override
  String get selectionSaveFailed => '選択を保存できませんでした。もう一度お試しください。';

  @override
  String groupCount(int count) {
    final intl.NumberFormat countNumberFormat =
        intl.NumberFormat.decimalPattern(localeName);
    final String countString = countNumberFormat.format(count);

    return '$countString グループ';
  }

  @override
  String photoCount(int count) {
    final intl.NumberFormat countNumberFormat =
        intl.NumberFormat.decimalPattern(localeName);
    final String countString = countNumberFormat.format(count);

    return '$countString 枚の写真';
  }

  @override
  String deselectNamedGroup(String name) {
    return '写真グループ「$name」の選択を解除';
  }

  @override
  String selectNamedGroup(String name) {
    return '写真グループ「$name」を選択';
  }

  @override
  String get filter => '絞り込み';

  @override
  String get showSubfolders => 'サブフォルダーを表示';

  @override
  String get hideSubfolders => 'サブフォルダーを非表示';

  @override
  String get allPhotosFilter => 'すべての写真';

  @override
  String locationCount(int count) {
    final intl.NumberFormat countNumberFormat =
        intl.NumberFormat.decimalPattern(localeName);
    final String countString = countNumberFormat.format(count);

    return '$countString か所';
  }

  @override
  String duplicatePhotoCount(int count) {
    final intl.NumberFormat countNumberFormat =
        intl.NumberFormat.decimalPattern(localeName);
    final String countString = countNumberFormat.format(count);

    return '重複写真 $countString 枚';
  }

  @override
  String physicalFileCount(int count) {
    final intl.NumberFormat countNumberFormat =
        intl.NumberFormat.decimalPattern(localeName);
    final String countString = countNumberFormat.format(count);

    return '$countString ファイル';
  }

  @override
  String get cloudOnly => 'クラウドのみ';

  @override
  String get folderOffline => 'フォルダーはオフラインです';

  @override
  String get temporarilyInaccessible => '一時的に利用できません';

  @override
  String get deleteChangedFile => '変更されたファイルを残しました。最新の写真を確認し、もう一度選択してください。';

  @override
  String get noPhotosToDelete => '削除する写真はありません';

  @override
  String aboutToDeletePhotos(int count) {
    final intl.NumberFormat countNumberFormat =
        intl.NumberFormat.decimalPattern(localeName);
    final String countString = countNumberFormat.format(count);

    return '$countString 枚の写真を削除します';
  }

  @override
  String confirmDeleteFiles(int count) {
    final intl.NumberFormat countNumberFormat =
        intl.NumberFormat.decimalPattern(localeName);
    final String countString = countNumberFormat.format(count);

    return '$countString 個のファイルを削除しますか？';
  }

  @override
  String confirmDeleteWholeGroup(int count) {
    final intl.NumberFormat countNumberFormat =
        intl.NumberFormat.decimalPattern(localeName);
    final String countString = countNumberFormat.format(count);

    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: 'このグループのファイル $countString 個をすべて削除しますか？',
      one: 'このグループにある唯一のファイルを削除しますか？',
    );
    return '$_temp0';
  }

  @override
  String confirmDeleteGroups(int groups, int files) {
    final intl.NumberFormat groupsNumberFormat =
        intl.NumberFormat.decimalPattern(localeName);
    final String groupsString = groupsNumberFormat.format(groups);
    final intl.NumberFormat filesNumberFormat =
        intl.NumberFormat.decimalPattern(localeName);
    final String filesString = filesNumberFormat.format(files);

    return '写真 $groupsString グループに含まれる $filesString 個のファイルを削除しますか？';
  }

  @override
  String get destinationFolderDirect => '保存先フォルダーの直下';

  @override
  String get byCategory => 'カテゴリ別';

  @override
  String get byAlbum => 'アルバム別';

  @override
  String get byDate => '日付別';

  @override
  String get bySourceFolder => '元のフォルダー別';

  @override
  String get byAlbumYear => 'アルバム・年別';

  @override
  String get dateFolderRule =>
      'ライブラリの日付を使って「YYYY-MM」形式のフォルダーを作成します。日付のないファイルはスキップします。';

  @override
  String get albumYearRule => 'ライブラリの日付を使って「旅行/2024」のようなアルバム別・年別のフォルダーに整理します。';

  @override
  String get unknownDestinationRule =>
      '保存先が決まらないファイルはスキップし、保存先フォルダーにレポートを保存します。';

  @override
  String get copy => 'コピー';

  @override
  String get move => '移動';

  @override
  String get rename => '名前の変更';

  @override
  String get deleteAction => '削除';

  @override
  String filesCompleted(String action, int count) {
    final intl.NumberFormat countNumberFormat =
        intl.NumberFormat.decimalPattern(localeName);
    final String countString = countNumberFormat.format(count);

    return '$action：$countString ファイル';
  }

  @override
  String get deleted => '削除済み';

  @override
  String get completed => '完了';

  @override
  String filesIncomplete(int count) {
    final intl.NumberFormat countNumberFormat =
        intl.NumberFormat.decimalPattern(localeName);
    final String countString = countNumberFormat.format(count);

    return '未完了：$countString';
  }

  @override
  String filesStopped(int count) {
    final intl.NumberFormat countNumberFormat =
        intl.NumberFormat.decimalPattern(localeName);
    final String countString = countNumberFormat.format(count);

    return '停止：$countString';
  }

  @override
  String filesSkipped(int count) {
    final intl.NumberFormat countNumberFormat =
        intl.NumberFormat.decimalPattern(localeName);
    final String countString = countNumberFormat.format(count);

    return 'スキップ：$countString';
  }

  @override
  String get noFilesProcessed => '処理したファイルはありません。';

  @override
  String get reportNotNeeded => 'レポートは不要です';

  @override
  String get reportSaved => 'レポートを保存しました';

  @override
  String get reportFailed => 'レポートを作成できませんでした';

  @override
  String get checkFileOperation => 'ファイル操作を確認';

  @override
  String get generateReport => 'レポートを作成';

  @override
  String preparingOperation(String action) {
    return '準備中：$action';
  }

  @override
  String runningOperation(String action) {
    return '実行中：$action';
  }

  @override
  String operationEnded(String action) {
    return '終了：$action';
  }

  @override
  String operationStoppedLabel(String action) {
    return '停止：$action';
  }

  @override
  String get fileOperationNeedsAttention => 'ファイル操作の確認が必要です';

  @override
  String get notStarted => '未開始';

  @override
  String get verifyResults => '結果の確認が必要です';

  @override
  String get moveIncomplete => '移動が完了していません';

  @override
  String get filesProcessedRefreshPending => 'ファイルの処理が終了しました。ライブラリの更新を待っています';

  @override
  String get incomplete => '未完了';

  @override
  String get stopped => '停止しました';

  @override
  String get resultsToVerify => '確認待ちの結果';

  @override
  String get duplicateVerificationPending =>
      '重複グループを確認中です。しばらくしてからもう一度お試しください。';

  @override
  String get selectedPhotosChanged => '選択した写真に変更がありました。ライブラリを確認し、写真を選び直してください。';

  @override
  String get selectedFilesChanged => '選択したファイルに変更がありました。内容を確認し、ファイルを選び直してください。';

  @override
  String get deleteFilesUnverified =>
      'ファイルに変更があったか、状態を確認できませんでした。最新の写真を確認し、削除する写真を選び直してください。';

  @override
  String get filesUnverified =>
      'ファイルに変更があったか、状態を確認できませんでした。最新のファイルを確認し、選び直してください。';

  @override
  String get multipleFileLocations =>
      'これらの写真には複数の保存場所があります。選択を解除して、各写真の右クリックメニューから操作してください。';

  @override
  String get destinationMatchesSource =>
      '保存先が元のファイルと同じです。別のフォルダーかファイル名を選んでください。';

  @override
  String get destinationUnavailable => '保存先フォルダーを開けませんでした。フォルダーを選び直してください。';

  @override
  String get selectedFilesUnreadable =>
      '選択したファイルを読み込めませんでした。ライブラリを更新し、ファイルを選び直してください。';

  @override
  String get retainedPhotoChanged =>
      '残す写真が変更されたか、読み込めませんでした。残す写真を選び直してください。ファイルは削除されていません。';

  @override
  String get fileNameUnavailable => 'このファイル名は使用できません。別の名前を入力してください。';

  @override
  String get fileOperationPrepareFailed =>
      'ファイル操作を準備できませんでした。ファイルを選び直してもう一度お試しください。';

  @override
  String get fileOperationBusy => '別のファイル操作を実行中です。しばらくしてからもう一度お試しください。';

  @override
  String get fileHistoryCheckRequired => '続ける前にファイル操作履歴で結果を確認してください。';

  @override
  String get fileOperationCancelled => '操作をキャンセルしました。';

  @override
  String get reportWriteFailed => '操作結果は保存されています。履歴からレポートを作成し直せます。';

  @override
  String get fileProcessingUncertain =>
      '確認が必要なファイルが残っている可能性があります。ファイル操作履歴を確認してください。';

  @override
  String get moreFilters => 'その他の絞り込み条件';

  @override
  String get filterPhotos => '写真を絞り込む';

  @override
  String get clearFilters => '絞り込みを解除';

  @override
  String get cancel => 'キャンセル';

  @override
  String get apply => '適用';

  @override
  String get date => '日付';

  @override
  String get dateField => '日付の種類';

  @override
  String get startDate => '開始日';

  @override
  String get endDate => '終了日';

  @override
  String get fileFormat => 'ファイル形式';

  @override
  String get fileSize => 'ファイルサイズ';

  @override
  String get width => '幅';

  @override
  String get height => '高さ';

  @override
  String get minimum => '最小';

  @override
  String get maximum => '最大';

  @override
  String get unit => '単位';

  @override
  String get dimensions => '画像の寸法';

  @override
  String get minimumWidth => '最小幅';

  @override
  String get maximumWidth => '最大幅';

  @override
  String get minimumHeight => '最小の高さ';

  @override
  String get maximumHeight => '最大の高さ';

  @override
  String get cameraInformation => 'カメラ情報';

  @override
  String get cameraHint => 'メーカーまたは機種';

  @override
  String get invalidExtension =>
      '形式は最大 16 種類まで指定できます。各形式は半角英数字 16 文字以内で入力してください。';

  @override
  String get invalidNumber => '0 以上の数値を入力してください。';

  @override
  String get invalidInteger => '0 以上の整数を入力してください。';

  @override
  String get fractionalBytes => 'バイトに換算した値が整数になるように指定してください。';

  @override
  String get numberTooLarge => '数値が大きすぎます。';

  @override
  String get reversedRange => '最大値は最小値以上にしてください。';

  @override
  String get invalidDate => '「2026-01-31」のように、有効な日付を入力してください。';

  @override
  String get reversedDates => '終了日には開始日以降の日付を指定してください。';

  @override
  String get invalidCamera => 'メーカーまたは機種を 128 文字以内で入力してください。';

  @override
  String get captureDate => '撮影日';

  @override
  String get createdDate => '作成日';

  @override
  String get modifiedDate => '更新日';

  @override
  String get favorites => 'お気に入り';

  @override
  String get albums => 'アルバム';

  @override
  String get smartAlbums => 'スマートアルバム';

  @override
  String get modelDownloading => 'スマートアルバムのモデルをダウンロード中';

  @override
  String get modelVerifying => 'モデルを確認中';

  @override
  String get modelEnabling => 'スマートアルバムを有効にしています';

  @override
  String get modelEnabled => 'スマートアルバムを有効にしました';

  @override
  String get modelEnableCancelled => 'スマートアルバムの設定をキャンセルしました';

  @override
  String get modelEnableFailed => 'スマートアルバムを有効にできませんでした';

  @override
  String get modelPreparing => 'スマートアルバムのモデルを準備中';

  @override
  String get modelConfigUnreadable => 'モデルの設定を読み込めませんでした。もう一度お試しください。';

  @override
  String get modelDownloadRetained => 'ダウンロードの進行状況を保存しました。再試行できます。';

  @override
  String get modelEnableRetry =>
      'スマートアルバムをもう一度有効にしてください。ダウンロード済みのモデルファイルは再利用されます。';

  @override
  String get albumNameInvalid => 'アルバム名を 1～128 文字で入力してください。';

  @override
  String get albumNameTaken => '同じ名前のアルバムがあります。別の名前を入力してください。';

  @override
  String get albumMissing => 'このアルバムはすでに削除されています。別のアルバムを選んでください。';

  @override
  String get organizationDataChanged =>
      '写真またはアルバムの情報が変更されました。内容を確認してもう一度お試しください。';

  @override
  String get organizationPositionChanged => '整理位置が変更されました。この写真を開き直してください。';

  @override
  String get organizationReselectPhotos =>
      '写真または整理情報が変更されました。キャンセルして写真を選び直してください。';

  @override
  String get noActionablePhotos => 'この操作の対象となる写真がありません。写真を選び直してください。';

  @override
  String get previousOperationRunning => '前の操作がまだ実行中です。しばらくしてからもう一度お試しください。';

  @override
  String get chooseConflictAlbum => 'この写真のアルバム指定が競合しています。アルバムを 1 つ選んでください。';

  @override
  String get photoInformationUpdating => '写真の情報を更新中です。しばらくしてからもう一度お試しください。';

  @override
  String get saveResultUnknown => '保存できたか確認できませんでした。もう一度お試しください。';

  @override
  String get stoppingCurrentTask => '現在のタスクを停止しています';

  @override
  String get resumeOrCancelPaused => '先に、一時停止中のタスクを再開するかキャンセルしてください';

  @override
  String get loadingLibrary => 'ライブラリを読み込み中…';

  @override
  String get selectingFolder => 'フォルダーを選択中…';

  @override
  String checkingFolderPhotos(String name) {
    return '「$name」の写真を確認中…';
  }

  @override
  String updatingFolder(String name) {
    return '「$name」を更新中…';
  }

  @override
  String addingFolder(String name) {
    return '「$name」を追加中…';
  }

  @override
  String get pausingEllipsis => '一時停止しています…';

  @override
  String get stoppingEllipsis => '停止しています…';

  @override
  String get loadingPhotos => '写真を読み込み中…';

  @override
  String get updateLibraryCancelled => 'ライブラリの更新をキャンセルしました';

  @override
  String get addFolderCancelled => 'フォルダーの追加をキャンセルしました';

  @override
  String get updateLibraryPaused => 'ライブラリの更新を一時停止しました';

  @override
  String get addFolderPaused => 'フォルダーの追加を一時停止しました';

  @override
  String get filesChangedRetry => 'ファイルに変更がありました。もう一度お試しください。';

  @override
  String get updateLibraryFailed => 'ライブラリを更新できませんでした';

  @override
  String get addFolderFailed => 'フォルダーを追加できませんでした';

  @override
  String get updateComplete => '更新が完了しました';

  @override
  String get addComplete => 'フォルダーを追加しました';

  @override
  String totalPhotos(int count) {
    final intl.NumberFormat countNumberFormat =
        intl.NumberFormat.decimalPattern(localeName);
    final String countString = countNumberFormat.format(count);

    return '写真は全部で $countString 枚です';
  }

  @override
  String photosAdded(int count) {
    final intl.NumberFormat countNumberFormat =
        intl.NumberFormat.decimalPattern(localeName);
    final String countString = countNumberFormat.format(count);

    return '写真を $countString 枚追加しました';
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

    return '確認済みの写真：$checkedString/$totalString 枚・確認済みのファイル：$filesString 個';
  }

  @override
  String filesCheckedPhotosFound(int files, int photos) {
    final intl.NumberFormat filesNumberFormat =
        intl.NumberFormat.decimalPattern(localeName);
    final String filesString = filesNumberFormat.format(files);
    final intl.NumberFormat photosNumberFormat =
        intl.NumberFormat.decimalPattern(localeName);
    final String photosString = photosNumberFormat.format(photos);

    return '確認済みのファイル：$filesString 個・見つかった写真：$photosString 枚';
  }

  @override
  String get chooseFolderAgain => 'フォルダーを選び直す';

  @override
  String get retryRemoval => 'もう一度外す';

  @override
  String get photos => '写真';

  @override
  String get removeFromLater => '「あとで整理」から外す';

  @override
  String get organizeLater => 'あとで整理';

  @override
  String get unignore => '除外を解除';

  @override
  String get ignoreSelected => '選択した写真を除外';

  @override
  String get ignorePhoto => '写真を除外';

  @override
  String get photoAnalysisPaused => '写真の解析を一時停止しました';

  @override
  String get searchUnavailable => '検索は一時的に利用できません';

  @override
  String get retryAnalysis => '解析を再試行';

  @override
  String get searchContent => '画像の内容';

  @override
  String get searchNamePath => 'ファイル名とパス';

  @override
  String get searchCurrentScope => '現在の表示範囲';

  @override
  String get searchEntireLibrary => 'ライブラリ全体';

  @override
  String get findRelatedPhotos => '関連する写真を探す';

  @override
  String get updateSearchResults => '検索結果を更新';

  @override
  String get retrySearch => '検索を再試行';

  @override
  String get contentSearchHint => '探したい写真の内容を入力';

  @override
  String get fileSearchHint => 'ファイル名やパスを検索';

  @override
  String get contentSearchEmptyHint =>
      '別の言葉で検索するか、絞り込み条件を変更するか、ライブラリ全体を検索してください。';

  @override
  String get filterSearchEmptyHint => '絞り込み条件を変更するか解除してください。';

  @override
  String get searchNeedsModel => '先に「設定」から写真解析モデルをインストールしてください。';

  @override
  String get searchDescriptionTooLong => '検索する内容を 512 文字以内で入力してください。';

  @override
  String get searchPartialAnalysis =>
      '未解析の写真があります。取得済みの結果は保持されています。解析を再試行してください。';

  @override
  String get searchLibraryChanged => 'ライブラリが変更されました。もう一度検索してください。';

  @override
  String get searchPhotoChanged => 'この写真は変更されたか、読み込めませんでした。選び直してください。';

  @override
  String get searchAnalysisUnavailable => '写真の解析を利用できません。もう一度検索してください。';

  @override
  String get searchIndexDamaged => '検索データが破損しています。再試行すると再構築されます。';

  @override
  String get searchStorageUnavailable => '検索データにアクセスできませんでした。もう一度お試しください。';

  @override
  String get searchFailedRetained => '検索を利用できません。前の写真一覧を表示しています。もう一度お試しください。';

  @override
  String searchCoverage(int indexed, int total, String status) {
    final intl.NumberFormat indexedNumberFormat =
        intl.NumberFormat.decimalPattern(localeName);
    final String indexedString = indexedNumberFormat.format(indexed);
    final intl.NumberFormat totalNumberFormat =
        intl.NumberFormat.decimalPattern(localeName);
    final String totalString = totalNumberFormat.format(total);

    return '関連度順・解析済みの写真：$indexedString/$totalString 枚$status';
  }

  @override
  String get analysisStorageFullSuffix => '・解析用のストレージがいっぱいです';

  @override
  String get partialResults => '一部の結果';

  @override
  String get showFilmstrip => 'サムネイルを表示';

  @override
  String get hideFilmstrip => 'サムネイルを非表示';

  @override
  String get scrollFilmstripLeft => '左へスクロール';

  @override
  String get scrollFilmstripRight => '右へスクロール';

  @override
  String get fitImageToWindow => 'ウィンドウに合わせる（0 / Ctrl+0）';

  @override
  String get showImageActualSize => '原寸で表示（1 / Ctrl+1）';

  @override
  String get enterViewerFullscreen => '全画面表示（F11）';

  @override
  String get exitViewerFullscreen => '全画面表示を終了（Esc / F11）';

  @override
  String get clearSearch => '検索をクリア';

  @override
  String get library => 'ライブラリ';

  @override
  String get allImages => 'すべての写真';

  @override
  String get addFolder => 'ライブラリにフォルダーを追加';

  @override
  String get noFolder => 'フォルダーはまだ追加されていません';

  @override
  String get expandFolder => 'フォルダーを展開';

  @override
  String get collapseFolder => 'フォルダーを折りたたむ';

  @override
  String get loadingFolders => 'フォルダーを読み込み中…';

  @override
  String get showMoreFolders => 'フォルダーをさらに表示';

  @override
  String get retryFolders => 'フォルダーを再読み込み';

  @override
  String get synchronized => '最新の状態';

  @override
  String get synchronizing => '更新中';

  @override
  String get backgroundVerification => '確認中';

  @override
  String get needsReconciliation => '更新できません';

  @override
  String get synchronizationUnconfirmed => '未確認';

  @override
  String get synchronizationRetryPending => '再試行待ち';

  @override
  String get continuityLiveOnly => '自動更新は利用できません';

  @override
  String get continuityUnavailable => '自動更新できません';

  @override
  String get sourceUnavailable => '利用できません';

  @override
  String get sourceAvailable => '利用可能';

  @override
  String get sourceMissing => 'フォルダーが見つかりません';

  @override
  String get sourceOffline => 'オフライン';

  @override
  String get sourceUnknown => '状態不明';

  @override
  String get addingSource => '追加中';

  @override
  String get refreshingSourceDisplay => '表示を更新中';

  @override
  String get sourceAdded => '追加済み';

  @override
  String get sourceDisplayReloadPending => '再読み込みが必要です';

  @override
  String get sourceScanResultReadFailed => '結果を読み込めませんでした';

  @override
  String get sourceReadingScanResult => '結果を読み込み中';

  @override
  String get layout => 'レイアウト';

  @override
  String get sort => '並べ替え';

  @override
  String get fileName => '名前';

  @override
  String get newestAddedFirst => '追加が新しい順';

  @override
  String get ascending => '昇順';

  @override
  String get descending => '降順';

  @override
  String get equalHeight => '高さをそろえる';

  @override
  String get square => '正方形';

  @override
  String get sizeMediumLong => '中';

  @override
  String get more => 'その他';

  @override
  String get selectAll => 'すべて選択';

  @override
  String get unknownCaptureDate => '撮影日不明';

  @override
  String get open => '開く';

  @override
  String get viewInformation => '情報を表示';

  @override
  String get copyPath => 'パスをコピー';

  @override
  String get updateLibrary => 'ライブラリを更新';

  @override
  String get relocateLibrary => 'フォルダーの場所を指定し直す';

  @override
  String get removeFromAme => 'Ame から外す';

  @override
  String get updatingLibrary => 'ライブラリを更新中…';

  @override
  String get emptyLibraryTitle => '写真ライブラリを作成';

  @override
  String get emptyLibraryBody => 'フォルダーを選ぶと、そこにある写真を Ame でまとめて閲覧できます。';

  @override
  String get importPhotos => 'インポート';

  @override
  String get firstImportIncomplete => 'フォルダーの追加が完了していません';

  @override
  String get firstImportInProgress => '写真を追加しています';

  @override
  String get firstImportInProgressDetail => 'フォルダーの追加が完了すると、ここに写真が表示されます。';

  @override
  String get pausingSource => '一時停止しています';

  @override
  String get firstImportPaused => '写真の追加を一時停止しました';

  @override
  String get firstImportPausedDetail => '進行状況を保存しました。あとで続きから再開できます。';

  @override
  String get cancellingSource => 'キャンセルしています';

  @override
  String get firstImportCancelled => '写真の追加をキャンセルしました';

  @override
  String get firstImportRetryDetail => 'このフォルダーをライブラリに追加できませんでした。';

  @override
  String get firstImportGalleryDetail => '必要なときに、このフォルダーを再度追加できます。';

  @override
  String get scanConfirmationPending => 'スキャン結果を読み込めませんでした';

  @override
  String get scanConfirmationDetail => 'スキャンした写真はまだ表示できません。';

  @override
  String get readingScanResult => 'スキャン結果を読み込み中…';

  @override
  String get rereadScanResult => '結果を再読み込み';

  @override
  String get galleryDisplayReloadPending => '写真を読み込めませんでした';

  @override
  String get galleryDisplayReloadDetail => '写真は追加されましたが、まだ表示できません。';

  @override
  String get addFolderAwaitingScanResult =>
      'スキャン結果を読み込めませんでした。しばらく待ってから別のフォルダーを追加してください。';

  @override
  String get addFolderAwaitingGalleryReload =>
      'ライブラリを読み込めませんでした。しばらく待ってから別のフォルダーを追加してください。';

  @override
  String get noSearchResults => '条件に合う写真がありません';

  @override
  String get noSearchResultsHint => '別の名前やパスで検索してください。';

  @override
  String get noSourceResults => 'このフォルダーには表示できる写真がありません';

  @override
  String get noSourceResultsHint => '別のフォルダーを選ぶか、ライブラリに戻ってください。';

  @override
  String get retryLoading => '再読み込み';

  @override
  String get retryPreview => 'プレビューを再読み込み';

  @override
  String get retryingPreview => 'プレビューを読み込み中…';

  @override
  String get previewFormatUnrecognized => '認識できない画像形式です';

  @override
  String get previewUpdateRequired => 'ライブラリを更新してから、プレビューを再読み込みしてください。';

  @override
  String get backToLibrary => 'ライブラリに戻る';

  @override
  String get originalImageUnavailable => '元の写真を開けませんでした';

  @override
  String get originalImagePreviewFallback => '元の写真を開けなかったため、プレビューを表示しています。';

  @override
  String get retryOriginalImage => 'もう一度開く';

  @override
  String get viewerSourceChanged => 'この写真は変更されています。再読み込みすると最新の写真が表示されます。';

  @override
  String get viewerSourceMissing => 'この写真はライブラリから外されています。';

  @override
  String removingFromAme(String name) {
    return '「$name」を Ame から外しています…';
  }

  @override
  String get removingFolderDetail => '元のフォルダーと写真はそのまま残ります。';

  @override
  String get removeFolderFailed => 'フォルダーを外せませんでした';

  @override
  String refreshingAfterRemoval(String name) {
    return '「$name」を外しました。再読み込み中…';
  }

  @override
  String get refreshingAfterRemovalDetail => '元のフォルダーと写真はそのまま残っています。';

  @override
  String get removedFolderRefreshFailed => 'フォルダーを外しましたが、写真を読み込めませんでした';

  @override
  String get scanOverlapsStorage =>
      'このフォルダーは Ame のデータ保存先と重なっています。別のフォルダーを選んでください。';

  @override
  String get scanBusy => '別のフォルダーを更新中です。完了してから続けてください。';

  @override
  String get scanCancelFailed => 'キャンセルできませんでした。もう一度お試しください。';

  @override
  String get parentScanIncomplete =>
      '親フォルダーのスキャンが完了していません。フォルダーを開けるか、スキャンの上限に達していないか確認してください。追加済みのサブフォルダーは保持されています。';

  @override
  String get childScanPending =>
      'サブフォルダーのスキャンが完了していません。親フォルダーを追加する前に、スキャンを再開するかキャンセルしてください。';

  @override
  String get childUpdateRequired => '親フォルダーを追加する前に、追加済みのサブフォルダーを更新してください。';

  @override
  String get childMergeFailed =>
      '追加済みのサブフォルダーを統合できませんでした。保存場所を確認してもう一度お試しください。';

  @override
  String get scanFolderFailed => 'このフォルダーを更新できませんでした。フォルダーを開けるか確認してください。';

  @override
  String get synchronizationRefreshFailureMessage => '再読み込みすると最新の写真が表示されます。';

  @override
  String get manualUpdateNotice => '手動で更新する必要があるフォルダーがあります';

  @override
  String get manualUpdateDetail =>
      'Ame を開き直し、フォルダーのメニューで「ライブラリを更新」を選ぶと、アプリを閉じていた間の変更が反映されます。';

  @override
  String synchronizationBlockedFolder(String name) {
    return '「$name」を更新できません';
  }

  @override
  String synchronizationUncheckedFolder(String name) {
    return '「$name」はまだ確認されていません';
  }

  @override
  String firstImportRequiredFolder(String name) {
    return '「$name」の追加が完了していません';
  }

  @override
  String get synchronizationSourceUnavailable => 'このフォルダーを開けませんでした。';

  @override
  String get synchronizationSourceUnhealthy => '自動更新が中断されました。';

  @override
  String get synchronizationSourceStarting => '自動更新を準備しています。';

  @override
  String get synchronizationEvidenceGap => 'ファイルの変更の一部がまだライブラリに反映されていません。';

  @override
  String get synchronizationCapacityExceeded =>
      '多くのファイルが変更されました。ライブラリの更新はまだ完了していません。';

  @override
  String get synchronizationNeedsReconciliation => 'ライブラリの更新はまだ完了していません。';

  @override
  String get synchronizationCompleted => 'フォルダーの変更がライブラリに反映されました。';

  @override
  String get synchronizationAutomaticRetry => '保留中の更新があります。Ame が自動で再試行します。';

  @override
  String get synchronizationRecoveryBlocked =>
      'フォルダーのメニューで「ライブラリを更新」を選んで、もう一度お試しください。';

  @override
  String get synchronizationLegacyRecoveryAuthorityMissing =>
      'フォルダーのメニューで「ライブラリを更新」を選ぶと、自動更新を再開できます。';

  @override
  String get synchronizationMonitoringAccessDenied =>
      'このフォルダーの変更を読み取れません。アクセス権を確認してください。';

  @override
  String get synchronizationMonitoringEventIncomplete =>
      '一部のファイルの変更を読み取れませんでした。';

  @override
  String get synchronizationRecoveryFailed => 'フォルダーの確認が完了しませんでした。';

  @override
  String get synchronizationPersistenceFailed => 'ライブラリの更新内容を保存できませんでした。';

  @override
  String get synchronizationPreparing => '更新の準備中';

  @override
  String get synchronizationCheckingFolder => 'フォルダーを確認中';

  @override
  String get synchronizationCheckingChanges => '変更を確認中';

  @override
  String get synchronizationUpdatingLibrary => 'ライブラリを更新中';

  @override
  String get synchronizationRetryLater => '自動再試行待ち';

  @override
  String get synchronizationScanningFolder => 'フォルダーをスキャン中';

  @override
  String get synchronizationUpdateIncomplete => '更新が完了していません';

  @override
  String get synchronizationComplete => '同期が完了しました';

  @override
  String get synchronizationWaitingFolder => 'フォルダーの接続待ち';

  @override
  String get libraryUpdateBlocked => 'ライブラリを更新できません';

  @override
  String pendingChanges(String count) {
    return '反映待ちの変更：$count 件';
  }

  @override
  String retryingChanges(String count) {
    return '再試行待ちの変更：$count 件';
  }

  @override
  String incompleteChanges(String count) {
    return '未完了の更新：$count 件';
  }

  @override
  String get changesToCheck => '確認が必要なファイルの変更があります';

  @override
  String get waitingForUpdate => '更新待ち';

  @override
  String get checkingPhotos => '写真を確認中';

  @override
  String get loadingPhotosShort => '写真を読み込み中';

  @override
  String get cancelled => 'キャンセルしました';

  @override
  String get filesChanged => 'ファイルに変更があります';

  @override
  String get updateFailed => '更新できませんでした';

  @override
  String get scanCompleted => 'スキャンが完了しました。';

  @override
  String reloadRoot(String name) {
    return '「$name」を再読み込み';
  }

  @override
  String updateRoot(String name) {
    return '「$name」のライブラリを更新';
  }

  @override
  String retryRoot(String name) {
    return '「$name」の更新を再試行';
  }

  @override
  String progressPhotos(String phase, int checked, int total, int percent) {
    final intl.NumberFormat checkedNumberFormat =
        intl.NumberFormat.decimalPattern(localeName);
    final String checkedString = checkedNumberFormat.format(checked);
    final intl.NumberFormat totalNumberFormat =
        intl.NumberFormat.decimalPattern(localeName);
    final String totalString = totalNumberFormat.format(total);

    return '$phase、写真 $totalString 枚中 $checkedString 枚（$percent%）';
  }

  @override
  String progressFiles(String phase, int files, int photos) {
    final intl.NumberFormat filesNumberFormat =
        intl.NumberFormat.decimalPattern(localeName);
    final String filesString = filesNumberFormat.format(files);
    final intl.NumberFormat photosNumberFormat =
        intl.NumberFormat.decimalPattern(localeName);
    final String photosString = photosNumberFormat.format(photos);

    return '$phase、確認済みのファイル $filesString 個、見つかった写真 $photosString 枚';
  }

  @override
  String get duplicates => '重複写真';

  @override
  String get collapseSidebar => 'サイドバーを折りたたむ';

  @override
  String get expandSidebar => 'サイドバーを展開';

  @override
  String get saveSidebarFailed => 'サイドバーの設定を保存できませんでした';

  @override
  String get changePhotoFailed => '写真を切り替えられませんでした';

  @override
  String removedFolders(int count) {
    final intl.NumberFormat countNumberFormat =
        intl.NumberFormat.decimalPattern(localeName);
    final String countString = countNumberFormat.format(count);

    return 'Ame から $countString 個のフォルダーを外しました';
  }

  @override
  String get saveGallerySettingsFailed => 'ギャラリーの設定を保存できませんでした';

  @override
  String get filePathCopied => 'ファイルのパスをコピーしました';

  @override
  String get copyPathFailed => 'パスをコピーできませんでした';

  @override
  String get confirmRemoveFolder => 'このフォルダーを Ame から外しますか？';

  @override
  String confirmRemoveFolderDetail(String name) {
    return '「$name」にある写真は Ame に表示されなくなります。元のフォルダーと写真はそのまま残ります。';
  }

  @override
  String removedFolder(String name) {
    return '「$name」を Ame から外しました';
  }

  @override
  String get seekDateFailed => '指定した日付に移動できませんでした';

  @override
  String get scrollUp => '上にスクロール';

  @override
  String scrollPosition(int percent) {
    return 'スクロール位置：$percent%';
  }

  @override
  String get scrollDown => '下にスクロール';

  @override
  String get createAlbum => '新しいアルバム';

  @override
  String get renameAlbum => 'アルバム名を変更';

  @override
  String changeAlbumIcon(String name) {
    return '「$name」のアイコンを変更';
  }

  @override
  String get confirmDeleteAlbum => 'アルバムを削除しますか？';

  @override
  String get save => '保存';

  @override
  String get saving => '保存中';

  @override
  String get deleteAlbumImpact =>
      'このアルバムの写真グループは「除外済み」に移動します。写真ファイルとお気に入りはそのまま残ります。';

  @override
  String get albumName => 'アルバム名';

  @override
  String get chooseAlbumIcon => 'アルバムのアイコンを選択';

  @override
  String get albumIconTravel => '旅行';

  @override
  String get albumIconNature => '自然';

  @override
  String get albumIconPets => 'ペット';

  @override
  String get albumIconPeople => '人物';

  @override
  String get albumIconFood => '食べ物';

  @override
  String get albumIconArt => 'アート';

  @override
  String get albumIconWork => '仕事';

  @override
  String get albumIconStudy => '勉強';

  @override
  String get albumIconGames => 'ゲーム';

  @override
  String get albumIconMusic => '音楽';

  @override
  String get albumIconMemories => '思い出';

  @override
  String confirmDeleteAlbums(int count) {
    final intl.NumberFormat countNumberFormat =
        intl.NumberFormat.decimalPattern(localeName);
    final String countString = countNumberFormat.format(count);

    return '$countString 個のアルバムを削除しますか？';
  }

  @override
  String get deleteAlbumsImpact =>
      'これらのアルバムの写真グループは「除外済み」に移動します。写真ファイルとお気に入りはそのまま残ります。';

  @override
  String get deleteAlbum => 'アルバムを削除';

  @override
  String get selectPageAlbums => 'このページのアルバムをすべて選択';

  @override
  String get selectAllAlbums => 'すべてのアルバムを選択';

  @override
  String get deleteSelectedAlbums => '選択したアルバムを削除';

  @override
  String get albumsEmpty => 'アルバムはまだありません';

  @override
  String get previousPage => '前のページ';

  @override
  String get nextPage => '次のページ';

  @override
  String get albumsUnavailable => 'アルバムを読み込めませんでした';

  @override
  String get changeIcon => 'アイコンを変更';

  @override
  String get chooseAlbum => 'アルバムを選択';

  @override
  String get updatingAlbums => 'アルバムを更新中';

  @override
  String get createFirstAlbum => '先にアルバムを作成してください。';

  @override
  String get loadingAlbums => 'アルバムを読み込み中';

  @override
  String get albumsLoadFailed => 'アルバムを読み込めませんでした。';

  @override
  String get removeFromAlbum => 'アルバムから外す';

  @override
  String get albumGroupConflict => 'このグループのアルバム指定が競合しています。アルバムを選んでください。';

  @override
  String get albumDetailsUnavailable => 'アルバムの詳細を読み込めませんでした。選択すると再試行します。';

  @override
  String get albumsSidebarHint => 'サイドバーで「お気に入り」かアルバムを選ぶか、アルバムを新しく作成してください。';

  @override
  String get file => 'ファイル';

  @override
  String get path => 'パス';

  @override
  String get fileType => '種類';

  @override
  String get imageDimensions => '画像の寸法';

  @override
  String get size => 'サイズ';

  @override
  String get cameraMake => 'カメラのメーカー';

  @override
  String get cameraModel => 'カメラの機種';

  @override
  String get timeTaken => '撮影日時';

  @override
  String get timeSource => '日時の取得元';

  @override
  String get timeCreated => '作成日時';

  @override
  String get timeModified => '更新日時';

  @override
  String get cameraOriginalTime => 'カメラの撮影日時';

  @override
  String get digitizedTime => 'デジタル化した日時';

  @override
  String get photoRecordedTime => '画像に記録された日時';

  @override
  String get loadingAlbumAssignments => 'アルバムの割り当てを読み込み中';

  @override
  String get conflictAlbumsUnavailable => '競合しているアルバムを読み込めませんでした。';

  @override
  String get albumAssignmentsUpdated => 'アルバムの割り当てを更新しました。';

  @override
  String get conflictAlbums => '競合しているアルバム';

  @override
  String deletedAlbumName(String name) {
    return '$name（削除済み）';
  }

  @override
  String get similarKind => '類似';

  @override
  String get duplicateKind => '重複';

  @override
  String deletePhotoKind(String kind) {
    return '写真を削除（$kind）';
  }

  @override
  String get saveSelectionFailed => '選択を保存できませんでした';

  @override
  String get tryAgain => 'もう一度お試しください。';

  @override
  String similarPhotoCount(int count) {
    final intl.NumberFormat countNumberFormat =
        intl.NumberFormat.decimalPattern(localeName);
    final String countString = countNumberFormat.format(count);

    return '類似写真 $countString 枚';
  }

  @override
  String get updatingFileLocations => 'ファイルの保存場所を更新中';

  @override
  String get deleteSimilarPhotos => '類似写真を削除';

  @override
  String get deleteDuplicatePhotos => '重複写真を削除';

  @override
  String get deleteSelected => '選択した項目を削除';

  @override
  String get loadingFileLocations => 'ファイルの保存場所を読み込み中';

  @override
  String get checkingPhotoContent => '写真の内容を確認中';

  @override
  String get favoritesUnavailable => 'お気に入りの状態を読み込めませんでした';

  @override
  String get savingChanges => '変更を保存中';

  @override
  String get loadingFavoriteStatus => 'お気に入りの状態を読み込み中';

  @override
  String get removeFavorite => 'お気に入りから外す';

  @override
  String get addFavorite => 'お気に入りに追加';

  @override
  String get readingPhotos => '写真を読み込み中';

  @override
  String photoOperation(String action) {
    return '写真の$action';
  }

  @override
  String fileCount(int count) {
    final intl.NumberFormat countNumberFormat =
        intl.NumberFormat.decimalPattern(localeName);
    final String countString = countNumberFormat.format(count);

    return '$countString 個のファイル';
  }

  @override
  String get destinationFolder => '保存先フォルダー';

  @override
  String get change => '変更';

  @override
  String get folderStructure => 'フォルダー構成';

  @override
  String get fileNameLabel => 'ファイル名';

  @override
  String get permanentDeleteWarning => 'これらのファイルを完全に削除します。ごみ箱からは復元できません。';

  @override
  String get recycleWarning => 'これらのファイルを Windows のごみ箱に移動します。';

  @override
  String get deletePermanently => '完全に削除';

  @override
  String get back => '戻る';

  @override
  String get operationReport => '操作レポート';

  @override
  String get affectedFiles => '対象ファイル';

  @override
  String selectedCountShort(int count) {
    final intl.NumberFormat countNumberFormat =
        intl.NumberFormat.decimalPattern(localeName);
    final String countString = countNumberFormat.format(count);

    return '$countString 件選択中';
  }

  @override
  String get reportFile => 'レポートファイル';

  @override
  String get reportHistoryUnavailable => 'レポート履歴を読み込めませんでした。もう一度お試しください。';

  @override
  String get loadingReport => 'レポートを読み込み中';

  @override
  String get noExportHistory => 'エクスポート履歴はまだありません';

  @override
  String get reportPending => 'レポートはまだ作成されていません';

  @override
  String get noReportNeeded => '報告する問題はありません';

  @override
  String get reportWriteFailedStatus => 'レポートを書き込めませんでした。ファイル操作の結果は保持されています。';

  @override
  String get createReport => 'レポートを作成';

  @override
  String get regenerate => '再作成';

  @override
  String get updatingReport => 'レポートを更新中';

  @override
  String get latestRecords => '最新の記録';

  @override
  String get earlierRecords => '過去の記録';

  @override
  String get stop => '停止';

  @override
  String get viewHistory => '履歴を表示';

  @override
  String get noExceptionsReport => '問題はありませんでした。レポートの作成は不要です。';

  @override
  String get loadingOperationHistory => '操作履歴を読み込み中';

  @override
  String verifiedFiles(int checked, int total) {
    final intl.NumberFormat checkedNumberFormat =
        intl.NumberFormat.decimalPattern(localeName);
    final String checkedString = checkedNumberFormat.format(checked);
    final intl.NumberFormat totalNumberFormat =
        intl.NumberFormat.decimalPattern(localeName);
    final String totalString = totalNumberFormat.format(total);

    return '確認済みのファイル：$totalString 個中 $checkedString 個';
  }

  @override
  String get reportSavedDestination => 'レポートを保存先フォルダーに保存しました。';

  @override
  String processedFiles(int processed, int total) {
    final intl.NumberFormat processedNumberFormat =
        intl.NumberFormat.decimalPattern(localeName);
    final String processedString = processedNumberFormat.format(processed);
    final intl.NumberFormat totalNumberFormat =
        intl.NumberFormat.decimalPattern(localeName);
    final String totalString = totalNumberFormat.format(total);

    return '処理済みのファイル：$totalString 個中 $processedString 個';
  }

  @override
  String get fileHistoryUnavailable => 'ファイル操作履歴を読み込めませんでした。もう一度お試しください。';

  @override
  String get confirmContinueMove => 'このファイルの移動を続けますか？';

  @override
  String get continueMove => '移動を続ける';

  @override
  String get fileOperationHistory => 'ファイル操作履歴';

  @override
  String get updatingFileRecords => 'ファイルの記録を更新中';

  @override
  String get fileRecords => 'ファイルの記録';

  @override
  String get loadingFileRecords => 'ファイルの記録を読み込み中';

  @override
  String get noFileOperations => 'ファイル操作の履歴はまだありません';

  @override
  String get fileOperationDetails => 'ファイル操作の詳細';

  @override
  String get checkResults => '結果を確認';

  @override
  String get recycleBinOpenFailed => 'ごみ箱を開けませんでした。Windows のデスクトップから開いてください。';

  @override
  String get openRecycleBin => 'ごみ箱を開く';

  @override
  String get duplicatesOnly => '重複写真のみ';

  @override
  String get similarOnly => '類似写真のみ';

  @override
  String get mergeDuplicates => '重複写真をまとめる';

  @override
  String removeFilter(String label) {
    return '「$label」を外す';
  }

  @override
  String selectedItems(int count) {
    final intl.NumberFormat countNumberFormat =
        intl.NumberFormat.decimalPattern(localeName);
    final String countString = countNumberFormat.format(count);

    return '$countString 件の項目を選択中';
  }

  @override
  String duplicateGroupCount(int count) {
    final intl.NumberFormat countNumberFormat =
        intl.NumberFormat.decimalPattern(localeName);
    final String countString = countNumberFormat.format(count);

    return '重複グループ $countString 件';
  }

  @override
  String similarGroupCount(int count) {
    final intl.NumberFormat countNumberFormat =
        intl.NumberFormat.decimalPattern(localeName);
    final String countString = countNumberFormat.format(count);

    return '類似グループ $countString 件';
  }

  @override
  String deduplicatedCount(String summary, int count) {
    final intl.NumberFormat countNumberFormat =
        intl.NumberFormat.decimalPattern(localeName);
    final String countString = countNumberFormat.format(count);

    return '$summary・重複を除くと $countString 枚';
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

    return '写真 $totalString 枚・整理済み $organizedString 枚・未整理 $pendingString 枚・除外済み $ignoredString 枚';
  }

  @override
  String get organizationMode => '整理モード';

  @override
  String get exitOrganization => '整理モードを終了';

  @override
  String get noPendingPhotos => '未整理の写真はありません';

  @override
  String get noDeferredPhotos => '「あとで整理」に写真はありません';

  @override
  String get noIgnoredPhotos => '除外した写真はありません';

  @override
  String get organizationEmptyHint => '「その他」メニューで整理対象を切り替えるか、整理モードを終了してください。';

  @override
  String get noCategoryPhotos => 'このカテゴリにはまだ写真がありません';

  @override
  String get categoryEmptyHint => '別のカテゴリを選ぶか、ライブラリに戻ってください。';

  @override
  String get noAlbumPhotos => 'このアルバムにはまだ写真がありません';

  @override
  String get albumEmptyHint => 'ライブラリで写真を開いて、アルバムに追加してください。';

  @override
  String get noFavoritePhotos => 'お気に入りの写真はまだありません';

  @override
  String get favoritesEmptyHint => '写真を開いて、お気に入りに追加してください。';

  @override
  String get filmstripSettingFailed => 'サムネイルの設定を保存できませんでした。もう一度お試しください。';

  @override
  String photoPosition(int position, int total) {
    final intl.NumberFormat totalNumberFormat =
        intl.NumberFormat.decimalPattern(localeName);
    final String totalString = totalNumberFormat.format(total);

    return '$totalString 枚中 $position 枚目';
  }

  @override
  String clearFilter(String label) {
    return '「$label」を解除';
  }

  @override
  String get hide => '非表示';

  @override
  String get downloadSmartModel => 'スマートアルバムのモデルをダウンロード';

  @override
  String get smartModelDownloadDetail =>
      'このデバイスで写真を解析するため、約 504 MB をダウンロードします。保存済みの完全なモデルファイルは再利用されます。';

  @override
  String get downloadAndEnable => 'ダウンロードして有効にする';

  @override
  String get folders => 'フォルダー';

  @override
  String get removeSelectedFolders => '選択したフォルダーを Ame から外す';

  @override
  String get resizeSidebar => 'サイドバーの幅を変更';

  @override
  String pixelCount(int count) {
    final intl.NumberFormat countNumberFormat =
        intl.NumberFormat.decimalPattern(localeName);
    final String countString = countNumberFormat.format(count);

    return '$countString ピクセル';
  }

  @override
  String get undoAction => '操作を元に戻す';

  @override
  String get redoAction => '操作をやり直す';

  @override
  String get historyLoadRetry => '操作履歴を読み込めませんでした。選択すると再試行します。';

  @override
  String get organizationOptions => '整理オプション';

  @override
  String get albumsAndFavoritesUnavailable => 'アルバムとお気に入りを読み込めませんでした';

  @override
  String get reopenMenuRetry => 'メニューを開き直してもう一度お試しください。';

  @override
  String get copyToFolder => 'フォルダーにコピー';

  @override
  String get moveToFolder => 'フォルダーに移動';

  @override
  String get photoListFailed => '写真の一覧を読み込めませんでした';

  @override
  String get continueOrganizing => '整理を続ける';

  @override
  String get toOrganize => '未整理';

  @override
  String photosRelatedTo(String name) {
    return '「$name」に関連する写真';
  }

  @override
  String get selectedAlbumConflict =>
      '選択した写真の一部でアルバム指定が競合しています。アルバムを 1 つ選んでください。';

  @override
  String get actionFailed => '操作を完了できませんでした';

  @override
  String get favorite => 'お気に入り';

  @override
  String get preparing => '準備中';

  @override
  String get processing => '処理中';

  @override
  String itemsCompleted(int completed, int total) {
    final intl.NumberFormat totalNumberFormat =
        intl.NumberFormat.decimalPattern(localeName);
    final String totalString = totalNumberFormat.format(total);

    return '完了した項目：$totalString 件中 $completed 件。';
  }

  @override
  String get sortOrderSaveFailed => '新しい並び順を保存できませんでした。項目をもう一度ドラッグしてください。';

  @override
  String confirmRemoveFolders(int count) {
    final intl.NumberFormat countNumberFormat =
        intl.NumberFormat.decimalPattern(localeName);
    final String countString = countNumberFormat.format(count);

    return '$countString 個のフォルダーを Ame から外しますか？';
  }

  @override
  String get removeFoldersImpact =>
      'これらのフォルダー内の写真は Ame に表示されなくなります。これらのフォルダーだけにある写真のアルバム割り当て、お気に入り、整理記録は削除されます。他の読み込み元にもある写真の記録は保持されます。元のフォルダーとファイルはそのまま残ります。';

  @override
  String selectedCount(int count) {
    final intl.NumberFormat countNumberFormat =
        intl.NumberFormat.decimalPattern(localeName);
    final String countString = countNumberFormat.format(count);

    return '$countString 件選択中';
  }

  @override
  String get selectedLong => '選択済み';

  @override
  String get selectedShort => '選択済み';

  @override
  String get deselectAll => 'すべての選択を解除';

  @override
  String selectAllNamed(String label) {
    return '$labelをすべて選択';
  }

  @override
  String collapseSection(String section) {
    return '$sectionを折りたたむ';
  }

  @override
  String expandSection(String section) {
    return '$sectionを展開';
  }

  @override
  String deselectNamed(String name) {
    return '「$name」の選択を解除';
  }

  @override
  String selectNamed(String name) {
    return '「$name」を選択';
  }

  @override
  String get noClassificationResults => '分類結果はまだありません';

  @override
  String get downloadModel => 'モデルをダウンロード';

  @override
  String get loadingSmartAlbums => 'スマートアルバムを読み込み中';

  @override
  String get categoriesUnavailable => 'カテゴリを読み込めませんでした';

  @override
  String recentlyAddedCount(int count) {
    final intl.NumberFormat countNumberFormat =
        intl.NumberFormat.decimalPattern(localeName);
    final String countString = countNumberFormat.format(count);

    return '最近追加・$countString 枚';
  }

  @override
  String previouslyAddedCount(int count) {
    final intl.NumberFormat countNumberFormat =
        intl.NumberFormat.decimalPattern(localeName);
    final String countString = countNumberFormat.format(count);

    return '以前に追加・$countString 枚';
  }

  @override
  String get recentlyAdded => '最近追加';

  @override
  String get previouslyAdded => '以前に追加';

  @override
  String get pause => '一時停止';

  @override
  String get resume => '再開';

  @override
  String get libraryUpdates => 'ライブラリの更新';

  @override
  String get clearFinishedTasks => '終了したタスクを消去';

  @override
  String cancelRootUpdate(String name) {
    return '「$name」の更新をキャンセル';
  }

  @override
  String closeRootUpdate(String name) {
    return '「$name」の更新通知を閉じる';
  }

  @override
  String rootUpdateProgress(String name) {
    return '「$name」の更新状況';
  }

  @override
  String categoryLabel(String name) {
    return 'カテゴリ：$name';
  }

  @override
  String get previousPhotoShortcut => '前の写真（←）';

  @override
  String get nextPhotoShortcut => '次の写真（→）';

  @override
  String get photoCopied => '写真をクリップボードにコピーしました';

  @override
  String get photoTooLargeToCopy => '写真が大きすぎるため、クリップボードにコピーできません';

  @override
  String get copyPhotoFailed => '写真をコピーできませんでした。もう一度お試しください。';

  @override
  String get reloadThumbnails => 'サムネイルを再読み込み';

  @override
  String get toggleFullscreenFailed => '全画面表示を切り替えられませんでした。もう一度お試しください。';

  @override
  String get readingPhotoStatus => '写真の状態を読み込み中';

  @override
  String get rereadPhotoStatus => '写真の状態を再読み込み';

  @override
  String get copyPhoto => '写真をコピー';

  @override
  String get rotateClockwise => '右に 90° 回転';

  @override
  String get zoomOutShortcut => '縮小（- / Ctrl+-）';

  @override
  String get zoomInShortcut => '拡大（+ / Ctrl++）';

  @override
  String get archiveFileType => 'Ame の整理記録';

  @override
  String get exportAction => 'エクスポート';

  @override
  String get chooseBackup => 'バックアップを選択';

  @override
  String get inspect => '確認';

  @override
  String get restore => '復元';

  @override
  String get preparingBackup => 'バックアップを準備中';

  @override
  String get exportingArchive => '整理記録をエクスポート中';

  @override
  String get checkingBackup => 'バックアップを確認中';

  @override
  String get linkingPhotos => 'ライブラリの写真と照合中';

  @override
  String get restoringArchive => '整理記録を復元中';

  @override
  String archiveInProgress(String operation) {
    return '整理記録の処理中：$operation';
  }

  @override
  String get archiveOutsideLibrary => 'ライブラリと Ame のデータフォルダーの外にある場所を選んでください。';

  @override
  String get archiveFileExists => '同じ名前のファイルがあります。別の名前を入力してください。';

  @override
  String get archiveVersionUnsupported =>
      'このバージョンのバックアップには対応していません。対応するバックアップを選んでください。';

  @override
  String get archiveInvalid => 'このバックアップは不完全か、形式が無効です。別のバックアップを選んでください。';

  @override
  String get archiveLibraryChanged =>
      'ライブラリまたは整理記録が変更されました。復元する前にバックアップをもう一度確認してください。';

  @override
  String get archiveChanged => '選択したバックアップが変更されました。復元する前にもう一度確認してください。';

  @override
  String get archiveAlreadyRunning => '別のバックアップ操作を実行中です。しばらくしてからもう一度お試しください。';

  @override
  String get archiveUnreadable => 'バックアップを読み込めませんでした。このデバイスでファイルを開けるか確認してください。';

  @override
  String get archiveIoFailed =>
      'バックアップにアクセスできませんでした。選択したファイルと保存先を確認してもう一度お試しください。';

  @override
  String get archiveConflictsWithCurrent =>
      'バックアップの内容が現在の記録と競合しています。現在の記録を保持しました。別のバックアップを選んでください。';

  @override
  String get archiveResultUnknown => '結果を確認できませんでした。もう一度お試しください。';

  @override
  String get settingsSaveRetry => '設定を保存できませんでした。もう一度お試しください。';

  @override
  String get unreadable => '読み取れませんでした';

  @override
  String get askBeforeAdding => 'アルバムに追加する前に確認';

  @override
  String get askBeforeAddingHelp => '追加先のアルバムを毎回選びます';

  @override
  String get defaultAlbum => '既定のアルバム';

  @override
  String get defaultAlbumHelp => '設定していない場合は、写真を追加するときにアルバムを選びます。';

  @override
  String get gpuSettingFailed => 'GPU アクセラレーションの設定を変更できませんでした';

  @override
  String get gpuSettingRetry =>
      'もう一度お試しください。写真の解析に引き続き失敗する場合は、GPU アクセラレーションをオフにしてください。';

  @override
  String get gpuAcceleration => 'GPU アクセラレーション';

  @override
  String get loadingSettings => '設定を読み込み中';

  @override
  String get settingsReadFailed => '設定を読み込めませんでした。もう一度お試しください。';

  @override
  String get switchingAnalysisDevice => '解析に使うデバイスを切り替えています';

  @override
  String get smartAlbumsCpu => 'スマートアルバムは CPU で写真を解析しています';

  @override
  String smartAlbumsDevice(String name) {
    return 'スマートアルバムは $name で写真を解析しています';
  }

  @override
  String get smartAlbumsPreferGpu => 'スマートアルバムは利用可能な GPU で写真を解析します';

  @override
  String get restoreOrganizationRecords => '整理記録を復元';

  @override
  String createdAt(String timestamp) {
    return '作成日時：$timestamp';
  }

  @override
  String albumCount(int count) {
    final intl.NumberFormat countNumberFormat =
        intl.NumberFormat.decimalPattern(localeName);
    final String countString = countNumberFormat.format(count);

    return 'アルバム $countString 個';
  }

  @override
  String photoGroupCount(int count) {
    final intl.NumberFormat countNumberFormat =
        intl.NumberFormat.decimalPattern(localeName);
    final String countString = countNumberFormat.format(count);

    return '写真グループ $countString 件';
  }

  @override
  String favoriteGroupCount(int count) {
    final intl.NumberFormat countNumberFormat =
        intl.NumberFormat.decimalPattern(localeName);
    final String countString = countNumberFormat.format(count);

    return 'お気に入りのグループ $countString 件';
  }

  @override
  String duplicateChoiceCount(int count) {
    final intl.NumberFormat countNumberFormat =
        intl.NumberFormat.decimalPattern(localeName);
    final String countString = countNumberFormat.format(count);

    return '重複写真の選択記録 $countString 件';
  }

  @override
  String matchedGroups(int count) {
    final intl.NumberFormat countNumberFormat =
        intl.NumberFormat.decimalPattern(localeName);
    final String countString = countNumberFormat.format(count);

    return '照合できるグループ $countString 件';
  }

  @override
  String unmatchedGroups(int count) {
    final intl.NumberFormat countNumberFormat =
        intl.NumberFormat.decimalPattern(localeName);
    final String countString = countNumberFormat.format(count);

    return '照合できないグループ $countString 件';
  }

  @override
  String albumConflictCount(int count) {
    final intl.NumberFormat countNumberFormat =
        intl.NumberFormat.decimalPattern(localeName);
    final String countString = countNumberFormat.format(count);

    return 'アルバムの競合 $countString 件';
  }

  @override
  String unresolvedCopies(int count) {
    final intl.NumberFormat countNumberFormat =
        intl.NumberFormat.decimalPattern(localeName);
    final String countString = countNumberFormat.format(count);

    return '照合できていない優先コピー $countString 件';
  }

  @override
  String get includesOrganizationPosition => '整理モードで中断した位置を含みます';

  @override
  String get restoreKeepsChoices => '現在の選択と整理位置は保持されます。元の写真の保存場所は変わりません。';

  @override
  String get unmatchedAwaitPhotos => '照合できない記録も、対応する写真が見つかるまで保持します。';

  @override
  String get filePickerFailed => 'ファイル選択画面を開けませんでした';

  @override
  String get organizationRecords => '整理記録';

  @override
  String get exportOrganizationRecords => '整理記録をエクスポート';

  @override
  String get exportArchiveHelp => 'アルバム、お気に入り、整理の進行状況をバックアップします。元の写真は含まれません。';

  @override
  String get restoreArchiveHelp => '現在の選択を保持したままバックアップから復元します';

  @override
  String get viewBackup => 'バックアップを表示';

  @override
  String get chooseFile => 'ファイルを選択';

  @override
  String get noUnlinkedRecords => '条件に合う未関連付けの記録はありません';

  @override
  String get unlinkedLessThanDay => '関連付けが外れてから 1 日未満';

  @override
  String unlinkedDays(int days) {
    return '関連付けが外れてから $days 日';
  }

  @override
  String unlinkedRange(String shortest, int longest) {
    return '関連付けが外れてから $shortest～$longest 日';
  }

  @override
  String get lessThanOne => '1 日未満';

  @override
  String get clearUnlinkedRecords => '未関連付けの記録を消去';

  @override
  String get checkingRecords => '記録を確認中';

  @override
  String get cleanupScope => '消去する記録';

  @override
  String get unlinkedNinetyDays => '関連付けが外れてから 90 日を超えた記録';

  @override
  String get unlinkedThirtyDays => '関連付けが外れてから 30 日を超えた記録';

  @override
  String get allUnlinkedRecords => 'すべての未関連付けの記録';

  @override
  String get checkFailedRetry => '確認を完了できませんでした。もう一度お試しください。';

  @override
  String get checkCancelled => '確認をキャンセルしました';

  @override
  String get clearRecords => '記録を消去';

  @override
  String cleanupCounts(int groups, int choices) {
    final intl.NumberFormat groupsNumberFormat =
        intl.NumberFormat.decimalPattern(localeName);
    final String groupsString = groupsNumberFormat.format(groups);
    final intl.NumberFormat choicesNumberFormat =
        intl.NumberFormat.decimalPattern(localeName);
    final String choicesString = choicesNumberFormat.format(choices);

    return '写真グループ：$groupsString 件・重複写真の選択記録：$choicesString 件';
  }

  @override
  String cleanupAlbumGroups(int count) {
    final intl.NumberFormat countNumberFormat =
        intl.NumberFormat.decimalPattern(localeName);
    final String countString = countNumberFormat.format(count);

    return 'アルバムに属するグループ：$countString 件';
  }

  @override
  String cleanupFavoriteGroups(int count) {
    final intl.NumberFormat countNumberFormat =
        intl.NumberFormat.decimalPattern(localeName);
    final String countString = countNumberFormat.format(count);

    return 'お気に入りのグループ：$countString 件';
  }

  @override
  String cleanupProgressGroups(int count) {
    final intl.NumberFormat countNumberFormat =
        intl.NumberFormat.decimalPattern(localeName);
    final String countString = countNumberFormat.format(count);

    return '整理の進行状況があるグループ：$countString 件';
  }

  @override
  String cleanupClassificationGroups(int count) {
    final intl.NumberFormat countNumberFormat =
        intl.NumberFormat.decimalPattern(localeName);
    final String countString = countNumberFormat.format(count);

    return '分類記録があるグループ：$countString 件';
  }

  @override
  String get cleanupRecordImpact =>
      'アルバムと元の写真は保持されます。写真を再度インポートしても、これらの記録は復元されません。';

  @override
  String get cleanupUndoWarning =>
      '消去した記録は元に戻せません。復元するには、事前にエクスポートしたバックアップが必要です。';

  @override
  String get cleanupRecordsHelp => '写真に関連付けられないアルバム、お気に入り、整理記録を確認します';

  @override
  String get cleanUp => 'クリーンアップ';

  @override
  String get stopNotConfirmed => 'タスクはまだ停止していません。もう一度お試しください。';

  @override
  String get clearingRecords => '記録を消去中';

  @override
  String get checkAgain => '再確認';

  @override
  String get recordsChanged => '記録に変更がありました。確定する前にもう一度確認してください。';

  @override
  String get cleanupResultUnknown => '消去の結果を確認できませんでした。もう一度お試しください。';

  @override
  String get confirmClearThumbnails => 'サムネイルを消去しますか？';

  @override
  String get confirmClearOldThumbnails => '以前のサムネイルフォルダーをクリーンアップしますか？';

  @override
  String get oldThumbnailFolder => '以前のサムネイルフォルダー';

  @override
  String get clearThumbnailsImpact =>
      'キャッシュしたサムネイルを消去します。元の写真はそのまま残ります。次にライブラリを開くと、サムネイルの再作成に時間がかかることがあります。';

  @override
  String get clearOldThumbnailsImpact =>
      'このフォルダーにある Ame が管理するサムネイルだけを消去します。元の写真とその他のファイルはそのまま残ります。';

  @override
  String get startCleanup => 'クリーンアップを開始';

  @override
  String get cleanupAlreadyFinished => 'クリーンアップはすでに終了しています。';

  @override
  String get storage => 'ストレージ';

  @override
  String get loadingStorageSettings => 'ストレージ設定を読み込み中';

  @override
  String get checkingStoragePaths => 'ライブラリデータとサムネイルの保存場所を確認中';

  @override
  String get storageSettingsUnavailable => 'ストレージ設定を読み込めませんでした';

  @override
  String get storageRestartRequired => '新しいストレージ設定を適用するには Ame を再起動してください';

  @override
  String get existingFilesKept => '既存のファイルは現在の場所に残ります';

  @override
  String get catalogStorageLocation => 'ライブラリデータの保存場所';

  @override
  String catalogStorageHelp(String path) {
    return 'ライブラリの索引とスキャン結果を保存します\n$path';
  }

  @override
  String storageUsageWithDetail(String size, String detail) {
    return '使用容量：$size$detail';
  }

  @override
  String storageUsageDetail(String live, String reclaimable) {
    return '（使用中：約 $live、解放可能：$reclaimable）';
  }

  @override
  String get catalogLocationLocked => 'ライブラリがある間は変更できません';

  @override
  String get starting => '開始しています';

  @override
  String get cleanupNotNeeded => 'クリーンアップは不要です';

  @override
  String get thumbnailLocation => 'サムネイルの保存場所';

  @override
  String thumbnailStorageHelp(String path) {
    return '必要に応じて再作成できるプレビューを保存します\n$path';
  }

  @override
  String get retiredThumbnailDirectory => '以前のサムネイルフォルダー';

  @override
  String retiredThumbnailsHelp(String path) {
    return '新しいフォルダーを使用中です。クリーンアップを実行すると、以前のフォルダーから Ame のサムネイルを消去します。\n$path';
  }

  @override
  String get cleaning => 'クリーンアップ中';

  @override
  String get cleanOldFolder => '以前のフォルダーをクリーンアップ';

  @override
  String get thumbnailBudget => 'サムネイルの容量上限';

  @override
  String get thumbnailBudgetHelp =>
      '上限に達すると、使用していないサムネイルを自動で消去します。元の写真はそのまま残ります。';

  @override
  String storageUsage(String size) {
    return '使用容量：$size';
  }

  @override
  String get storageSaveFailed => 'ストレージ設定を保存できませんでした';

  @override
  String get storageOverlapsSource =>
      '保存先が追加済みの写真フォルダーと重なっています。保存用に別のフォルダーを選んでください。';

  @override
  String get storageErrorDetails => 'エラーの詳細を表示';

  @override
  String get storageHideErrorDetails => 'エラーの詳細を閉じる';

  @override
  String get catalogMaintenanceStatusError => 'ライブラリのメンテナンス状況を読み込めませんでした';

  @override
  String get thumbnailCleanupStatusError => 'サムネイルの消去状況を読み込めませんでした';

  @override
  String get storageLocationPending => '完了するまでは現在の保存場所を使用します';

  @override
  String get optimizeCatalog => 'ライブラリのストレージを最適化';

  @override
  String get catalogWaiting => 'ライブラリのストレージは最適化待ちです';

  @override
  String get catalogInspecting => 'ライブラリデータを確認中';

  @override
  String get catalogWaitingIdle => '他の処理が終わり次第、ライブラリのストレージを最適化します';

  @override
  String get catalogCheckingSpace => '最適化に必要な空き領域を確認中';

  @override
  String get catalogOptimizing => 'ライブラリのデータベースを最適化中';

  @override
  String get catalogReclaiming => 'ライブラリの空き領域を回収中';

  @override
  String get catalogOptimized => 'ライブラリのストレージを最適化しました';

  @override
  String get catalogOptimizationCancelled => 'ライブラリの最適化をキャンセルしました';

  @override
  String get catalogOptimizationFailed => 'ライブラリを最適化できませんでした';

  @override
  String reclaimableSpace(String size) {
    return '約 $size を解放できます';
  }

  @override
  String get catalogAutoReclaim =>
      'ライブラリを外すと、Ame がバックグラウンドでデータベースの未使用領域を解放します。元の写真はそのまま残ります。';

  @override
  String get catalogRemovalApplied => 'ライブラリを外しました。まもなくバックグラウンドで最適化を開始します。';

  @override
  String get catalogEstimating => '使用容量と解放できる領域を計算中';

  @override
  String get catalogBusyWait => 'ライブラリで別の操作を実行中です。完了すると最適化を自動で再開します。';

  @override
  String get catalogCheckingTempSpace => 'データベースの再構築に必要なディスクの空き領域を確認中';

  @override
  String catalogConverting(String size) {
    return '約 $size を解放するためデータベースを変換中です。この処理中は進捗率を表示できません。';
  }

  @override
  String catalogReleasedRemaining(String released, String remaining) {
    return '$released を解放しました。残りは約 $remaining です';
  }

  @override
  String catalogReleased(String released) {
    return '$released を解放しました。既存のライブラリデータは保持されています。';
  }

  @override
  String get catalogStopped =>
      '最適化を停止しました。ライブラリの削除は反映済みで、既存のデータは保持されています。あとでもう一度お試しください。';

  @override
  String get catalogRetryLater => 'ライブラリの最適化を完了できませんでした。しばらくしてからもう一度お試しください。';

  @override
  String spaceRequired(String requiredSize, String available) {
    return '必要な空き容量：$requiredSize、利用可能：$available';
  }

  @override
  String get thumbnails => 'サムネイル';

  @override
  String cleaningSubject(String subject) {
    return 'クリーンアップ中：$subject';
  }

  @override
  String cleanupSubjectComplete(String subject) {
    return 'クリーンアップ完了：$subject';
  }

  @override
  String cleanupSubjectCancelled(String subject) {
    return 'クリーンアップをキャンセルしました：$subject';
  }

  @override
  String cleanupSubjectFailed(String subject) {
    return 'クリーンアップに失敗しました：$subject';
  }

  @override
  String get clearThumbnails => 'サムネイルを消去';

  @override
  String get thumbnailCleanupHelp => '必要になったサムネイルは再作成されます。元の写真はそのまま残ります。';

  @override
  String get countingThumbnails => '消去できるサムネイルを集計中';

  @override
  String cleanupFileProgress(String processed, String total, String size) {
    return '処理済みのファイル：$total 個中 $processed 個、解放した容量：$size';
  }

  @override
  String cleanupFilesRemoved(String count, String size) {
    return '消去したファイル：$count 個、解放した容量：$size';
  }

  @override
  String cleanupBeforeStop(String count) {
    return '停止するまでに $count 個のファイルを消去しました。元の写真はそのまま残っています。';
  }

  @override
  String get thumbnailCleanupFailed => 'サムネイルの消去を完了できませんでした';

  @override
  String cleanupFailedFiles(String count) {
    return '消去できなかったファイル：$count 個';
  }

  @override
  String get chooseThisFolder => 'このフォルダーを選択';

  @override
  String get catalogCleanupAlreadyFinished => 'ライブラリの最適化はすでに終了しています。';

  @override
  String folderStatus(String name, String status) {
    return '「$name」：$status';
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
  String get clauseSeparator => '、';

  @override
  String get sentenceSeparator => '。';

  @override
  String listSentence(String items) {
    return '$items。';
  }

  @override
  String get categoryIllustration => 'イラスト';

  @override
  String get categoryComic => '漫画';

  @override
  String get categoryMeme => 'ミーム';

  @override
  String get categoryScreenshot => 'スクリーンショット';

  @override
  String get categoryPhoto => '写真';

  @override
  String get categoryOther => 'その他';

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

    return '$percent%・$receivedString / $totalString MB';
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
